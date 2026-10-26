//! Local SQLite storage. Every row Ankura creates carries a UUID or a device ID, and exposures and
//! lookups are an append-only event log, so sync can be added later without a migration (ADR 0009).

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::knowledge::{CardFacts, KnowledgeSettings, WordState, word_state};
use crate::{Result, WordKey};

const SCHEMA_VERSION: i64 = 2;

const SCHEMA: &str = r#"
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE texts (
    id TEXT PRIMARY KEY,
    lang TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    source TEXT NOT NULL,          -- 'paste' | 'generated'
    created_at INTEGER NOT NULL,   -- ms since epoch
    opened_at INTEGER
);

-- Which field of an Anki note type holds the word (architecture-options §3).
CREATE TABLE note_type_mappings (
    model_id INTEGER PRIMARY KEY,
    model_name TEXT NOT NULL,
    target_field TEXT NOT NULL,
    reading_field TEXT,
    updated_at INTEGER NOT NULL,
    target_mode TEXT NOT NULL DEFAULT 'field'
);

-- Last snapshot read from Anki.
CREATE TABLE anki_cards (
    card_id INTEGER PRIMARY KEY,
    note_id INTEGER NOT NULL,
    model_id INTEGER NOT NULL,
    card_type INTEGER NOT NULL,
    queue INTEGER NOT NULL,
    interval INTEGER NOT NULL,
    lapses INTEGER NOT NULL,
    stability REAL,
    retrievability REAL
);
CREATE TABLE anki_card_keys (
    card_id INTEGER NOT NULL,
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,
    reading TEXT NOT NULL,
    PRIMARY KEY (card_id, lang, lemma, reading)
);
CREATE INDEX anki_card_keys_key ON anki_card_keys (lang, lemma, reading);

-- Word state derived from the snapshot.
CREATE TABLE knowledge (
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,
    reading TEXT NOT NULL,
    state TEXT NOT NULL,
    strength REAL NOT NULL,
    cards INTEGER NOT NULL,
    PRIMARY KEY (lang, lemma, reading)
);

-- The user's own marks; they win over Anki.
CREATE TABLE word_overrides (
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,
    reading TEXT NOT NULL,
    state TEXT NOT NULL,
    device_id TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (lang, lemma, reading)
);

CREATE TABLE events (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    ts INTEGER NOT NULL,
    kind TEXT NOT NULL,            -- 'exposure' | 'lookup' | 'dont_know'
    lang TEXT NOT NULL,
    lemma TEXT NOT NULL,
    reading TEXT NOT NULL,
    text_id TEXT
);
CREATE INDEX events_key ON events (lang, lemma, reading, kind);

CREATE TABLE dictionaries (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    revision TEXT NOT NULL,
    imported_at INTEGER NOT NULL
);
CREATE TABLE dict_terms (
    dict_id INTEGER NOT NULL REFERENCES dictionaries (id) ON DELETE CASCADE,
    expression TEXT NOT NULL,
    reading TEXT NOT NULL,
    tags TEXT NOT NULL,
    score INTEGER NOT NULL,
    glossary TEXT NOT NULL,        -- JSON array of plain-text senses
    sequence INTEGER NOT NULL,
    term_tags TEXT NOT NULL
);
CREATE INDEX dict_terms_expression ON dict_terms (expression);
CREATE INDEX dict_terms_reading ON dict_terms (reading);
"#;

pub(crate) fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMeta {
    pub id: String,
    pub lang: String,
    pub title: String,
    pub source: String,
    pub created_at: i64,
    pub opened_at: Option<i64>,
    pub chars: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub meta: TextMeta,
    pub body: String,
}

/// Where in the target field the word is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetMode {
    /// The whole field is the word (vocabulary cards).
    #[default]
    Field,
    /// The word is the bold, italic or underlined part of the field (sentence cards).
    Marked,
}

impl TargetMode {
    fn as_str(self) -> &'static str {
        match self {
            TargetMode::Field => "field",
            TargetMode::Marked => "marked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteTypeMapping {
    pub model_id: i64,
    pub model_name: String,
    pub target_field: String,
    pub reading_field: Option<String>,
    #[serde(default)]
    pub target_mode: TargetMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownWord {
    pub state: WordState,
    pub strength: f64,
    pub cards: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewEvent {
    pub kind: String,
    pub key: WordKey,
    pub text_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WordStats {
    pub exposures: i64,
    pub lookups: i64,
}

pub struct Store {
    pub(crate) conn: Connection,
    device_id: String,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.pragma_update(None, "journal_mode", "wal")?;
        conn.pragma_update(None, "foreign_keys", "on")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version == 0 {
            conn.execute_batch(SCHEMA)?;
        }
        if version == 1 {
            conn.execute_batch("ALTER TABLE note_type_mappings ADD COLUMN target_mode TEXT NOT NULL DEFAULT 'field';")?;
        }
        if version < SCHEMA_VERSION {
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        let device_id = match conn
            .query_row("SELECT value FROM meta WHERE key = 'device_id'", [], |r| r.get::<_, String>(0))
            .optional()?
        {
            Some(id) => id,
            None => {
                let id = Uuid::new_v4().to_string();
                conn.execute("INSERT INTO meta (key, value) VALUES ('device_id', ?1)", [&id])?;
                id
            }
        };
        Ok(Store { conn, device_id })
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    // --- settings ---

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [format!("setting:{key}")], |r| r.get(0))
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![format!("setting:{key}"), value],
        )?;
        Ok(())
    }

    // --- texts ---

    pub fn add_text(&self, lang: &str, title: &str, body: &str, source: &str) -> Result<TextMeta> {
        let id = Uuid::new_v4().to_string();
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO texts (id, lang, title, body, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, lang, title, body, source, now],
        )?;
        Ok(TextMeta {
            id,
            lang: lang.into(),
            title: title.into(),
            source: source.into(),
            created_at: now,
            opened_at: None,
            chars: body.chars().count() as i64,
        })
    }

    pub fn list_texts(&self) -> Result<Vec<TextMeta>> {
        let mut st = self.conn.prepare(
            "SELECT id, lang, title, source, created_at, opened_at, length(body) FROM texts
             ORDER BY coalesce(opened_at, created_at) DESC",
        )?;
        let rows = st.query_map([], |r| {
            Ok(TextMeta {
                id: r.get(0)?,
                lang: r.get(1)?,
                title: r.get(2)?,
                source: r.get(3)?,
                created_at: r.get(4)?,
                opened_at: r.get(5)?,
                chars: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn open_text(&self, id: &str) -> Result<Option<Text>> {
        self.conn.execute("UPDATE texts SET opened_at = ?2 WHERE id = ?1", params![id, now_ms()])?;
        Ok(self
            .conn
            .query_row(
                "SELECT id, lang, title, source, created_at, opened_at, length(body), body FROM texts WHERE id = ?1",
                [id],
                |r| {
                    Ok(Text {
                        meta: TextMeta {
                            id: r.get(0)?,
                            lang: r.get(1)?,
                            title: r.get(2)?,
                            source: r.get(3)?,
                            created_at: r.get(4)?,
                            opened_at: r.get(5)?,
                            chars: r.get(6)?,
                        },
                        body: r.get(7)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn rename_text(&self, id: &str, title: &str) -> Result<()> {
        self.conn.execute("UPDATE texts SET title = ?2 WHERE id = ?1", params![id, title])?;
        Ok(())
    }

    pub fn delete_text(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM texts WHERE id = ?1", [id])?;
        Ok(())
    }

    // --- note type mappings ---

    pub fn mappings(&self) -> Result<Vec<NoteTypeMapping>> {
        let mut st = self.conn.prepare(
            "SELECT model_id, model_name, target_field, reading_field, target_mode FROM note_type_mappings ORDER BY model_name",
        )?;
        let rows = st.query_map([], |r| {
            Ok(NoteTypeMapping {
                model_id: r.get(0)?,
                model_name: r.get(1)?,
                target_field: r.get(2)?,
                reading_field: r.get(3)?,
                target_mode: if r.get::<_, String>(4)? == "marked" { TargetMode::Marked } else { TargetMode::Field },
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn save_mapping(&self, m: &NoteTypeMapping) -> Result<()> {
        self.conn.execute(
            "INSERT INTO note_type_mappings (model_id, model_name, target_field, reading_field, updated_at, target_mode)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (model_id) DO UPDATE SET model_name = excluded.model_name,
               target_field = excluded.target_field, reading_field = excluded.reading_field,
               updated_at = excluded.updated_at, target_mode = excluded.target_mode",
            params![m.model_id, m.model_name, m.target_field, m.reading_field, now_ms(), m.target_mode.as_str()],
        )?;
        Ok(())
    }

    pub fn delete_mapping(&self, model_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM note_type_mappings WHERE model_id = ?1", [model_id])?;
        Ok(())
    }

    // --- Anki snapshot and knowledge ---

    /// Replaces the Anki snapshot and recomputes word knowledge in one transaction.
    pub fn replace_anki_snapshot(
        &mut self,
        cards: &[(i64, CardFacts)],
        card_keys: &[(i64, WordKey)],
        settings: &KnowledgeSettings,
    ) -> Result<usize> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM anki_card_keys", [])?;
        tx.execute("DELETE FROM anki_cards", [])?;
        {
            let mut ins = tx.prepare(
                "INSERT OR REPLACE INTO anki_cards (card_id, note_id, model_id, card_type, queue, interval, lapses, stability, retrievability)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for (model_id, c) in cards {
                ins.execute(params![
                    c.card_id, c.note_id, model_id, c.card_type, c.queue, c.interval, c.lapses, c.stability, c.retrievability
                ])?;
            }
            let mut ins = tx.prepare(
                "INSERT OR IGNORE INTO anki_card_keys (card_id, lang, lemma, reading) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (card_id, k) in card_keys {
                ins.execute(params![card_id, k.lang, k.lemma, k.reading])?;
            }
        }
        let by_id: HashMap<i64, &CardFacts> = cards.iter().map(|(_, c)| (c.card_id, c)).collect();
        let mut by_key: HashMap<&WordKey, Vec<&CardFacts>> = HashMap::new();
        for (card_id, k) in card_keys {
            if let Some(c) = by_id.get(card_id) {
                by_key.entry(k).or_default().push(c);
            }
        }
        tx.execute("DELETE FROM knowledge", [])?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO knowledge (lang, lemma, reading, state, strength, cards) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for (k, cs) in &by_key {
                let (state, strength) = word_state(cs.iter().copied(), settings);
                ins.execute(params![k.lang, k.lemma, k.reading, state.as_str(), strength, cs.len() as i64])?;
            }
        }
        tx.commit()?;
        Ok(by_key.len())
    }

    /// Knowledge for a key: exact (lemma, reading) first, then the strongest entry with the same
    /// lemma (a card without a reading, or a reading the tokenizer guessed differently).
    pub fn knowledge(&self, key: &WordKey) -> Result<Option<KnownWord>> {
        let row = |r: &rusqlite::Row| -> rusqlite::Result<KnownWord> {
            let s: String = r.get(0)?;
            Ok(KnownWord { state: WordState::parse(&s).unwrap_or(WordState::Unknown), strength: r.get(1)?, cards: r.get(2)? })
        };
        let exact = self
            .conn
            .prepare_cached("SELECT state, strength, cards FROM knowledge WHERE lang = ?1 AND lemma = ?2 AND reading = ?3")?
            .query_row(params![key.lang, key.lemma, key.reading], row)
            .optional()?;
        if exact.is_some() {
            return Ok(exact);
        }
        Ok(self
            .conn
            .prepare_cached(
                "SELECT state, strength, cards FROM knowledge WHERE lang = ?1 AND lemma = ?2 ORDER BY strength DESC LIMIT 1",
            )?
            .query_row(params![key.lang, key.lemma], row)
            .optional()?)
    }

    pub fn word_override(&self, key: &WordKey) -> Result<Option<WordState>> {
        let s: Option<String> = self
            .conn
            .prepare_cached("SELECT state FROM word_overrides WHERE lang = ?1 AND lemma = ?2 AND reading = ?3")?
            .query_row(params![key.lang, key.lemma, key.reading], |r| r.get(0))
            .optional()?;
        Ok(s.and_then(|s| WordState::parse(&s)))
    }

    /// `None` removes the mark.
    pub fn set_word_override(&self, key: &WordKey, state: Option<WordState>) -> Result<()> {
        match state {
            Some(s) => self.conn.execute(
                "INSERT INTO word_overrides (lang, lemma, reading, state, device_id, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (lang, lemma, reading) DO UPDATE SET state = excluded.state,
                   device_id = excluded.device_id, updated_at = excluded.updated_at",
                params![key.lang, key.lemma, key.reading, s.as_str(), self.device_id, now_ms()],
            )?,
            None => self.conn.execute(
                "DELETE FROM word_overrides WHERE lang = ?1 AND lemma = ?2 AND reading = ?3",
                params![key.lang, key.lemma, key.reading],
            )?,
        };
        Ok(())
    }

    pub fn anki_summary(&self) -> Result<HashMap<String, i64>> {
        let mut st = self.conn.prepare("SELECT state, count(*) FROM knowledge GROUP BY state")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // --- events ---

    pub fn record_events(&mut self, events: &[NewEvent]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO events (id, device_id, ts, kind, lang, lemma, reading, text_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            let now = now_ms();
            for e in events {
                ins.execute(params![
                    Uuid::new_v4().to_string(),
                    self.device_id,
                    now,
                    e.kind,
                    e.key.lang,
                    e.key.lemma,
                    e.key.reading,
                    e.text_id
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn word_stats(&self, key: &WordKey) -> Result<WordStats> {
        let mut st = self.conn.prepare_cached(
            "SELECT kind, count(*) FROM events WHERE lang = ?1 AND lemma = ?2 AND reading = ?3 GROUP BY kind",
        )?;
        let mut out = WordStats::default();
        let rows = st.query_map(params![key.lang, key.lemma, key.reading], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (kind, n) = row?;
            match kind.as_str() {
                "exposure" => out.exposures += n,
                "lookup" | "dont_know" => out.lookups += n,
                _ => {}
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(lemma: &str, reading: &str) -> WordKey {
        WordKey { lang: "ja".into(), lemma: lemma.into(), reading: reading.into() }
    }

    #[test]
    fn texts_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        let t = s.add_text("ja", "テスト", "猫が好き。", "paste").unwrap();
        assert_eq!(t.chars, 5);
        assert_eq!(s.list_texts().unwrap().len(), 1);
        let full = s.open_text(&t.id).unwrap().unwrap();
        assert_eq!(full.body, "猫が好き。");
        assert!(full.meta.opened_at.is_some());
        s.delete_text(&t.id).unwrap();
        assert!(s.list_texts().unwrap().is_empty());
    }

    #[test]
    fn device_id_is_stable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.db");
        let a = Store::open(&path).unwrap().device_id().to_string();
        let b = Store::open(&path).unwrap().device_id().to_string();
        assert_eq!(a, b);
    }

    #[test]
    fn knowledge_from_snapshot() {
        let mut s = Store::open_in_memory().unwrap();
        let card = |id, interval| CardFacts {
            card_id: id,
            note_id: id,
            card_type: 2,
            queue: 2,
            interval,
            lapses: 0,
            stability: None,
            retrievability: None,
        };
        let cards = vec![(10, card(1, 40)), (10, card(2, 3))];
        let keys = vec![(1, key("分かる", "わかる")), (2, key("人気", "ひとけ"))];
        s.replace_anki_snapshot(&cards, &keys, &KnowledgeSettings::default()).unwrap();
        assert_eq!(s.knowledge(&key("分かる", "わかる")).unwrap().unwrap().state, WordState::Known);
        // Lemma-only fallback when the reading differs.
        assert_eq!(s.knowledge(&key("人気", "にんき")).unwrap().unwrap().state, WordState::Learning);
        assert!(s.knowledge(&key("猫", "ねこ")).unwrap().is_none());
    }

    #[test]
    fn overrides_and_events() {
        let mut s = Store::open_in_memory().unwrap();
        let k = key("猫", "ねこ");
        s.set_word_override(&k, Some(WordState::MarkedKnown)).unwrap();
        assert_eq!(s.word_override(&k).unwrap(), Some(WordState::MarkedKnown));
        s.set_word_override(&k, None).unwrap();
        assert_eq!(s.word_override(&k).unwrap(), None);

        let ev = |kind: &str| NewEvent { kind: kind.into(), key: k.clone(), text_id: None };
        s.record_events(&[ev("exposure"), ev("exposure"), ev("dont_know")]).unwrap();
        let st = s.word_stats(&k).unwrap();
        assert_eq!((st.exposures, st.lookups), (2, 1));
    }

    #[test]
    fn migrates_version_1_databases() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(&SCHEMA.replace(",\n    target_mode TEXT NOT NULL DEFAULT 'field'", "")).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute(
                "INSERT INTO note_type_mappings (model_id, model_name, target_field, reading_field, updated_at) VALUES (1, 'Old', 'Word', NULL, 0)",
                [],
            )
            .unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.mappings().unwrap()[0].target_mode, TargetMode::Field);
        let v: i64 = s.conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(v, SCHEMA_VERSION);
    }

    #[test]
    fn settings_and_mappings() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.setting("anki_url").unwrap(), None);
        s.set_setting("anki_url", "http://127.0.0.1:8765").unwrap();
        assert_eq!(s.setting("anki_url").unwrap().as_deref(), Some("http://127.0.0.1:8765"));
        let m = NoteTypeMapping {
            model_id: 7,
            model_name: "Core 2k".into(),
            target_field: "Sentence".into(),
            reading_field: Some("Reading".into()),
            target_mode: TargetMode::Marked,
        };
        s.save_mapping(&m).unwrap();
        assert_eq!(s.mappings().unwrap(), vec![m]);
        s.delete_mapping(7).unwrap();
        assert!(s.mappings().unwrap().is_empty());
    }
}
