//! Review (ADR 0022): short rounds on what the learner is about to forget. Words Anki has as
//! fading, words met in Ankura that have no card (looked up, missed in questions), and kanji, on
//! Ankura's own schedule (`fsrs`). Words are asked in a sentence the learner read, with a gap.
//! A word recalled without help is also answered "Good" on its Anki card, if that card is due.

use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::anki::AnkiSource;
use crate::fsrs::{self, Grade, Memory};
use crate::kana::{is_kana, is_kanji, romaji_to_hiragana, to_hiragana};
use crate::knowledge::WordState;
use crate::proficiency::Rng;
use crate::store::{Store, now_ms};
use crate::tokenize::Tokenizer;
use crate::{Error, Result, WordKey};

pub(crate) const REVIEW_SCHEMA: &str = r#"
-- Ankura's own schedule: one row per word or kanji answered in Review.
CREATE TABLE review_items (
    kind TEXT NOT NULL,            -- 'word' | 'kanji'
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,           -- the word key's lemma, or the kanji
    reading TEXT NOT NULL,
    display TEXT NOT NULL,
    stability REAL NOT NULL,       -- days until recall falls to 90% (FSRS)
    difficulty REAL NOT NULL,
    due INTEGER NOT NULL,          -- ms since epoch
    last INTEGER NOT NULL,         -- the last answer, ms since epoch
    reps INTEGER NOT NULL,
    lapses INTEGER NOT NULL,
    PRIMARY KEY (kind, lang, lemma, reading)
);
-- Every answer given in Review.
CREATE TABLE review_log (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    ts INTEGER NOT NULL,
    day TEXT NOT NULL,             -- the learner's local date, YYYY-MM-DD
    kind TEXT NOT NULL,
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,
    reading TEXT NOT NULL,
    display TEXT NOT NULL,
    mode TEXT NOT NULL,            -- 'gap' | 'listen' | 'match' | 'kanji'
    result TEXT NOT NULL,          -- 'recalled' | 'helped' | 'missed'
    points INTEGER NOT NULL
);
CREATE INDEX review_log_day ON review_log (day);
-- The one word an Anki card tests, and the card's sentence if it has one (filled by sync).
CREATE TABLE review_cards (
    card_id INTEGER PRIMARY KEY,
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,
    reading TEXT NOT NULL,
    sentence TEXT
);
CREATE INDEX review_cards_key ON review_cards (lang, lemma, reading);
-- Answers for Anki cards: waiting to be sent, and the log of what was sent.
CREATE TABLE anki_answers (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    ts INTEGER NOT NULL,
    card_id INTEGER NOT NULL,
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,
    reading TEXT NOT NULL,
    display TEXT NOT NULL,
    ease INTEGER NOT NULL,         -- 3: Good
    status TEXT NOT NULL,          -- 'pending' | 'sent' | 'not_due' | 'failed' | 'expired'
    detail TEXT,
    sent_at INTEGER
);
"#;

const DAY_MS: i64 = 24 * 3600 * 1000;
/// Right answers a day, unless the learner sets another goal (setting `review_goal`).
pub const DEFAULT_GOAL: i64 = 20;
/// A missed item comes back after four hours at the earliest.
const MIN_DAYS: f64 = 4.0 / 24.0;
/// An item answered with help comes back within three days, to be recalled without help.
const HELPED_MAX_DAYS: f64 = 3.0;
/// A missed item comes back within a day.
const MISSED_MAX_DAYS: f64 = 1.0;
/// Good, as Anki's answer buttons count (1 Again … 4 Easy).
const EASE_GOOD: i64 = 3;

/// Not marked known or ignored by the learner.
const NOT_OVERRIDDEN: &str = "NOT EXISTS (SELECT 1 FROM word_overrides o WHERE o.lang = x.lang AND o.lemma = x.lemma
                               AND o.reading = x.reading AND o.state IN ('marked_known', 'ignored'))";
/// Not yet on Ankura's own schedule.
const NOT_TRACKED: &str = "NOT EXISTS (SELECT 1 FROM review_items i WHERE i.kind = 'word' AND i.lang = x.lang
                            AND i.lemma = x.lemma AND i.reading = x.reading)";

// --- what sync tells Review about the cards ---

/// The word a card tests, with the card's own sentence when the word is marked in one.
#[derive(Debug, Clone, PartialEq)]
pub struct CardWord {
    pub card_id: i64,
    pub key: WordKey,
    pub sentence: Option<String>,
}

/// Replaces what Review knows about the Anki cards (called by `mapping::sync_anki`).
pub fn replace_cards(store: &mut Store, cards: &[CardWord]) -> Result<()> {
    let tx = store.conn.transaction()?;
    tx.execute("DELETE FROM review_cards", [])?;
    {
        let mut ins =
            tx.prepare("INSERT OR REPLACE INTO review_cards (card_id, lang, lemma, reading, sentence) VALUES (?1, ?2, ?3, ?4, ?5)")?;
        for c in cards {
            ins.execute(params![c.card_id, c.key.lang, c.key.lemma, c.key.reading, c.sentence])?;
        }
    }
    tx.commit()?;
    Ok(())
}

// --- what is due ---

#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    key: WordKey,
    display: String,
}

fn word_rows(store: &Store, lang: &str, sql: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<Candidate>> {
    let mut st = store.conn.prepare(sql)?;
    let rows = st.query_map(args, |r| {
        Ok(Candidate { key: WordKey { lang: lang.to_string(), lemma: r.get(0)?, reading: r.get(1)? }, display: r.get(2)? })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

struct Queue {
    /// On Ankura's schedule and due.
    own: Vec<Candidate>,
    /// Met in Ankura, never reviewed: looked up, missed in questions, or known in Anki but looked
    /// up again and again.
    fresh: Vec<Candidate>,
    /// Fading in Anki, the best-established first.
    fading: Vec<Candidate>,
}

fn word_queue(store: &Store, lang: &str, now: i64) -> Result<Queue> {
    let since_90 = now - 90 * DAY_MS;
    let own = word_rows(
        store,
        lang,
        &format!(
            "SELECT x.lemma, x.reading, x.display FROM review_items x
             WHERE x.kind = 'word' AND x.lang = ?1 AND x.due <= ?2 AND {NOT_OVERRIDDEN} ORDER BY x.due LIMIT 500"
        ),
        &[&lang, &now],
    )?;
    let mut fresh = word_rows(
        store,
        lang,
        &format!(
            "SELECT x.lemma, x.reading, coalesce(max(x.display), x.lemma) FROM events x
             WHERE x.lang = ?1 AND x.kind IN ('dont_know', 'quiz_missed') AND x.ts >= ?2 AND {NOT_TRACKED} AND {NOT_OVERRIDDEN}
             GROUP BY x.lang, x.lemma, x.reading ORDER BY max(x.ts) DESC LIMIT 300"
        ),
        &[&lang, &since_90],
    )?;
    let leeches = word_rows(
        store,
        lang,
        &format!(
            "SELECT x.lemma, x.reading, coalesce(max(x.display), x.lemma) FROM events x
             JOIN knowledge k ON k.lang = x.lang AND k.lemma = x.lemma AND k.reading = x.reading
             WHERE x.lang = ?1 AND x.kind = 'lookup' AND x.ts >= ?2 AND k.state = 'known' AND {NOT_TRACKED} AND {NOT_OVERRIDDEN}
             GROUP BY x.lang, x.lemma, x.reading HAVING count(*) >= 2 ORDER BY count(*) DESC LIMIT 100"
        ),
        &[&lang, &since_90],
    )?;
    for l in leeches {
        if !fresh.iter().any(|f| f.key == l.key) {
            fresh.push(l);
        }
    }
    let mut fading = word_rows(
        store,
        lang,
        &format!(
            "SELECT x.lemma, x.reading, coalesce(x.display, x.lemma) FROM knowledge x
             WHERE x.lang = ?1 AND x.state = 'fading' AND {NOT_TRACKED} AND {NOT_OVERRIDDEN} ORDER BY x.strength DESC LIMIT 500"
        ),
        &[&lang],
    )?;
    fading.retain(|f| !fresh.iter().any(|x| x.key == f.key));
    Ok(Queue { own, fresh, fading })
}

/// Words that aren't due, for practising anyway: the next ones on Ankura's schedule, then words
/// being learned in Anki.
fn words_ahead(store: &Store, lang: &str, now: i64) -> Result<Vec<Candidate>> {
    let mut out = word_rows(
        store,
        lang,
        &format!(
            "SELECT x.lemma, x.reading, x.display FROM review_items x
             WHERE x.kind = 'word' AND x.lang = ?1 AND x.due > ?2 AND {NOT_OVERRIDDEN} ORDER BY x.due LIMIT 40"
        ),
        &[&lang, &now],
    )?;
    out.extend(word_rows(
        store,
        lang,
        &format!(
            "SELECT x.lemma, x.reading, coalesce(x.display, x.lemma) FROM knowledge x
             WHERE x.lang = ?1 AND x.state = 'learning' AND {NOT_TRACKED} AND {NOT_OVERRIDDEN} ORDER BY x.strength DESC LIMIT 40"
        ),
        &[&lang],
    )?);
    Ok(out)
}

/// Up to `n` words for a round: about half from Ankura's own schedule, some new from reading,
/// the rest fading Anki words; then whatever is left, in that order.
fn pick_words(q: &Queue, n: usize) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let take = |from: &[Candidate], k: usize, out: &mut Vec<Candidate>| {
        let mut added = 0;
        for c in from {
            if out.len() >= n || added >= k {
                break;
            }
            if out.iter().any(|o| o.key == c.key) {
                continue;
            }
            out.push(c.clone());
            added += 1;
        }
    };
    take(&q.own, n.div_ceil(2), &mut out);
    take(&q.fresh, (n / 4).max(1), &mut out);
    take(&q.fading, n, &mut out);
    for list in [&q.own, &q.fresh, &q.fading] {
        take(list, n, &mut out);
    }
    out
}

struct KanjiRow {
    kanji: String,
    keyword: String,
    story: String,
}

fn kanji_rows(store: &Store, sql: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<KanjiRow>> {
    let mut st = store.conn.prepare(sql)?;
    let rows = st.query_map(args, |r| Ok(KanjiRow { kanji: r.get(0)?, keyword: r.get(1)?, story: r.get(2)? }))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Kanji that are due: on Ankura's schedule first, then fading in Anki (best-established first).
fn kanji_queue(store: &Store, now: i64) -> Result<Vec<KanjiRow>> {
    let mut out = kanji_rows(
        store,
        "SELECT k.kanji, k.keyword, k.story FROM review_items x JOIN kanji k ON k.kanji = x.lemma
         WHERE x.kind = 'kanji' AND x.due <= ?1 AND k.keyword != '' ORDER BY x.due LIMIT 500",
        &[&now],
    )?;
    out.extend(kanji_rows(
        store,
        "SELECT k.kanji, k.keyword, k.story FROM kanji k
         WHERE k.state = 'fading' AND k.keyword != ''
           AND NOT EXISTS (SELECT 1 FROM review_items i WHERE i.kind = 'kanji' AND i.lemma = k.kanji)
         ORDER BY k.strength DESC LIMIT 500",
        &[],
    )?);
    Ok(out)
}

// --- the items of a round ---

/// One thing to answer. Words: a sentence with a gap (`before`, `after`; both empty when no
/// sentence was found) and the meaning as the cue. Kanji: four `options`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// 'word' or 'kanji'.
    pub kind: String,
    pub key: WordKey,
    /// The word in its dictionary form as the learner knows it, or the kanji.
    pub display: String,
    /// Reading of the dictionary form.
    pub reading: String,
    /// Short meaning (the kanji's keyword for kanji).
    pub gloss: String,
    /// Anki has a card for it.
    pub has_card: bool,
    pub before: String,
    pub after: String,
    /// The word as it stands in the sentence (the dictionary form when there is no sentence).
    pub answer: String,
    pub answer_reading: String,
    /// Where the sentence is from: a text's title, or "your Anki card".
    pub from: String,
    /// Typed answers that count as right.
    pub accept: Vec<String>,
    /// Four choices (dictionary forms; keywords or kanji in the kanji round); empty when Ankura
    /// doesn't know enough other words.
    pub options: Vec<String>,
    pub option_answer: usize,
    /// Kanji round: 'keyword' (the kanji is shown, pick its keyword) or 'kanji' (the reverse).
    pub ask: String,
    /// Kanji round: the mnemonic story, shown after the answer.
    pub story: String,
}

fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max).collect();
    t.push('…');
    t
}

/// The first sense the dictionary has for the word, shortened.
fn gloss(store: &Store, key: &WordKey, display: &str) -> Result<String> {
    let mut entries = crate::dict::lookup(store, display, &key.reading)?;
    if entries.is_empty() && display != key.lemma {
        entries = crate::dict::lookup(store, &key.lemma, &key.reading)?;
    }
    let sense = entries.iter().flat_map(|e| e.senses.iter()).map(|s| s.trim()).find(|s| !s.is_empty()).unwrap_or("");
    Ok(shorten(sense.lines().next().unwrap_or(""), 90))
}

/// A word without its last kana (食べる → 食べ), to find its inflected forms in a text.
fn stem(word: &str) -> String {
    let chars: Vec<char> = word.chars().collect();
    match chars.last() {
        Some(c) if chars.len() > 1 && crate::kana::is_hiragana(*c) => chars[..chars.len() - 1].iter().collect(),
        _ => word.to_string(),
    }
}

fn flush(cur: &mut String, out: &mut Vec<String>) {
    let s = cur.trim();
    if !s.is_empty() {
        out.push(s.to_string());
    }
    cur.clear();
}

/// Splits a text into sentences, keeping the closing punctuation and quotes.
fn sentences(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            flush(&mut cur, &mut out);
            continue;
        }
        cur.push(c);
        if matches!(c, '。' | '！' | '？' | '!' | '?') {
            while let Some(&n) = chars.peek() {
                if matches!(n, '」' | '』' | '）' | ')' | '。' | '！' | '？' | '!' | '?') {
                    cur.push(n);
                    chars.next();
                } else {
                    break;
                }
            }
            flush(&mut cur, &mut out);
        }
    }
    flush(&mut cur, &mut out);
    out
}

struct Found {
    before: String,
    after: String,
    surface: String,
    reading: String,
    from: String,
}

/// The sentence with the word taken out, if the word is in it and something is left around it.
fn blank(
    store: &Store,
    tok: &dyn Tokenizer,
    sentence: &str,
    key: &WordKey,
    text_id: Option<&str>,
    from: &str,
) -> Result<Option<Found>> {
    let doc = crate::reader::analyze_text(store, tok, sentence, text_id)?;
    let toks: Vec<&crate::reader::ReaderToken> = doc.paragraphs.iter().flatten().collect();
    let exact = toks.iter().position(|t| t.key.as_ref() == Some(key));
    let by_lemma = || toks.iter().position(|t| t.key.as_ref().is_some_and(|k| k.lemma == key.lemma));
    let Some(pos) = exact.or_else(by_lemma) else { return Ok(None) };
    let before: String = toks[..pos].iter().map(|t| t.surface.as_str()).collect();
    let after: String = toks[pos + 1..].iter().map(|t| t.surface.as_str()).collect();
    let context = before.chars().chain(after.chars()).filter(|c| is_kana(*c) || is_kanji(*c)).count();
    if context < 2 {
        return Ok(None);
    }
    let t = toks[pos];
    let reading: String = t.segments.iter().map(|g| g.ruby.as_deref().unwrap_or(&g.text)).collect();
    Ok(Some(Found { before, after, surface: t.surface.clone(), reading: to_hiragana(&reading), from: from.to_string() }))
}

/// A sentence with this word: from the texts where the learner met it (newest first), then from
/// other texts they opened, then the sentence on their own Anki card. `seed` picks among several.
fn find_sentence(store: &Store, tok: &dyn Tokenizer, key: &WordKey, display: &str, seed: u64) -> Result<Option<Found>> {
    let mut stems: Vec<String> = vec![stem(display), stem(&key.lemma)];
    if key.reading.chars().count() >= 3 {
        stems.push(stem(&key.reading));
    }
    stems.retain(|s| !s.is_empty());
    stems.dedup();

    let mut texts: Vec<(String, String, String)> = Vec::new();
    {
        let mut st = store.conn.prepare_cached(
            "SELECT t.id, t.title, t.body FROM texts t WHERE t.lang = ?1 AND t.id IN (
                 SELECT e.text_id FROM events e WHERE e.kind = 'exposure' AND e.lang = ?1 AND e.lemma = ?2
                   AND e.reading = ?3 AND e.text_id IS NOT NULL)
             ORDER BY coalesce(t.opened_at, t.created_at) DESC LIMIT 4",
        )?;
        let rows = st.query_map(params![key.lang, key.lemma, key.reading], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        for row in rows {
            texts.push(row?);
        }
    }
    if texts.len() < 4 {
        let mut st = store.conn.prepare_cached(
            "SELECT id, title, body FROM texts
             WHERE lang = ?1 AND opened_at IS NOT NULL AND (instr(body, ?2) > 0 OR instr(body, ?3) > 0)
             ORDER BY opened_at DESC LIMIT 4",
        )?;
        let rows = st.query_map(params![key.lang, stem(display), stem(&key.lemma)], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        for row in rows {
            let t: (String, String, String) = row?;
            if !texts.iter().any(|x| x.0 == t.0) {
                texts.push(t);
            }
        }
    }

    let mut found: Vec<Found> = Vec::new();
    let mut tried = 0;
    'texts: for (id, title, body) in &texts {
        for s in sentences(body) {
            if !(5..=90).contains(&s.chars().count()) || !stems.iter().any(|x| s.contains(x.as_str())) {
                continue;
            }
            tried += 1;
            if let Some(f) = blank(store, tok, &s, key, Some(id.as_str()), title)? {
                found.push(f);
            }
            if found.len() >= 3 || tried >= 12 {
                break 'texts;
            }
        }
    }
    if found.is_empty() {
        let own: Option<String> = store
            .conn
            .query_row(
                "SELECT sentence FROM review_cards WHERE lang = ?1 AND lemma = ?2 AND reading = ?3 AND sentence IS NOT NULL LIMIT 1",
                params![key.lang, key.lemma, key.reading],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(s) = own {
            if let Some(f) = blank(store, tok, &s, key, None, "your Anki card")? {
                found.push(f);
            }
        }
    }
    if found.is_empty() {
        return Ok(None);
    }
    let i = (seed % found.len() as u64) as usize;
    Ok(Some(found.swap_remove(i)))
}

/// Words that end alike make better wrong choices (verbs with verbs, more or less).
fn ends_alike(a: &str, b: &str) -> bool {
    match (a.chars().last(), b.chars().last()) {
        (Some(x), Some(y)) => (is_kana(x) && x == y) || (is_kanji(x) && is_kanji(y)),
        _ => false,
    }
}

/// The answer among three others from `pool`, shuffled; empty when the pool is too small.
fn choices(answer: &str, pool: &[String], rng: &mut Rng) -> (Vec<String>, usize) {
    let mut alike: Vec<&String> = pool.iter().filter(|p| p.as_str() != answer && ends_alike(p, answer)).collect();
    let mut other: Vec<&String> = pool.iter().filter(|p| p.as_str() != answer && !ends_alike(p, answer)).collect();
    rng.shuffle(&mut alike);
    rng.shuffle(&mut other);
    let mut opts: Vec<String> = Vec::new();
    for p in alike.into_iter().chain(other) {
        if opts.len() == 3 {
            break;
        }
        if !opts.contains(p) {
            opts.push(p.clone());
        }
    }
    if opts.len() < 3 {
        return (Vec::new(), 0);
    }
    let at = rng.below(4);
    opts.insert(at, answer.to_string());
    (opts, at)
}

fn word_item(
    store: &Store,
    tok: Option<&dyn Tokenizer>,
    c: &Candidate,
    pool: &[String],
    rng: &mut Rng,
    seed: u64,
    want_sentence: bool,
) -> Result<Option<Item>> {
    let gloss = gloss(store, &c.key, &c.display)?;
    let found = match tok {
        Some(t) if want_sentence => find_sentence(store, t, &c.key, &c.display, seed)?,
        _ => None,
    };
    if gloss.is_empty() && found.is_none() {
        return Ok(None);
    }
    let has_card = store.knowledge(&c.key)?.is_some();
    let (before, after, answer, answer_reading, from) = match found {
        Some(f) => (f.before, f.after, f.surface, f.reading, f.from),
        None => (String::new(), String::new(), c.display.clone(), c.key.reading.clone(), String::new()),
    };
    let mut accept: Vec<String> = Vec::new();
    for a in [&answer, &answer_reading, &c.display, &c.key.lemma, &c.key.reading] {
        if !a.is_empty() && !accept.contains(a) {
            accept.push(a.clone());
        }
    }
    let (options, option_answer) = choices(&c.display, pool, rng);
    Ok(Some(Item {
        kind: "word".into(),
        key: c.key.clone(),
        display: c.display.clone(),
        reading: c.key.reading.clone(),
        gloss,
        has_card,
        before,
        after,
        answer,
        answer_reading,
        from,
        accept,
        options,
        option_answer,
        ask: String::new(),
        story: String::new(),
    }))
}

fn kanji_item(lang: &str, k: &KanjiRow, pool: &[(String, String)], rng: &mut Rng, reverse: bool) -> Option<Item> {
    let mut others: Vec<&(String, String)> = pool.iter().filter(|(kanji, keyword)| *kanji != k.kanji && *keyword != k.keyword).collect();
    rng.shuffle(&mut others);
    if others.len() < 3 {
        return None;
    }
    let mut options: Vec<String> = others[..3].iter().map(|(kanji, keyword)| if reverse { kanji.clone() } else { keyword.clone() }).collect();
    let at = rng.below(4);
    options.insert(at, if reverse { k.kanji.clone() } else { k.keyword.clone() });
    Some(Item {
        kind: "kanji".into(),
        key: WordKey { lang: lang.to_string(), lemma: k.kanji.clone(), reading: String::new() },
        display: k.kanji.clone(),
        reading: String::new(),
        gloss: k.keyword.clone(),
        has_card: true,
        before: String::new(),
        after: String::new(),
        answer: k.kanji.clone(),
        answer_reading: String::new(),
        from: String::new(),
        accept: Vec::new(),
        options,
        option_answer: at,
        ask: if reverse { "kanji".into() } else { "keyword".into() },
        story: crate::normalize::plain_text(&k.story),
    })
}

/// The items of a round of up to `n`. `mode`: 'gap' (fill the gap), 'listen' (the same, heard:
/// only words with a sentence), 'match' (word and meaning: only words with a meaning) or 'kanji'.
/// With `extra`, items that aren't due fill the round. Without the tokenizer, words come without
/// sentences.
pub fn round(store: &Store, tok: Option<&dyn Tokenizer>, lang: &str, mode: &str, extra: bool, n: usize) -> Result<Vec<Item>> {
    let now = now_ms();
    let mut rng = Rng::new();
    if mode == "kanji" {
        let mut rows = kanji_queue(store, now)?;
        if extra && rows.len() < n {
            let ahead = kanji_rows(
                store,
                "SELECT k.kanji, k.keyword, k.story FROM kanji k
                 WHERE k.state IN ('learning', 'known') AND k.keyword != '' ORDER BY k.strength LIMIT 40",
                &[],
            )?;
            for a in ahead {
                if !rows.iter().any(|r| r.kanji == a.kanji) {
                    rows.push(a);
                }
            }
        }
        let pool: Vec<(String, String)> = {
            let mut st = store.conn.prepare("SELECT kanji, keyword FROM kanji WHERE keyword != '' ORDER BY random() LIMIT 60")?;
            let found = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            found.collect::<rusqlite::Result<_>>()?
        };
        let mut out = Vec::new();
        for (i, k) in rows.iter().enumerate() {
            if out.len() >= n {
                break;
            }
            // Every other one the other way round: keyword shown, pick the kanji.
            if let Some(item) = kanji_item(lang, k, &pool, &mut rng, i % 2 == 1) {
                out.push(item);
            }
        }
        return Ok(out);
    }

    let mut cands = pick_words(&word_queue(store, lang, now)?, n * 3);
    if extra && cands.len() < n {
        for c in words_ahead(store, lang, now)? {
            if !cands.iter().any(|x| x.key == c.key) {
                cands.push(c);
            }
        }
    }
    let pool: Vec<String> = {
        let mut st = store.conn.prepare(
            "SELECT coalesce(display, lemma) FROM knowledge
             WHERE lang = ?1 AND state IN ('known', 'fading', 'learning') ORDER BY random() LIMIT 60",
        )?;
        let found = st.query_map([lang], |r| r.get(0))?;
        found.collect::<rusqlite::Result<_>>()?
    };
    let seed = (now / DAY_MS) as u64;
    let mut out: Vec<Item> = Vec::new();
    for c in &cands {
        if out.len() >= n {
            break;
        }
        let Some(item) = word_item(store, tok, c, &pool, &mut rng, seed + out.len() as u64, mode != "match")? else { continue };
        let has_sentence = !(item.before.is_empty() && item.after.is_empty());
        if (mode == "listen" && !has_sentence) || (mode == "match" && item.gloss.is_empty()) {
            continue;
        }
        out.push(item);
    }
    Ok(out)
}

// --- answers ---

fn norm(s: &str) -> String {
    let s: String = to_hiragana(&s.nfkc().collect::<String>()).chars().filter(|c| !c.is_whitespace()).collect();
    // Latin script (Italian, Spanish, romaji): capitals and accents don't count (Città = citta).
    if s.chars().all(|c| (c as u32) < 0x250) {
        s.as_str().nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect::<String>().to_lowercase()
    } else {
        s
    }
}

/// Whether a typed answer is one of the accepted ones: kanji or kana (hiragana and katakana
/// count the same), or romaji for the kana.
pub fn matches(typed: &str, accept: &[String]) -> bool {
    let t = norm(typed);
    if t.is_empty() {
        return false;
    }
    let roma = if t.is_ascii() { romaji_to_hiragana(&t) } else { None };
    accept.iter().map(|a| norm(a)).any(|a| a == t || roma.as_deref() == Some(a.as_str()))
}

#[derive(Debug, Clone, Deserialize)]
pub struct Answer {
    pub kind: String,
    pub key: WordKey,
    pub display: String,
    /// 'gap' | 'listen' | 'match' | 'kanji'.
    pub mode: String,
    /// What the learner typed; checked against `accept`.
    #[serde(default)]
    pub typed: Option<String>,
    #[serde(default)]
    pub accept: Vec<String>,
    /// For answers that aren't typed (picked, matched): whether it was right.
    #[serde(default)]
    pub right: Option<bool>,
    /// A hint was used.
    #[serde(default)]
    pub helped: bool,
    /// Right answers in a row before this one.
    #[serde(default)]
    pub combo: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub right: bool,
    /// 'recalled' (typed without help in fill-the-gap), 'helped' (right, but picked, hinted,
    /// heard or matched) or 'missed'.
    pub result: String,
    pub points: i64,
    /// Days until Ankura asks again.
    pub next_days: f64,
    /// An answer for the word's Anki card is waiting to be sent.
    pub anki: bool,
}

/// What Anki knows about the item, as (strength in days, chance of recall now), for the first
/// answer in Ankura.
fn anki_memory(store: &Store, kind: &str, key: &WordKey) -> Result<Option<(f64, f64)>> {
    let (state, strength) = if kind == "kanji" {
        match crate::kanji::get(store, &key.lemma)? {
            Some(k) => (k.state, k.strength),
            None => return Ok(None),
        }
    } else {
        match store.knowledge(key)? {
            Some(k) => (k.state, k.strength),
            None => return Ok(None),
        }
    };
    Ok(match state {
        WordState::Fading => Some((strength, 0.7)),
        WordState::Known | WordState::Learning => Some((strength, 0.9)),
        _ => None,
    })
}

/// Moves the item on Ankura's schedule; returns the days until it is due again.
fn schedule(store: &Store, kind: &str, key: &WordKey, display: &str, grade: Grade, now: i64) -> Result<f64> {
    let row: Option<(f64, f64, i64, i64, i64)> = store
        .conn
        .query_row(
            "SELECT stability, difficulty, last, reps, lapses FROM review_items
             WHERE kind = ?1 AND lang = ?2 AND lemma = ?3 AND reading = ?4",
            params![kind, key.lang, key.lemma, key.reading],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let (mem, reps, lapses) = match row {
        Some((stability, difficulty, last, reps, lapses)) => {
            let elapsed = (now - last) as f64 / DAY_MS as f64;
            let m = Memory { stability, difficulty };
            let m = if elapsed < 0.5 { fsrs::next_same_day(m, grade) } else { fsrs::next(m, fsrs::retrievability(elapsed, stability), grade) };
            (m, reps, lapses)
        }
        None => match anki_memory(store, kind, key)? {
            Some((strength, recall)) => (fsrs::next(fsrs::from_anki(strength), recall, grade), 0, 0),
            None => (fsrs::first(grade), 0, 0),
        },
    };
    let mut days = fsrs::interval(mem.stability, 0.9).max(MIN_DAYS);
    match grade {
        Grade::Again => days = days.min(MISSED_MAX_DAYS),
        Grade::Hard => days = days.min(HELPED_MAX_DAYS),
        Grade::Good => {}
    }
    let due = now + (days * DAY_MS as f64) as i64;
    store.conn.execute(
        "INSERT INTO review_items (kind, lang, lemma, reading, display, stability, difficulty, due, last, reps, lapses)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT (kind, lang, lemma, reading) DO UPDATE SET display = excluded.display, stability = excluded.stability,
             difficulty = excluded.difficulty, due = excluded.due, last = excluded.last, reps = excluded.reps,
             lapses = excluded.lapses",
        params![
            kind,
            key.lang,
            key.lemma,
            key.reading,
            display,
            mem.stability,
            mem.difficulty,
            due,
            now,
            reps + 1,
            lapses + i64::from(grade == Grade::Again)
        ],
    )?;
    Ok(days)
}

/// Queues "Good" for the word's Anki cards: review cards that test this one word and aren't
/// suspended or buried. At most one answer per card in 20 hours. Returns how many were queued.
fn queue_anki(store: &Store, key: &WordKey, display: &str, now: i64) -> Result<usize> {
    let cards: Vec<i64> = {
        let mut st = store.conn.prepare(
            "SELECT rc.card_id FROM review_cards rc JOIN anki_cards c ON c.card_id = rc.card_id
             WHERE rc.lang = ?1 AND rc.lemma = ?2 AND rc.reading = ?3 AND c.card_type = 2 AND c.queue >= 0
               AND NOT EXISTS (SELECT 1 FROM anki_answers a WHERE a.card_id = rc.card_id
                               AND a.status IN ('pending', 'sent') AND a.ts >= ?4)",
        )?;
        let rows = st.query_map(params![key.lang, key.lemma, key.reading, now - 20 * 3600 * 1000], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for card_id in &cards {
        store.conn.execute(
            "INSERT INTO anki_answers (id, device_id, ts, card_id, lang, lemma, reading, display, ease, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
            params![Uuid::new_v4().to_string(), store.device_id(), now, card_id, key.lang, key.lemma, key.reading, display, EASE_GOOD],
        )?;
    }
    Ok(cards.len())
}

/// Records one answer: checks it, moves the item on Ankura's schedule, logs it with its points,
/// and (with `to_anki`) queues "Good" for the Anki card of a word recalled without help.
/// `day` is the learner's local date.
pub fn answer(store: &Store, day: &str, a: &Answer, to_anki: bool) -> Result<Outcome> {
    let now = now_ms();
    let right = match &a.typed {
        Some(t) => matches(t, &a.accept),
        None => a.right.unwrap_or(false),
    };
    let (result, grade, base) = if !right {
        ("missed", Grade::Again, 0)
    } else if a.typed.is_some() && a.mode == "gap" && !a.helped {
        ("recalled", Grade::Good, 10)
    } else {
        ("helped", Grade::Hard, 5)
    };
    let bonus = if right { 2 * i64::from(a.combo.min(5)) } else { 0 };
    let points = base + bonus;
    let next_days = schedule(store, &a.kind, &a.key, &a.display, grade, now)?;
    store.conn.execute(
        "INSERT INTO review_log (id, device_id, ts, day, kind, lang, lemma, reading, display, mode, result, points)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            Uuid::new_v4().to_string(),
            store.device_id(),
            now,
            day,
            a.kind,
            a.key.lang,
            a.key.lemma,
            a.key.reading,
            a.display,
            a.mode,
            result,
            points
        ],
    )?;
    let anki = result == "recalled" && to_anki && a.kind == "word" && queue_anki(store, &a.key, &a.display, now)? > 0;
    Ok(Outcome { right, result: result.into(), points, next_days, anki })
}

// --- answers for Anki ---

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SendReport {
    /// Cards answered "Good" in Anki now.
    pub sent: usize,
    /// Cards that weren't due in Anki (already reviewed there): left alone.
    pub not_due: usize,
    pub failed: usize,
    /// Answers still waiting (Anki isn't running); they are dropped after a day.
    pub waiting: usize,
    /// Why answers are waiting, in plain words.
    pub error: Option<String>,
}

fn plain_anki_error(e: &str) -> String {
    if e.contains("unsupported action") {
        "Your AnkiConnect add-on can't answer cards yet. Update it in Anki (Tools → Add-ons → Check for Updates).".into()
    } else if e.contains("can't reach") {
        "Anki isn't running. The answers are sent when it is, if that's within a day.".into()
    } else {
        e.to_string()
    }
}

/// Sends the waiting answers to Anki: each card that is still due there is answered "Good", so
/// Anki's own scheduler sets its next date. Cards that aren't due are left alone. Answers older
/// than a day are dropped (the recall was then, not now). Never an error for an unreachable Anki:
/// the report says what is waiting and why.
pub fn send_anki(store: &Store, anki: &dyn AnkiSource) -> Result<SendReport> {
    let now = now_ms();
    store.conn.execute("UPDATE anki_answers SET status = 'expired' WHERE status = 'pending' AND ts < ?1", [now - DAY_MS])?;
    let pending: Vec<(String, i64, WordKey)> = {
        let mut st = store.conn.prepare("SELECT id, card_id, lang, lemma, reading FROM anki_answers WHERE status = 'pending' ORDER BY ts")?;
        let rows = st.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, WordKey { lang: r.get(2)?, lemma: r.get(3)?, reading: r.get(4)? }))
        })?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut report = SendReport::default();
    if pending.is_empty() {
        return Ok(report);
    }
    let ids: Vec<i64> = pending.iter().map(|p| p.1).collect();
    let due = match anki.are_due(&ids) {
        Ok(d) if d.len() == ids.len() => d,
        Ok(_) => {
            report.waiting = pending.len();
            report.error = Some("Anki gave an unexpected answer about which cards are due.".into());
            return Ok(report);
        }
        Err(e) => {
            report.waiting = pending.len();
            report.error = Some(plain_anki_error(&e.to_string()));
            return Ok(report);
        }
    };
    let to_send: Vec<i64> = ids.iter().zip(&due).filter(|(_, d)| **d).map(|(id, _)| *id).collect();
    let answered: Vec<bool> = if to_send.is_empty() {
        Vec::new()
    } else {
        match anki.answer_cards(&to_send, EASE_GOOD) {
            Ok(a) => a,
            Err(e) => {
                report.waiting = pending.len();
                report.error = Some(plain_anki_error(&e.to_string()));
                return Ok(report);
            }
        }
    };
    for ((id, card_id, key), is_due) in pending.iter().zip(&due) {
        let (status, detail) = if !*is_due {
            report.not_due += 1;
            ("not_due", "The card wasn't due in Anki.")
        } else {
            let ok = to_send.iter().position(|c| c == card_id).and_then(|i| answered.get(i).copied()).unwrap_or(false);
            if ok {
                report.sent += 1;
                ("sent", "")
            } else {
                report.failed += 1;
                ("failed", "Anki didn't find the card.")
            }
        };
        store.conn.execute(
            "UPDATE anki_answers SET status = ?2, detail = ?3, sent_at = ?4 WHERE id = ?1",
            params![id, status, detail, now],
        )?;
        if status == "sent" {
            // Until the next sync reads the card again: it isn't fading any more.
            store.conn.execute(
                "UPDATE knowledge SET state = 'known' WHERE lang = ?1 AND lemma = ?2 AND reading = ?3 AND state = 'fading'",
                params![key.lang, key.lemma, key.reading],
            )?;
        }
    }
    Ok(report)
}

// --- the Review screen's numbers ---

#[derive(Debug, Clone, Serialize)]
pub struct AnkiAnswer {
    pub ts: i64,
    pub display: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    /// Words due now.
    pub due_words: usize,
    /// Of those: fading in Anki and not yet answered in Ankura.
    pub due_fading: usize,
    pub due_kanji: usize,
    /// Words and kanji on Ankura's own schedule.
    pub tracked: i64,
    pub today_answers: i64,
    pub today_right: i64,
    pub today_points: i64,
    /// Right answers wanted per day.
    pub goal: i64,
    /// Days in a row with the goal met, up to today (or yesterday, if today isn't there yet).
    pub streak: i64,
    pub best_streak: i64,
    /// Anki cards answered from Ankura in the last 7 days.
    pub rescued_week: i64,
    /// Answers waiting to be sent to Anki.
    pub waiting: i64,
    /// The latest answers for Anki, newest first.
    pub anki_log: Vec<AnkiAnswer>,
}

pub fn overview(store: &Store, lang: &str, day: &str) -> Result<Overview> {
    let now = now_ms();
    let c = &store.conn;
    let q = word_queue(store, lang, now)?;
    let mut keys: HashSet<&WordKey> = HashSet::new();
    for list in [&q.own, &q.fresh, &q.fading] {
        keys.extend(list.iter().map(|x| &x.key));
    }
    let goal = store.setting("review_goal")?.and_then(|g| g.parse::<i64>().ok()).filter(|g| *g > 0).unwrap_or(DEFAULT_GOAL);
    let (today_answers, today_right, today_points): (i64, i64, i64) = c.query_row(
        "SELECT count(*), coalesce(sum(result != 'missed'), 0), coalesce(sum(points), 0) FROM review_log WHERE day = ?1",
        [day],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let met: Vec<i64> = {
        let mut st = c.prepare("SELECT day FROM review_log GROUP BY day HAVING sum(result != 'missed') >= ?1")?;
        let rows = st.query_map([goal], |r| r.get::<_, String>(0))?;
        rows.filter_map(|d| d.ok()).filter_map(|d| crate::stats::parse_day(&d)).collect()
    };
    let today_n = crate::stats::parse_day(day).ok_or_else(|| Error::Llm(format!("bad date {day}")))?;
    let (streak, best_streak) = crate::stats::streaks(&met, today_n);
    let anki_log: Vec<AnkiAnswer> = {
        let mut st = c.prepare("SELECT ts, display, status, coalesce(detail, '') FROM anki_answers ORDER BY ts DESC LIMIT 20")?;
        let rows = st.query_map([], |r| Ok(AnkiAnswer { ts: r.get(0)?, display: r.get(1)?, status: r.get(2)?, detail: r.get(3)? }))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    Ok(Overview {
        due_words: keys.len(),
        due_fading: q.fading.len(),
        due_kanji: kanji_queue(store, now)?.len(),
        tracked: c.query_row("SELECT count(*) FROM review_items", [], |r| r.get(0))?,
        today_answers,
        today_right,
        today_points,
        goal,
        streak,
        best_streak,
        rescued_week: c.query_row("SELECT count(*) FROM anki_answers WHERE status = 'sent' AND sent_at >= ?1", [now - 7 * DAY_MS], |r| {
            r.get(0)
        })?,
        waiting: c.query_row("SELECT count(*) FROM anki_answers WHERE status = 'pending'", [], |r| r.get(0))?,
        anki_log,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;
    use crate::anki::AnkiConnect;
    use crate::anki::fake::{FakeCard, FakeNote, serve_logged};
    use crate::kanji::KanjiNoteFacts;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::store::NewEvent;
    use crate::tokenize::{Token, TokenKind};

    const DAY: &str = "2026-11-15";

    /// Splits a text into the words it is given (surface, dictionary form, reading of that form,
    /// reading of the surface) and single characters, so these tests run without Sudachi.
    struct FakeTok(Vec<(&'static str, &'static str, &'static str, &'static str)>);

    impl Tokenizer for FakeTok {
        fn lang(&self) -> &'static str {
            "ja"
        }
        fn tokenize(&self, text: &str) -> Result<Vec<Token>> {
            let mut out = Vec::new();
            let mut rest = text;
            while let Some(c) = rest.chars().next() {
                let (surface, form, form_reading, reading, kind) = match self.0.iter().find(|w| rest.starts_with(w.0)) {
                    Some(w) => (w.0.to_string(), w.1.to_string(), w.2.to_string(), w.3.to_string(), TokenKind::Word),
                    None => {
                        let s = c.to_string();
                        let kind = if is_kana(c) || is_kanji(c) { TokenKind::Grammar } else { TokenKind::Symbol };
                        let reading = if is_kana(c) { s.clone() } else { String::new() };
                        (s.clone(), s.clone(), reading.clone(), reading, kind)
                    }
                };
                rest = &rest[surface.len()..];
                out.push(Token {
                    surface,
                    dictionary_form: form.clone(),
                    lemma: form,
                    lemma_reading: form_reading,
                    reading,
                    pos: Vec::new(),
                    kind,
                    parts: Vec::new(),
                });
            }
            Ok(out)
        }
    }

    fn tok() -> FakeTok {
        FakeTok(vec![("傘", "傘", "かさ", "かさ"), ("忘れた", "忘れる", "わすれる", "わすれた"), ("猫", "猫", "ねこ", "ねこ")])
    }

    fn key(lemma: &str, reading: &str) -> WordKey {
        WordKey { lang: "ja".into(), lemma: lemma.into(), reading: reading.into() }
    }

    fn event(kind: &str, k: &WordKey, text_id: Option<&str>) -> NewEvent {
        NewEvent { kind: kind.into(), key: k.clone(), text_id: text_id.map(String::from), display: Some(k.lemma.clone()) }
    }

    fn review_card(id: i64, interval: i64, recall: f64) -> (i64, CardFacts) {
        (1, CardFacts { card_id: id, note_id: id, card_type: 2, queue: 2, interval, lapses: 0, stability: None, retrievability: Some(recall) })
    }

    fn typed(k: &WordKey, text: &str, accept: &[String]) -> Answer {
        Answer {
            kind: "word".into(),
            key: k.clone(),
            display: k.lemma.clone(),
            mode: "gap".into(),
            typed: Some(text.into()),
            accept: accept.to_vec(),
            right: None,
            helped: false,
            combo: 0,
        }
    }

    #[test]
    fn typed_answers_match_kana_kanji_and_romaji() {
        let accept = vec!["忘れた".to_string(), "わすれた".to_string(), "忘れる".to_string(), "わすれる".to_string()];
        for ok in ["忘れた", "わすれた", "ワスレタ", " wasureta ", "Wasureru", "忘れる"] {
            assert!(matches(ok, &accept), "{ok}");
        }
        for no in ["", "わすれ", "wasure", "覚えた", "xyz"] {
            assert!(!matches(no, &accept), "{no}");
        }
        // Italian and Spanish: capitals and accents don't count; a different word does.
        let accept = vec!["città".to_string(), "dímelo".to_string()];
        assert!(matches("Citta", &accept) && matches("città", &accept) && matches("dimelo", &accept));
        assert!(!matches("citte", &accept));
    }

    #[test]
    fn splits_sentences() {
        assert_eq!(sentences("今日は雨だ。「傘は？」と聞いた。\n終わり"), ["今日は雨だ。", "「傘は？」", "と聞いた。", "終わり"]);
        assert_eq!(stem("食べる"), "食べ");
        assert_eq!(stem("傘"), "傘");
    }

    #[test]
    fn a_looked_up_word_is_asked_in_the_sentence_it_was_read_in() {
        let mut s = Store::open_in_memory().unwrap();
        let t = s.add_text("ja", "雨の日", "今日は雨だ。駅で傘を忘れた。", "paste").unwrap();
        let k = key("傘", "かさ");
        s.record_events(&[event("exposure", &k, Some(&t.id)), event("dont_know", &k, Some(&t.id))]).unwrap();

        let items = round(&s, Some(&tok()), "ja", "gap", false, 10).unwrap();
        assert_eq!(items.len(), 1, "{items:?}");
        let it = &items[0];
        assert_eq!((it.before.as_str(), it.answer.as_str(), it.after.as_str()), ("駅で", "傘", "を忘れた。"));
        assert_eq!((it.from.as_str(), it.has_card, it.answer_reading.as_str()), ("雨の日", false, "かさ"));
        assert!(matches("kasa", &it.accept) && matches("傘", &it.accept) && !matches("あめ", &it.accept));
        // Listening needs the sentence too; matching needs a meaning, and there is no dictionary here.
        assert_eq!(round(&s, Some(&tok()), "ja", "listen", false, 10).unwrap().len(), 1);
        assert!(round(&s, Some(&tok()), "ja", "match", false, 10).unwrap().is_empty());

        // Recalled: on Ankura's schedule, a few days out, and out of the queue. No Anki card, nothing to send.
        let o = answer(&s, DAY, &typed(&k, "かさ", &it.accept), true).unwrap();
        assert_eq!((o.right, o.result.as_str(), o.points, o.anki), (true, "recalled", 10, false));
        assert!((o.next_days - 3.1262).abs() < 0.01, "{o:?}");
        assert!(round(&s, Some(&tok()), "ja", "gap", false, 10).unwrap().is_empty());
        // Due again later: it comes back from Ankura's own schedule; a miss brings it back within a day.
        s.conn.execute("UPDATE review_items SET due = 0, last = 0", []).unwrap();
        assert_eq!(round(&s, Some(&tok()), "ja", "gap", false, 10).unwrap().len(), 1);
        let o = answer(&s, DAY, &typed(&k, "ねこ", &it.accept), true).unwrap();
        assert_eq!((o.right, o.result.as_str(), o.points), (false, "missed", 0));
        assert!(o.next_days <= 1.0, "{o:?}");
        let lapses: i64 = s.conn.query_row("SELECT lapses FROM review_items WHERE lemma = '傘'", [], |r| r.get(0)).unwrap();
        assert_eq!(lapses, 1);
        // Not due, but there for practising anyway.
        assert!(round(&s, Some(&tok()), "ja", "gap", false, 10).unwrap().is_empty());
        assert_eq!(round(&s, Some(&tok()), "ja", "gap", true, 10).unwrap().len(), 1);
    }

    /// 忘れる: fading in Anki (card 5, with its sentence), not in any text.
    fn store_with_fading_card() -> (Store, WordKey) {
        let mut s = Store::open_in_memory().unwrap();
        let k = key("忘れる", "わすれる");
        s.replace_anki_snapshot(&[review_card(5, 100, 0.3)], &[(5, k.clone())], &HashMap::new(), &KnowledgeSettings::default()).unwrap();
        replace_cards(&mut s, &[CardWord { card_id: 5, key: k.clone(), sentence: Some("かぎを忘れた。".into()) }]).unwrap();
        (s, k)
    }

    fn fake_anki(overdue: i64) -> (AnkiConnect, std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>) {
        let (url, log) = serve_logged(vec![FakeNote {
            id: 5,
            model: ("Vocab", 10),
            fields: vec![("Word", "忘れる")],
            cards: vec![FakeCard { id: 5, interval: 100, flags: &["review"], stability: None, retrievability: None, overdue }],
        }]);
        (AnkiConnect::new(&url), log)
    }

    #[test]
    fn a_fading_anki_word_recalled_is_answered_in_anki() {
        let (s, k) = store_with_fading_card();
        let items = round(&s, Some(&tok()), "ja", "gap", false, 10).unwrap();
        assert_eq!(items.len(), 1, "{items:?}");
        let it = &items[0];
        // The card's own sentence, the word in the form it has there.
        assert_eq!((it.before.as_str(), it.answer.as_str(), it.after.as_str(), it.from.as_str()), ("かぎを", "忘れた", "。", "your Anki card"));
        assert!(it.has_card);
        let ov = overview(&s, "ja", DAY).unwrap();
        assert_eq!((ov.due_words, ov.due_fading, ov.goal), (1, 1, DEFAULT_GOAL));

        let mut a = typed(&k, "wasureta", &it.accept);
        a.combo = 2;
        let o = answer(&s, DAY, &a, true).unwrap();
        assert_eq!((o.right, o.result.as_str(), o.points, o.anki), (true, "recalled", 14, true));
        assert!(o.next_days > 100.0, "an old card keeps its strength: {o:?}");
        assert_eq!(overview(&s, "ja", DAY).unwrap().waiting, 1);

        let (anki, log) = fake_anki(200);
        let r = send_anki(&s, &anki).unwrap();
        assert_eq!(r, SendReport { sent: 1, ..Default::default() });
        assert_eq!(log.lock().unwrap()[0]["answerCards"]["answers"], json!([{"cardId": 5, "ease": 3}]));
        assert_eq!(s.knowledge(&k).unwrap().unwrap().state, WordState::Known);
        // Nothing is sent twice.
        assert_eq!(send_anki(&s, &anki).unwrap(), SendReport::default());
        assert_eq!(log.lock().unwrap().len(), 1);
        let ov = overview(&s, "ja", DAY).unwrap();
        assert_eq!((ov.today_answers, ov.today_right, ov.today_points, ov.rescued_week, ov.waiting, ov.due_words), (1, 1, 14, 1, 0, 0));
        assert_eq!((ov.anki_log.len(), ov.anki_log[0].status.as_str()), (1, "sent"));
    }

    #[test]
    fn only_recall_without_help_goes_to_anki() {
        let (s, k) = store_with_fading_card();
        let accept = vec!["忘れた".to_string()];
        // Picked from the choices: right, but not sent, and back within three days.
        let picked = Answer { typed: None, right: Some(true), ..typed(&k, "", &accept) };
        let o = answer(&s, DAY, &picked, true).unwrap();
        assert_eq!((o.result.as_str(), o.points, o.anki), ("helped", 5, false));
        assert!(o.next_days <= 3.0, "{o:?}");
        // Typed after a hint, typed in the listening round, typed wrong: not sent either.
        s.conn.execute("DELETE FROM review_items", []).unwrap();
        let hinted = Answer { helped: true, ..typed(&k, "忘れた", &accept) };
        assert_eq!(answer(&s, DAY, &hinted, true).unwrap().result, "helped");
        let heard = Answer { mode: "listen".into(), ..typed(&k, "忘れた", &accept) };
        assert_eq!(answer(&s, DAY, &heard, true).unwrap().result, "helped");
        assert_eq!(answer(&s, DAY, &typed(&k, "覚えた", &accept), true).unwrap().result, "missed");
        // Recalled, with sending switched off.
        assert!(!answer(&s, DAY, &typed(&k, "忘れた", &accept), false).unwrap().anki);
        let n: i64 = s.conn.query_row("SELECT count(*) FROM anki_answers", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn answers_wait_for_anki_and_skip_cards_that_are_not_due() {
        let (s, k) = store_with_fading_card();
        assert!(answer(&s, DAY, &typed(&k, "忘れた", &["忘れた".to_string()]), true).unwrap().anki);
        // Anki isn't running: the answer waits.
        let r = send_anki(&s, &AnkiConnect::new("http://127.0.0.1:9")).unwrap();
        assert_eq!((r.sent, r.waiting), (0, 1));
        assert!(r.error.as_deref().is_some_and(|e| e.contains("isn't running")), "{r:?}");
        // The card was reviewed in Anki in the meantime: left alone.
        let (anki, log) = fake_anki(0);
        let r = send_anki(&s, &anki).unwrap();
        assert_eq!(r, SendReport { not_due: 1, ..Default::default() });
        assert!(log.lock().unwrap().is_empty());
        // An answer older than a day is dropped, not sent.
        s.conn.execute("UPDATE anki_answers SET status = 'pending', ts = 0", []).unwrap();
        assert_eq!(send_anki(&s, &anki).unwrap(), SendReport::default());
        let status: String = s.conn.query_row("SELECT status FROM anki_answers", [], |r| r.get(0)).unwrap();
        assert_eq!(status, "expired");
    }

    #[test]
    fn kanji_round_asks_fading_kanji_both_ways() {
        let mut s = Store::open_in_memory().unwrap();
        let note = |i: i64, kanji: &str, keyword: &str, recall: f64| KanjiNoteFacts {
            model_id: 2,
            note_id: i,
            kanji: kanji.into(),
            keyword: keyword.into(),
            story: format!("A <b>{keyword}</b> story."),
            cards: vec![review_card(i, 100, recall).1],
        };
        let notes = [note(1, "猫", "cat", 0.3), note(2, "駅", "station", 0.3), note(3, "雨", "rain", 0.95), note(4, "傘", "umbrella", 0.95)];
        crate::kanji::replace_kanji(&mut s, &notes, &KnowledgeSettings::default()).unwrap();
        let items = round(&s, None, "ja", "kanji", false, 10).unwrap();
        assert_eq!(items.len(), 2, "{items:?}");
        assert_eq!(overview(&s, "ja", DAY).unwrap().due_kanji, 2);
        for it in &items {
            let right = if it.ask == "keyword" { &it.gloss } else { &it.display };
            assert_eq!((it.options.len(), &it.options[it.option_answer]), (4, right), "{it:?}");
            assert_eq!(it.story, format!("A {} story.", it.gloss));
        }
        assert_eq!(items.iter().map(|i| i.ask.as_str()).collect::<Vec<_>>(), ["keyword", "kanji"]);
        let a = Answer { kind: "kanji".into(), mode: "kanji".into(), typed: None, right: Some(true), ..typed(&items[0].key, "", &[]) };
        let o = answer(&s, DAY, &a, true).unwrap();
        assert_eq!((o.result.as_str(), o.anki), ("helped", false));
        assert_eq!(round(&s, None, "ja", "kanji", false, 10).unwrap().len(), 1);
    }

    #[test]
    fn the_daily_goal_makes_the_streak() {
        let (s, k) = store_with_fading_card();
        s.set_setting("review_goal", "2").unwrap();
        let a = typed(&k, "忘れた", &["忘れた".to_string()]);
        for day in ["2026-11-13", "2026-11-13", "2026-11-14", "2026-11-14", "2026-11-15"] {
            answer(&s, day, &a, false).unwrap();
        }
        let ov = overview(&s, "ja", DAY).unwrap();
        assert_eq!((ov.goal, ov.today_right, ov.streak, ov.best_streak), (2, 1, 2, 2), "today's goal isn't met yet");
        answer(&s, DAY, &a, false).unwrap();
        assert_eq!(overview(&s, "ja", DAY).unwrap().streak, 3);
    }
}
