//! What the AI has produced, kept for reuse (ADR 0016): a log of every request and answer, a bank
//! of checked test questions (used or not), cached answers keyed by their input, and counters of
//! how often a stored answer saved an AI request.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde_json::Value;

use crate::Result;
use crate::llm::Llm;
use crate::store::{Store, now_ms};

pub(crate) const MEMORY_SCHEMA: &str = r#"
CREATE TABLE ai_log (
    id INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL,
    kind TEXT NOT NULL,              -- the answer schema's top-level fields, e.g. 'practice_items'
    system TEXT NOT NULL,
    prompt TEXT NOT NULL,
    response TEXT,                   -- JSON; NULL when the request failed
    error TEXT,
    ms INTEGER NOT NULL
);
CREATE TABLE item_bank (
    id TEXT PRIMARY KEY,
    test TEXT NOT NULL,              -- 'practice' | 'level_check'
    kind TEXT NOT NULL,              -- question type: a practice Mondai, or 'grammar' / 'reading'
    level INTEGER NOT NULL,          -- 5 = N5 … 1 = N1
    grp TEXT,                        -- questions on the same passage
    item TEXT NOT NULL,              -- JSON, with the key
    created_at INTEGER NOT NULL,
    uses INTEGER NOT NULL DEFAULT 0, -- tests it was put in
    last_used_at INTEGER,
    reported INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX item_bank_pick ON item_bank (test, kind, level);
CREATE TABLE ai_cache (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,              -- 'simplify' | 'card_sentence'
    key TEXT NOT NULL,
    value TEXT NOT NULL,             -- JSON
    created_at INTEGER NOT NULL
);
CREATE INDEX ai_cache_key ON ai_cache (kind, key);
CREATE TABLE ai_reuse (kind TEXT PRIMARY KEY, n INTEGER NOT NULL);
"#;

// --- the log ---

/// An `Llm` that stores every request and answer in `ai_log`. It has its own connection, so it
/// can be shared by the threads that write practice tests.
pub struct Recorded<L: Llm> {
    inner: L,
    store: Mutex<Store>,
}

impl<L: Llm> Recorded<L> {
    pub fn new(inner: L, store: Store) -> Self {
        Recorded { inner, store: Mutex::new(store) }
    }
}

fn schema_kind(schema: &Value) -> String {
    schema["properties"].as_object().map(|p| p.keys().cloned().collect::<Vec<_>>().join(",")).unwrap_or_default()
}

impl<L: Llm> Llm for Recorded<L> {
    fn json(&self, system: &str, user: &str, schema: &Value) -> Result<Value> {
        let start = Instant::now();
        let r = self.inner.json(system, user, schema);
        let (response, error) = match &r {
            Ok(v) => (Some(v.to_string()), None),
            Err(e) => (None, Some(e.to_string())),
        };
        // Logging never fails the request.
        if let Ok(store) = self.store.lock() {
            let _ = store.conn.execute(
                "INSERT INTO ai_log (created_at, kind, system, prompt, response, error, ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![now_ms(), schema_kind(schema), system, user, response, error, start.elapsed().as_millis() as i64],
            );
        }
        r
    }
}

// --- the question bank ---

#[derive(Debug, Clone)]
pub struct BankRow {
    pub id: String,
    pub group: Option<String>,
    pub item: String,
    pub uses: i64,
}

/// Adds checked questions (not yet used in a test). `rows`: (id, group, item JSON).
pub fn bank_add(store: &Store, test: &str, kind: &str, level: u8, rows: &[(String, Option<String>, String)]) -> Result<()> {
    for (id, group, item) in rows {
        store.conn.execute(
            "INSERT OR IGNORE INTO item_bank (id, test, kind, level, grp, item, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, test, kind, level, group, item, now_ms()],
        )?;
    }
    Ok(())
}

/// Questions to reuse, best first: never used, then least used and longest ago. Reported ones
/// never come back; used ones only with `repeat_seen`.
pub fn bank_candidates(store: &Store, test: &str, kind: &str, level: u8, repeat_seen: bool) -> Result<Vec<BankRow>> {
    let mut st = store.conn.prepare_cached(
        "SELECT id, grp, item, uses FROM item_bank
         WHERE test = ?1 AND kind = ?2 AND level = ?3 AND reported = 0 AND (?4 OR uses = 0)
         ORDER BY uses, coalesce(last_used_at, 0), random()",
    )?;
    let rows = st.query_map(params![test, kind, level, repeat_seen], |r| {
        Ok(BankRow { id: r.get(0)?, group: r.get(1)?, item: r.get(2)?, uses: r.get(3)? })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn bank_used(store: &Store, ids: &[String]) -> Result<()> {
    for id in ids {
        store.conn.execute("UPDATE item_bank SET uses = uses + 1, last_used_at = ?2 WHERE id = ?1", params![id, now_ms()])?;
    }
    Ok(())
}

pub fn bank_report(store: &Store, id: &str, reported: bool) -> Result<()> {
    store.conn.execute("UPDATE item_bank SET reported = ?2 WHERE id = ?1", params![id, reported as i64])?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BankCount {
    pub total: usize,
    pub unseen: usize,
}

/// Usable questions per level (5 … 1) for a test kind.
pub fn bank_counts(store: &Store, test: &str) -> Result<BTreeMap<u8, BankCount>> {
    let mut st = store.conn.prepare(
        "SELECT level, count(*), sum(uses = 0) FROM item_bank WHERE test = ?1 AND reported = 0 GROUP BY level",
    )?;
    let rows = st.query_map([test], |r| Ok((r.get::<_, u8>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (level, total, unseen) = row?;
        out.insert(level, BankCount { total: total as usize, unseen: unseen as usize });
    }
    Ok(out)
}

// --- cached answers ---

pub fn cache_get(store: &Store, kind: &str, key: &str) -> Result<Vec<Value>> {
    let mut st = store.conn.prepare_cached("SELECT value FROM ai_cache WHERE kind = ?1 AND key = ?2 ORDER BY id DESC")?;
    let rows = st.query_map([kind, key], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        if let Ok(v) = serde_json::from_str(&row?) {
            out.push(v);
        }
    }
    Ok(out)
}

/// Stores an answer unless the same one is there already.
pub fn cache_put(store: &Store, kind: &str, key: &str, value: &Value) -> Result<()> {
    let v = value.to_string();
    let exists: Option<i64> = store
        .conn
        .query_row("SELECT id FROM ai_cache WHERE kind = ?1 AND key = ?2 AND value = ?3", [kind, key, &v], |r| r.get(0))
        .optional()?;
    if exists.is_none() {
        store.conn.execute(
            "INSERT INTO ai_cache (kind, key, value, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![kind, key, v, now_ms()],
        )?;
    }
    Ok(())
}

// --- savings ---

/// Counts AI requests (or questions) a stored answer saved.
pub fn reused(store: &Store, kind: &str, n: usize) -> Result<()> {
    if n > 0 {
        store.conn.execute(
            "INSERT INTO ai_reuse (kind, n) VALUES (?1, ?2) ON CONFLICT(kind) DO UPDATE SET n = n + excluded.n",
            params![kind, n as i64],
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    /// AI requests logged, and how many failed.
    pub requests: usize,
    pub failed: usize,
    /// Test questions in the bank (not reported), and how many haven't been used yet.
    pub bank: usize,
    pub bank_unseen: usize,
    /// Cached answers (simpler sentences, card sentences).
    pub cached: usize,
    /// Reuses per kind: 'practice_items', 'level_check_items' (questions), 'simplify',
    /// 'card_sentence' (requests).
    pub reused: BTreeMap<String, usize>,
}

pub fn stats(store: &Store) -> Result<MemoryStats> {
    let c = &store.conn;
    let (requests, failed): (i64, i64) =
        c.query_row("SELECT count(*), coalesce(sum(response IS NULL), 0) FROM ai_log", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let (bank, unseen): (i64, i64) = c.query_row(
        "SELECT count(*), coalesce(sum(uses = 0), 0) FROM item_bank WHERE reported = 0",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let cached: i64 = c.query_row("SELECT count(*) FROM ai_cache", [], |r| r.get(0))?;
    let mut st = c.prepare("SELECT kind, n FROM ai_reuse")?;
    let reused = st
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as usize)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(MemoryStats {
        requests: requests as usize,
        failed: failed as usize,
        bank: bank as usize,
        bank_unseen: unseen as usize,
        cached: cached as usize,
        reused,
    })
}

/// Fills the bank from tests taken before it existed (schema v12): every practice question, and
/// the level check's grammar and reading questions, counted as used once.
pub(crate) fn backfill(conn: &rusqlite::Connection) -> Result<()> {
    let mut st = conn.prepare(
        "SELECT i.item_id, t.kind, i.section, i.level, i.item, i.reported, coalesce(i.answered_at, t.started_at)
         FROM test_items i JOIN tests t ON t.id = i.test_id
         WHERE t.kind = 'practice' OR (t.kind = 'level_check' AND i.section IN ('grammar1', 'grammar2', 'reading'))",
    )?;
    let rows: Vec<(String, String, String, u8, String, i64, i64)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, test, section, level, item, reported, at) in rows {
        let v: Value = serde_json::from_str(&item)?;
        let kind = match section.as_str() {
            "grammar1" | "grammar2" => "grammar",
            s => s,
        };
        // Level check reading questions share their passage text; use it as the group.
        let group = v["group"].as_str().map(String::from).or_else(|| v["passage"].as_str().map(|p| format!("p:{}", p.chars().take(40).collect::<String>())));
        conn.execute(
            "INSERT OR IGNORE INTO item_bank (id, test, kind, level, grp, item, created_at, uses, last_used_at, reported)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?7, ?8)",
            params![id, test, kind, level, group, item, at, reported],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct Echo;
    impl Llm for Echo {
        fn json(&self, _s: &str, user: &str, _schema: &Value) -> Result<Value> {
            if user == "fail" { Err(crate::Error::Llm("down".into())) } else { Ok(json!({"echo": user})) }
        }
    }

    #[test]
    fn every_request_is_logged() {
        let s = Store::open_in_memory().unwrap();
        let log = Store::open_in_memory().unwrap();
        let llm = Recorded::new(Echo, log);
        let schema = json!({"type": "object", "properties": {"echo": {"type": "string"}}});
        assert_eq!(llm.json("sys", "hi", &schema).unwrap(), json!({"echo": "hi"}));
        assert!(llm.json("sys", "fail", &schema).is_err());
        let log = llm.store.lock().unwrap();
        let st = stats(&log).unwrap();
        assert_eq!((st.requests, st.failed), (2, 1));
        let kind: String = log.conn.query_row("SELECT kind FROM ai_log LIMIT 1", [], |r| r.get(0)).unwrap();
        assert_eq!(kind, "echo");
        drop(s);
    }

    #[test]
    fn bank_prefers_unseen_and_skips_reported() {
        let s = Store::open_in_memory().unwrap();
        let rows: Vec<(String, Option<String>, String)> = (0..4).map(|i| (format!("q{i}"), None, format!("{{\"n\":{i}}}"))).collect();
        bank_add(&s, "practice", "context", 3, &rows).unwrap();
        bank_used(&s, &["q0".into()]).unwrap();
        bank_report(&s, "q1", true).unwrap();
        let fresh = bank_candidates(&s, "practice", "context", 3, false).unwrap();
        let mut ids: Vec<&str> = fresh.iter().map(|r| r.id.as_str()).collect();
        ids.sort();
        assert_eq!(ids, ["q2", "q3"]);
        let all = bank_candidates(&s, "practice", "context", 3, true).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all.last().unwrap().id, "q0", "seen ones come last");
        assert!(bank_candidates(&s, "practice", "context", 2, true).unwrap().is_empty());
        let c = bank_counts(&s, "practice").unwrap();
        assert_eq!((c[&3].total, c[&3].unseen), (3, 2));
    }

    #[test]
    fn upgrade_fills_the_bank_from_earlier_tests() {
        let path = std::env::temp_dir().join(format!("ankura-v11-{}.db", uuid::Uuid::new_v4()));
        {
            let s = Store::open(&path).unwrap();
            let lc = crate::proficiency::start(&s, 3).unwrap();
            let grammar = json!({"id": "g1", "section": "grammar1", "level": 3, "prompt": "文（　）", "options": ["a", "b", "c", "d"], "answer": 1});
            let vocab_id = &lc.vocab[0].id;
            s.conn.execute("INSERT INTO test_items (test_id, item_id, section, level, item) VALUES (?1, 'g1', 'grammar1', 3, ?2)", params![lc.id, grammar.to_string()]).unwrap();
            let p = crate::practice::create(&s, 4, true, false, 0, false).unwrap();
            let item = json!({"id": "p1", "mondai": "context", "level": 4, "part": 0, "prompt": "（　）", "options": ["a", "b", "c", "d"], "answer": 0});
            s.conn.execute("INSERT INTO test_items (test_id, item_id, section, level, item, reported) VALUES (?1, 'p1', 'context', 4, ?2, 1)", params![p, item.to_string()]).unwrap();
            s.conn.execute_batch("DROP TABLE ai_log; DROP TABLE item_bank; DROP TABLE ai_cache; DROP TABLE ai_reuse; PRAGMA user_version = 11;").unwrap();
            assert!(!vocab_id.is_empty());
        }
        let s = Store::open(&path).unwrap();
        let rows: Vec<(String, String, String, i64, i64)> = s
            .conn
            .prepare("SELECT id, test, kind, uses, reported FROM item_bank ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        // Only grammar/reading from the level check (no vocabulary or kanji), and practice items.
        assert_eq!(
            rows,
            vec![("g1".into(), "level_check".into(), "grammar".into(), 1, 0), ("p1".into(), "practice".into(), "context".into(), 1, 1)]
        );
        drop(s);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cache_keeps_distinct_answers_newest_first() {
        let s = Store::open_in_memory().unwrap();
        cache_put(&s, "card_sentence", "傘|かさ", &json!("a")).unwrap();
        cache_put(&s, "card_sentence", "傘|かさ", &json!("b")).unwrap();
        cache_put(&s, "card_sentence", "傘|かさ", &json!("a")).unwrap();
        assert_eq!(cache_get(&s, "card_sentence", "傘|かさ").unwrap(), vec![json!("b"), json!("a")]);
        reused(&s, "simplify", 1).unwrap();
        reused(&s, "simplify", 2).unwrap();
        assert_eq!(stats(&s).unwrap().reused["simplify"], 3);
    }
}
