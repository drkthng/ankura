//! The daily session (ADR 0019): today's practice words, chosen to rescue the Anki backlog and to
//! bring back words the learner struggles with, and the day's progress through the steps.

use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::store::{Store, now_ms};
use crate::{Result, WordKey};

pub(crate) const TODAY_SCHEMA: &str = r#"
CREATE TABLE today_sessions (
    day TEXT PRIMARY KEY,            -- the learner's local date, YYYY-MM-DD
    data TEXT NOT NULL               -- JSON (Session)
);
"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pick {
    pub key: WordKey,
    pub display: String,
    /// Why it was picked: 'fading' (overdue in Anki), 'stealth_leech', 'quiz_missed',
    /// 'looked_up', 'learning'.
    pub reason: String,
}

const DAY_MS: i64 = 24 * 3600 * 1000;

fn keys(store: &Store, sql: &str, args: &[&dyn rusqlite::ToSql], reason: &str) -> Result<Vec<Pick>> {
    let mut st = store.conn.prepare(sql)?;
    let rows = st.query_map(args, |r| {
        Ok(Pick {
            key: WordKey { lang: r.get(0)?, lemma: r.get(1)?, reading: r.get(2)? },
            display: r.get(3)?,
            reason: reason.into(),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Not marked known or ignored by the learner.
const NOT_OVERRIDDEN: &str = "NOT EXISTS (SELECT 1 FROM word_overrides o WHERE o.lang = x.lang AND o.lemma = x.lemma
                               AND o.reading = x.reading AND o.state IN ('marked_known', 'ignored'))";

/// Words used as targets in texts written since `since` (to vary the words from day to day).
fn recent_targets(store: &Store, since: i64) -> Result<HashSet<String>> {
    let mut st = store.conn.prepare("SELECT extra FROM texts WHERE created_at >= ?1 AND extra IS NOT NULL")?;
    let rows = st.query_map([since], |r| r.get::<_, String>(0))?;
    let mut out = HashSet::new();
    for row in rows {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&row?) {
            for t in v["targets"].as_array().into_iter().flatten().filter_map(|t| t.as_str()) {
                out.insert(t.to_string());
            }
        }
    }
    Ok(out)
}

/// Today's practice words (backlog rescue): about half from fading Anki words (overdue cards; the
/// best-established first, the ones still easy to save), then stealth leeches, words missed in
/// text questions, words looked up, and words being learned. Words used as targets in the last
/// day are left for later.
pub fn practice_words(store: &Store, lang: &str, n: usize) -> Result<Vec<Pick>> {
    let since_60 = now_ms() - 60 * DAY_MS;
    let since_14 = now_ms() - 14 * DAY_MS;
    let used = recent_targets(store, now_ms() - DAY_MS)?;
    let fading = keys(
        store,
        &format!(
            "SELECT x.lang, x.lemma, x.reading, coalesce(x.display, x.lemma) FROM knowledge x
             WHERE x.lang = ?1 AND x.state = 'fading' AND {NOT_OVERRIDDEN} ORDER BY x.strength DESC LIMIT 60"
        ),
        &[&lang],
        "fading",
    )?;
    let leeches = keys(
        store,
        &format!(
            "SELECT x.lang, x.lemma, x.reading, coalesce(max(x.display), x.lemma) FROM events x
             JOIN knowledge k ON k.lang = x.lang AND k.lemma = x.lemma AND k.reading = x.reading
             WHERE x.kind = 'lookup' AND x.lang = ?1 AND x.ts >= ?2 AND k.state IN ('known', 'fading') AND {NOT_OVERRIDDEN}
             GROUP BY x.lang, x.lemma, x.reading HAVING count(*) >= 2 ORDER BY count(*) DESC LIMIT 20"
        ),
        &[&lang, &since_60],
        "stealth_leech",
    )?;
    let missed = keys(
        store,
        &format!(
            "SELECT x.lang, x.lemma, x.reading, coalesce(max(x.display), x.lemma) FROM events x
             WHERE x.kind = 'quiz_missed' AND x.lang = ?1 AND x.ts >= ?2 AND {NOT_OVERRIDDEN}
             GROUP BY x.lang, x.lemma, x.reading ORDER BY max(x.ts) DESC LIMIT 20"
        ),
        &[&lang, &since_14],
        "quiz_missed",
    )?;
    let looked = keys(
        store,
        &format!(
            "SELECT x.lang, x.lemma, x.reading, coalesce(max(x.display), x.lemma) FROM events x
             WHERE x.kind = 'dont_know' AND x.lang = ?1 AND x.ts >= ?2 AND {NOT_OVERRIDDEN}
             GROUP BY x.lang, x.lemma, x.reading ORDER BY count(*) DESC, max(x.ts) DESC LIMIT 20"
        ),
        &[&lang, &since_60],
        "looked_up",
    )?;
    let learning = keys(
        store,
        &format!(
            "SELECT x.lang, x.lemma, x.reading, coalesce(x.display, x.lemma) FROM knowledge x
             WHERE x.lang = ?1 AND x.state = 'learning' AND {NOT_OVERRIDDEN} ORDER BY x.strength DESC LIMIT 20"
        ),
        &[&lang],
        "learning",
    )?;

    let mut out: Vec<Pick> = Vec::new();
    let take = |from: &[Pick], k: usize, out: &mut Vec<Pick>| {
        let mut added = 0;
        for p in from {
            if out.len() >= n || added >= k {
                break;
            }
            if used.contains(&p.key.lemma) || used.contains(&p.display) || out.iter().any(|o| o.key == p.key) {
                continue;
            }
            out.push(p.clone());
            added += 1;
        }
    };
    let half = n.div_ceil(2);
    take(&fading, half, &mut out);
    take(&leeches, 1, &mut out);
    take(&missed, 1, &mut out);
    take(&looked, 1, &mut out);
    // Fill up: more fading words first, then the rest.
    for list in [&fading, &leeches, &missed, &looked, &learning] {
        take(list, n, &mut out);
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct Rescue {
    /// Words Anki has as fading (overdue) now.
    pub fading: i64,
    /// How many of them the learner met in a text in the last 7 days.
    pub met_this_week: i64,
}

/// Progress on the backlog: fading words met in texts this week.
pub fn rescue(store: &Store, lang: &str) -> Result<Rescue> {
    let since = now_ms() - 7 * DAY_MS;
    let c = &store.conn;
    let fading: i64 = c.query_row("SELECT count(*) FROM knowledge WHERE lang = ?1 AND state = 'fading'", [lang], |r| r.get(0))?;
    let met: i64 = c.query_row(
        "SELECT count(*) FROM knowledge k WHERE k.lang = ?1 AND k.state = 'fading' AND EXISTS (
             SELECT 1 FROM events e WHERE e.kind = 'exposure' AND e.lang = k.lang AND e.lemma = k.lemma
               AND e.reading = k.reading AND e.ts >= ?2)",
        params![lang, since],
        |r| r.get(0),
    )?;
    Ok(Rescue { fading, met_this_week: met })
}

/// The day's progress through the session's steps.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub day: String,
    /// The words chosen for today (the learner can change them before writing).
    #[serde(default)]
    pub words: Vec<Pick>,
    /// Today's text, once written.
    #[serde(default)]
    pub text_id: Option<String>,
    /// Steps done: 'read', 'listened', 'questions', 'cards'.
    #[serde(default)]
    pub done: Vec<String>,
}

pub fn session(store: &Store, day: &str) -> Result<Session> {
    let data: Option<String> =
        store.conn.query_row("SELECT data FROM today_sessions WHERE day = ?1", [day], |r| r.get(0)).optional()?;
    Ok(data.and_then(|d| serde_json::from_str(&d).ok()).unwrap_or_else(|| Session { day: day.into(), ..Default::default() }))
}

pub fn save_session(store: &Store, s: &Session) -> Result<()> {
    store.conn.execute(
        "INSERT INTO today_sessions (day, data) VALUES (?1, ?2) ON CONFLICT(day) DO UPDATE SET data = excluded.data",
        params![s.day, serde_json::to_string(s)?],
    )?;
    Ok(())
}

/// Marks a step done for the day (idempotent).
pub fn mark(store: &Store, day: &str, step: &str) -> Result<Session> {
    let mut s = session(store, day)?;
    if !s.done.iter().any(|d| d == step) {
        s.done.push(step.to_string());
    }
    save_session(store, &s)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::store::NewEvent;

    fn key(l: &str) -> WordKey {
        WordKey { lang: "ja".into(), lemma: l.into(), reading: String::new() }
    }

    #[test]
    fn picks_fading_first_then_struggling_words() {
        let mut s = Store::open_in_memory().unwrap();
        // Ten words in Anki: interval 100 days, 200 days overdue → fading (FSRS off); one known.
        let words: Vec<String> = (0..10).map(|i| format!("語{i}")).collect();
        let mut cards: Vec<(i64, CardFacts)> = (0..10i64)
            .map(|i| (1, CardFacts { card_id: i, note_id: i, card_type: 2, queue: 2, interval: 100 + i, lapses: 0, stability: None, retrievability: Some(0.3) }))
            .collect();
        cards.push((1, CardFacts { card_id: 99, note_id: 99, card_type: 2, queue: 2, interval: 100, lapses: 0, stability: None, retrievability: Some(0.95) }));
        let mut keys: Vec<(i64, WordKey)> = words.iter().enumerate().map(|(i, w)| (i as i64, key(w))).collect();
        keys.push((99, key("猫")));
        s.replace_anki_snapshot(&cards, &keys, &HashMap::new(), &KnowledgeSettings::default()).unwrap();
        let ev = |kind: &str, l: &str| NewEvent { kind: kind.into(), key: key(l), text_id: None, display: None };
        s.record_events(&[ev("lookup", "猫"), ev("lookup", "猫"), ev("quiz_missed", "鳥"), ev("dont_know", "犬")]).unwrap();

        let picks = practice_words(&s, "ja", 6).unwrap();
        let reasons: Vec<&str> = picks.iter().map(|p| p.reason.as_str()).collect();
        assert_eq!(reasons, ["fading", "fading", "fading", "stealth_leech", "quiz_missed", "looked_up"], "{picks:?}");
        // The best-established fading cards come first (longest interval).
        assert_eq!(picks[0].key.lemma, "語9");

        // Words used in a text since yesterday wait.
        s.add_text_with_extra("ja", "t", "本文", "generated", Some(&serde_json::json!({"targets": ["語9", "猫"]}))).unwrap();
        let picks = practice_words(&s, "ja", 6).unwrap();
        assert!(picks.iter().all(|p| p.key.lemma != "語9" && p.key.lemma != "猫"), "{picks:?}");

        let r = rescue(&s, "ja").unwrap();
        assert_eq!((r.fading, r.met_this_week), (10, 0));
        s.record_events(&[ev("exposure", "語3")]).unwrap();
        assert_eq!(rescue(&s, "ja").unwrap().met_this_week, 1);
    }

    #[test]
    fn session_steps_are_kept_per_day() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(session(&s, "2026-10-30").unwrap().done, Vec::<String>::new());
        mark(&s, "2026-10-30", "read").unwrap();
        let x = mark(&s, "2026-10-30", "read").unwrap();
        assert_eq!(x.done, vec!["read"]);
        assert!(session(&s, "2026-10-31").unwrap().done.is_empty());
    }
}
