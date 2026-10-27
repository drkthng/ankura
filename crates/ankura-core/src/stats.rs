//! Reading stats (ROADMAP M3): characters read per day and streaks, words seen and looked up, and
//! "stealth leeches": words Anki says are known that the user keeps looking up while reading.

use std::collections::HashMap;

use serde::Serialize;

use crate::knowledge::WordState;
use crate::store::{Store, now_ms};
use crate::{Result, WordKey};

const DAYS_SHOWN: i64 = 30;
const LEECH_WINDOW_DAYS: i64 = 60;
const LEECH_MIN_LOOKUPS: i64 = 2;
const LIST_LEN: i64 = 30;

#[derive(Debug, Clone, Serialize)]
pub struct DayCount {
    pub day: String,
    pub chars: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Friction {
    pub key: WordKey,
    pub display: String,
    /// Anki's state, or `unknown` when Anki has no card for it.
    pub state: WordState,
    /// Anki's strength in days, when it has a card.
    pub strength: Option<f64>,
    pub lookups: i64,
    /// Time of the last look-up, ms since epoch.
    pub last: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub known_words: i64,
    pub chars_total: i64,
    pub chars_today: i64,
    /// The last 30 days, oldest first, including days without reading.
    pub days: Vec<DayCount>,
    /// Days in a row with reading, up to today (or yesterday, if today has none yet).
    pub streak: i64,
    pub best_streak: i64,
    pub texts_read: i64,
    pub words_seen_30d: i64,
    pub words_looked_up_30d: i64,
    pub stealth_leeches: Vec<Friction>,
    pub most_looked_up: Vec<Friction>,
}

// --- dates (YYYY-MM-DD) without a date library: days since 1970-01-01, proleptic Gregorian ---

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn parse_day(s: &str) -> Option<i64> {
    let mut it = s.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next()??);
    Some(days_from_civil(y, m, d))
}

fn format_day(n: i64) -> String {
    let (y, m, d) = civil_from_days(n);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Current and best streak from the set of days with reading.
fn streaks(read: &[i64], today: i64) -> (i64, i64) {
    let mut days = read.to_vec();
    days.sort_unstable();
    days.dedup();
    let (mut best, mut run, mut prev) = (0, 0, i64::MIN);
    for &d in &days {
        run = if d == prev + 1 { run + 1 } else { 1 };
        best = best.max(run);
        prev = d;
    }
    let set: std::collections::HashSet<i64> = days.into_iter().collect();
    let mut day = if set.contains(&today) { today } else { today - 1 };
    let mut current = 0;
    while set.contains(&day) {
        current += 1;
        day -= 1;
    }
    (current, best)
}

fn friction_rows(store: &Store, sql: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<Friction>> {
    let mut st = store.conn.prepare(sql)?;
    let rows = st.query_map(args, |r| {
        let state: Option<String> = r.get(4)?;
        Ok(Friction {
            key: WordKey { lang: r.get(0)?, lemma: r.get(1)?, reading: r.get(2)? },
            display: r.get(3)?,
            state: state.as_deref().and_then(WordState::parse).unwrap_or(WordState::Unknown),
            strength: r.get(5)?,
            lookups: r.get(6)?,
            last: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Look-ups per word since `since`, with Anki's state; `extra` filters further.
fn lookups_sql(extra: &str) -> String {
    format!(
        "SELECT e.lang, e.lemma, e.reading, coalesce(max(e.display), k.display, e.lemma), k.state, k.strength,
                count(*) AS n, max(e.ts)
         FROM events e LEFT JOIN knowledge k ON k.lang = e.lang AND k.lemma = e.lemma AND k.reading = e.reading
         WHERE e.kind = 'lookup' AND e.lang = ?1 AND e.ts >= ?2
           AND NOT EXISTS (SELECT 1 FROM word_overrides o WHERE o.lang = e.lang AND o.lemma = e.lemma
                           AND o.reading = e.reading AND o.state = 'ignored')
         GROUP BY e.lang, e.lemma, e.reading
         {extra}
         ORDER BY n DESC, max(e.ts) DESC LIMIT {LIST_LEN}"
    )
}

/// Stats for one language. `today` is the user's local date (YYYY-MM-DD).
pub fn stats(store: &Store, lang: &str, today: &str) -> Result<Stats> {
    let today_n = parse_day(today).ok_or_else(|| crate::Error::Llm(format!("bad date {today}")))?;
    let c = &store.conn;
    let per_day: HashMap<i64, i64> = {
        let mut st = c.prepare("SELECT day, sum(chars) FROM reading_days WHERE lang = ?1 GROUP BY day")?;
        let rows = st.query_map([lang], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        rows.filter_map(|r| r.ok()).filter_map(|(d, n)| Some((parse_day(&d)?, n))).collect()
    };
    let days = (0..DAYS_SHOWN)
        .rev()
        .map(|i| {
            let n = today_n - i;
            DayCount { day: format_day(n), chars: per_day.get(&n).copied().unwrap_or(0) }
        })
        .collect();
    let read: Vec<i64> = per_day.iter().filter(|(_, n)| **n > 0).map(|(d, _)| *d).collect();
    let (streak, best_streak) = streaks(&read, today_n);

    let ms_day = 24 * 3600 * 1000;
    let since_30 = now_ms() - 30 * ms_day;
    let count = |sql: &str, p: &[&dyn rusqlite::ToSql]| -> Result<i64> { Ok(c.query_row(sql, p, |r| r.get(0))?) };
    let known_words = count(
        "SELECT count(*) FROM (
             SELECT lemma, reading FROM knowledge WHERE lang = ?1 AND state IN ('known', 'fading')
             UNION SELECT lemma, reading FROM word_overrides WHERE lang = ?1 AND state = 'marked_known')",
        &[&lang],
    )?;
    let texts_read = count(
        "SELECT count(DISTINCT text_id) FROM events WHERE lang = ?1 AND kind = 'exposure' AND text_id IS NOT NULL",
        &[&lang],
    )?;
    let words_seen_30d = count(
        "SELECT count(*) FROM (SELECT DISTINCT lemma, reading FROM events WHERE lang = ?1 AND kind = 'exposure' AND ts >= ?2)",
        &[&lang, &since_30],
    )?;
    let words_looked_up_30d = count(
        "SELECT count(*) FROM (SELECT DISTINCT lemma, reading FROM events WHERE lang = ?1 AND kind = 'lookup' AND ts >= ?2)",
        &[&lang, &since_30],
    )?;
    let since_leech = now_ms() - LEECH_WINDOW_DAYS * ms_day;
    let stealth_leeches = friction_rows(
        store,
        &lookups_sql(&format!("HAVING k.state IN ('known', 'fading') AND count(*) >= {LEECH_MIN_LOOKUPS}")),
        &[&lang, &since_leech],
    )?;
    let most_looked_up = friction_rows(store, &lookups_sql("HAVING count(*) >= 2"), &[&lang, &since_leech])?;

    Ok(Stats {
        known_words,
        chars_total: per_day.values().sum(),
        chars_today: per_day.get(&today_n).copied().unwrap_or(0),
        days,
        streak,
        best_streak,
        texts_read,
        words_seen_30d,
        words_looked_up_30d,
        stealth_leeches,
        most_looked_up,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::store::NewEvent;

    #[test]
    fn dates_round_trip() {
        for d in ["1970-01-01", "2000-02-29", "2026-10-27", "2026-12-31", "2027-01-01"] {
            assert_eq!(format_day(parse_day(d).unwrap()), d);
        }
        assert_eq!(parse_day("2027-01-01").unwrap() - parse_day("2026-12-31").unwrap(), 1);
        assert_eq!(parse_day("2024-03-01").unwrap() - parse_day("2024-02-28").unwrap(), 2);
    }

    #[test]
    fn streaks_count_back_from_today_or_yesterday() {
        let t = parse_day("2026-10-27").unwrap();
        assert_eq!(streaks(&[t, t - 1, t - 2, t - 5, t - 6, t - 7, t - 8], t), (3, 4));
        assert_eq!(streaks(&[t - 1, t - 2], t), (2, 2), "today not read yet keeps the streak");
        assert_eq!(streaks(&[t - 2], t), (0, 1));
        assert_eq!(streaks(&[], t), (0, 0));
    }

    #[test]
    fn stats_and_stealth_leeches() {
        let mut s = Store::open_in_memory().unwrap();
        let key = |l: &str, r: &str| WordKey { lang: "ja".into(), lemma: l.into(), reading: r.into() };
        let card = |id| CardFacts { card_id: id, note_id: id, card_type: 2, queue: 2, interval: 60, lapses: 0, stability: None, retrievability: None };
        s.replace_anki_snapshot(
            &[(1, card(1)), (2, card(2))],
            &[(1, key("猫", "ねこ")), (2, key("犬", "いぬ"))],
            &HashMap::new(),
            &KnowledgeSettings::default(),
        )
        .unwrap();
        let ev = |kind: &str, k: WordKey| NewEvent { kind: kind.into(), key: k, text_id: Some("t1".into()), display: None };
        s.record_events(&[
            ev("exposure", key("猫", "ねこ")),
            ev("exposure", key("傘", "かさ")),
            ev("lookup", key("猫", "ねこ")),
            ev("lookup", key("猫", "ねこ")), // known in Anki, looked up twice: a stealth leech
            ev("lookup", key("犬", "いぬ")), // only once
            ev("lookup", key("傘", "かさ")),
            ev("lookup", key("傘", "かさ")),
            ev("lookup", key("傘", "かさ")), // not in Anki: friction, not a leech
        ])
        .unwrap();
        s.record_reading("ja", "2026-10-26", 300).unwrap();
        s.record_reading("ja", "2026-10-27", 100).unwrap();
        s.record_reading("ja", "2026-10-27", 50).unwrap();

        let st = stats(&s, "ja", "2026-10-27").unwrap();
        assert_eq!((st.chars_total, st.chars_today, st.streak, st.best_streak), (450, 150, 2, 2));
        assert_eq!(st.days.len(), 30);
        assert_eq!(st.days.last().unwrap().day, "2026-10-27");
        assert_eq!(st.days[28].chars, 300);
        assert_eq!((st.known_words, st.texts_read, st.words_seen_30d, st.words_looked_up_30d), (2, 1, 2, 3));
        let leeches: Vec<_> = st.stealth_leeches.iter().map(|f| (f.key.lemma.as_str(), f.lookups)).collect();
        assert_eq!(leeches, vec![("猫", 2)]);
        let most: Vec<_> = st.most_looked_up.iter().map(|f| (f.key.lemma.as_str(), f.lookups, f.state)).collect();
        assert_eq!(most, vec![("傘", 3, WordState::Unknown), ("猫", 2, WordState::Known)]);
    }
}
