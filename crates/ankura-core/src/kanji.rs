//! Kanji (ADR 0013): the user's kanji note types (RTK style: kanji, keyword, mnemonic story),
//! what the reader's kanji pop-up shows, new mnemonic stories, and the one kind of edit Ankura
//! makes to existing notes: the mapped story field, confirmed, backed up and undoable.

use std::collections::{BTreeMap, HashMap};

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::anki::AnkiSource;
use crate::kana::is_kanji;
use crate::knowledge::{CardFacts, KnowledgeSettings, WordState, word_state};
use crate::llm::Llm;
use crate::normalize::plain_text;
use crate::store::{Store, now_ms};
use crate::{Error, Result};

/// Tag on every note whose story Ankura changed.
pub const EDIT_TAG: &str = "ankura-edited";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KanjiMapping {
    pub model_id: i64,
    pub model_name: String,
    pub kanji_field: String,
    pub keyword_field: Option<String>,
    /// The mnemonic: the only field Ankura may change.
    pub story_field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KanjiNote {
    pub note_id: i64,
    pub model_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kanji {
    pub kanji: String,
    pub keyword: String,
    /// The story as stored in Anki (HTML).
    pub story: String,
    pub state: WordState,
    pub strength: f64,
    pub notes: Vec<KanjiNote>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KanjiWord {
    pub display: String,
    pub reading: String,
    pub state: WordState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanjiEdit {
    pub id: String,
    pub kanji: String,
    pub note_id: i64,
    pub field: String,
    pub old: String,
    pub new: String,
    pub created_at: i64,
    pub undone_at: Option<i64>,
}

/// What the kanji pop-up shows.
#[derive(Debug, Clone, Serialize)]
pub struct KanjiCard {
    pub kanji: String,
    /// `None` when no mapped kanji note has this kanji.
    pub info: Option<Kanji>,
    /// The story as plain text, for showing.
    pub story_text: String,
    pub words: Vec<KanjiWord>,
    /// The latest edit Ankura made to this kanji's story that hasn't been undone.
    pub last_edit: Option<KanjiEdit>,
}

pub(crate) const KANJI_SCHEMA: &str = r#"
CREATE TABLE kanji_mappings (
    model_id INTEGER PRIMARY KEY,
    model_name TEXT NOT NULL,
    kanji_field TEXT NOT NULL,
    keyword_field TEXT,
    story_field TEXT,
    updated_at INTEGER NOT NULL
);
CREATE TABLE kanji (
    kanji TEXT PRIMARY KEY,
    keyword TEXT NOT NULL,
    story TEXT NOT NULL,
    state TEXT NOT NULL,
    strength REAL NOT NULL,
    notes TEXT NOT NULL            -- JSON [{note_id, model_id}]
);
-- Every change Ankura made to a note (ADR 0013), so it can be undone.
CREATE TABLE kanji_edits (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    kanji TEXT NOT NULL,
    note_id INTEGER NOT NULL,
    field TEXT NOT NULL,
    old TEXT NOT NULL,
    new TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    undone_at INTEGER
);
"#;

// --- mappings ---

pub fn mappings(store: &Store) -> Result<Vec<KanjiMapping>> {
    let mut st = store
        .conn
        .prepare("SELECT model_id, model_name, kanji_field, keyword_field, story_field FROM kanji_mappings ORDER BY model_name")?;
    let rows = st.query_map([], |r| {
        Ok(KanjiMapping {
            model_id: r.get(0)?,
            model_name: r.get(1)?,
            kanji_field: r.get(2)?,
            keyword_field: r.get(3)?,
            story_field: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn save_mapping(store: &Store, m: &KanjiMapping) -> Result<()> {
    store.conn.execute(
        "INSERT INTO kanji_mappings (model_id, model_name, kanji_field, keyword_field, story_field, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (model_id) DO UPDATE SET model_name = excluded.model_name, kanji_field = excluded.kanji_field,
             keyword_field = excluded.keyword_field, story_field = excluded.story_field, updated_at = excluded.updated_at",
        params![m.model_id, m.model_name, m.kanji_field, m.keyword_field, m.story_field, now_ms()],
    )?;
    Ok(())
}

pub fn delete_mapping(store: &Store, model_id: i64) -> Result<()> {
    store.conn.execute("DELETE FROM kanji_mappings WHERE model_id = ?1", [model_id])?;
    Ok(())
}

/// The kanji of a field: its only kanji character (fields like "猫", "<div>猫</div>", "猫 cat").
pub fn kanji_of(raw: &str) -> Option<String> {
    plain_text(raw).chars().find(|c| is_kanji(*c)).map(String::from)
}

/// Proposes fields from the field names and a few notes: the kanji field holds one kanji, the
/// story field is the longest text, the keyword field a short one.
pub fn suggest_mapping(model_id: i64, model_name: &str, fields: &[String], samples: &[BTreeMap<String, String>]) -> Option<KanjiMapping> {
    let share = |f: &String, test: &dyn Fn(&str) -> bool| {
        let n = samples.iter().filter(|s| s.get(f).is_some_and(|v| test(v))).count();
        n as f64 / samples.len().max(1) as f64
    };
    let one_kanji = |v: &str| {
        let t = plain_text(v);
        t.chars().filter(|c| is_kanji(*c)).count() == 1 && t.chars().count() <= 3
    };
    let kanji_field = fields.iter().find(|f| share(f, &one_kanji) >= 0.8)?.clone();
    let avg_len = |f: &String| {
        samples.iter().map(|s| s.get(f).map(|v| plain_text(v).chars().count()).unwrap_or(0)).sum::<usize>() as f64
            / samples.len().max(1) as f64
    };
    let named = |words: &[&str]| fields.iter().find(|f| words.iter().any(|w| f.to_lowercase().contains(w))).cloned();
    let others: Vec<&String> = fields.iter().filter(|f| **f != kanji_field).collect();
    let story_field = named(&["story", "mnemonic", "geschichte", "eselsbr", "merk"])
        .filter(|f| *f != kanji_field)
        .or_else(|| others.iter().max_by(|a, b| avg_len(a).total_cmp(&avg_len(b))).filter(|f| avg_len(f) > 20.0).map(|f| (*f).clone()));
    let keyword_field = named(&["keyword", "schlüsselwort", "meaning", "bedeutung", "english", "deutsch"])
        .filter(|f| *f != kanji_field && Some(f) != story_field.as_ref())
        .or_else(|| {
            others
                .iter()
                .filter(|f| Some(**f) != story_field.as_ref())
                .find(|f| share(f, &|v: &str| (1..=30).contains(&plain_text(v).chars().count())) >= 0.8)
                .map(|f| (*f).clone())
        });
    Some(KanjiMapping { model_id, model_name: model_name.to_string(), kanji_field, keyword_field, story_field })
}

// --- the kanji table (filled by `mapping::sync_anki`) ---

/// One kanji note as sync reads it.
pub struct KanjiNoteFacts {
    pub model_id: i64,
    pub note_id: i64,
    pub kanji: String,
    pub keyword: String,
    pub story: String,
    pub cards: Vec<CardFacts>,
}

/// Replaces the kanji table. A kanji in several notes (Kanji and Kanji reverse) is one kanji: the
/// strongest card decides its state; keyword and story come from the first note that has them.
pub fn replace_kanji(store: &mut Store, notes: &[KanjiNoteFacts], settings: &KnowledgeSettings) -> Result<usize> {
    let mut by_kanji: BTreeMap<&str, Vec<&KanjiNoteFacts>> = BTreeMap::new();
    for n in notes {
        by_kanji.entry(n.kanji.as_str()).or_default().push(n);
    }
    let tx = store.conn.transaction()?;
    tx.execute("DELETE FROM kanji", [])?;
    {
        let mut ins = tx.prepare(
            "INSERT INTO kanji (kanji, keyword, story, state, strength, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for (k, ns) in &by_kanji {
            let (state, strength) = word_state(ns.iter().flat_map(|n| n.cards.iter()), settings);
            let keyword = ns.iter().map(|n| n.keyword.as_str()).find(|s| !s.is_empty()).unwrap_or("");
            let story = ns.iter().map(|n| n.story.as_str()).find(|s| !s.trim().is_empty()).unwrap_or("");
            let refs: Vec<KanjiNote> = ns.iter().map(|n| KanjiNote { note_id: n.note_id, model_id: n.model_id }).collect();
            ins.execute(params![k, keyword, story, state.as_str(), strength, serde_json::to_string(&refs)?])?;
        }
    }
    tx.commit()?;
    Ok(by_kanji.len())
}

fn kanji_row(r: &rusqlite::Row) -> rusqlite::Result<Kanji> {
    let state: String = r.get(3)?;
    let notes: String = r.get(5)?;
    Ok(Kanji {
        kanji: r.get(0)?,
        keyword: r.get(1)?,
        story: r.get(2)?,
        state: WordState::parse(&state).unwrap_or(WordState::Unknown),
        strength: r.get(4)?,
        notes: serde_json::from_str(&notes).unwrap_or_default(),
    })
}

pub fn get(store: &Store, kanji: &str) -> Result<Option<Kanji>> {
    Ok(store
        .conn
        .query_row("SELECT kanji, keyword, story, state, strength, notes FROM kanji WHERE kanji = ?1", [kanji], kanji_row)
        .optional()?)
}

fn last_edit(store: &Store, kanji: &str) -> Result<Option<KanjiEdit>> {
    Ok(store
        .conn
        .query_row(
            "SELECT id, kanji, note_id, field, old, new, created_at, undone_at FROM kanji_edits
             WHERE kanji = ?1 AND undone_at IS NULL ORDER BY created_at DESC LIMIT 1",
            [kanji],
            |r| {
                Ok(KanjiEdit {
                    id: r.get(0)?,
                    kanji: r.get(1)?,
                    note_id: r.get(2)?,
                    field: r.get(3)?,
                    old: r.get(4)?,
                    new: r.get(5)?,
                    created_at: r.get(6)?,
                    undone_at: r.get(7)?,
                })
            },
        )
        .optional()?)
}

/// The kanji pop-up: the user's kanji card and words in their collection that use the kanji.
pub fn card(store: &Store, kanji: &str) -> Result<KanjiCard> {
    let info = get(store, kanji)?;
    let mut st = store.conn.prepare(
        "SELECT coalesce(display, lemma), reading, state FROM knowledge
         WHERE lang = 'ja' AND (lemma LIKE ?1 OR display LIKE ?1) AND state != 'unknown'
         ORDER BY strength DESC LIMIT 24",
    )?;
    let words = st
        .query_map([format!("%{kanji}%")], |r| {
            let s: String = r.get(2)?;
            Ok(KanjiWord { display: r.get(0)?, reading: r.get(1)?, state: WordState::parse(&s).unwrap_or(WordState::Unknown) })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(KanjiCard {
        kanji: kanji.to_string(),
        story_text: info.as_ref().map(|k| plain_text(&k.story)).unwrap_or_default(),
        info,
        words,
        last_edit: last_edit(store, kanji)?,
    })
}

// --- new mnemonic stories ---

const STORY_SYSTEM: &str = "You write mnemonic stories for kanji in the style of Heisig's Remembering the Kanji \
(RTK). A story ties the kanji's keyword to the primitives (components) the kanji is built from, by their names, in \
a short, vivid, memorable image. Use the primitive names the learner uses in their other stories (examples below), \
and RTK's names for primitives they don't name. Where a component has several readings as a primitive (月 as moon \
or as flesh / part of the body), use the one that fits this kanji. Write in the same language, length and \
formatting (HTML such as <b> around the keyword, <br>) as the learner's example stories. The learner found their \
current story hard to remember, so make the new one clearly different and more concrete.";

fn story_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "story": {"type": "string", "description": "The new story, formatted like the examples"},
            "primitives": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {"component": {"type": "string"}, "name": {"type": "string"}},
                    "required": ["component", "name"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["story", "primitives"],
        "additionalProperties": false
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Primitive {
    pub component: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewStory {
    pub story: String,
    pub primitives: Vec<Primitive>,
}

const STORY_EXAMPLES: usize = 6;

/// A new RTK-style story for a kanji, in the style of the user's other stories.
pub fn new_story(store: &Store, llm: &dyn Llm, kanji: &str) -> Result<NewStory> {
    let k = get(store, kanji)?.ok_or_else(|| Error::Anki(format!("{kanji} isn't in your mapped kanji notes")))?;
    let mut st = store.conn.prepare(
        "SELECT kanji, keyword, story FROM kanji WHERE kanji != ?1 AND story != '' ORDER BY strength DESC LIMIT ?2",
    )?;
    let examples = st
        .query_map(params![kanji, STORY_EXAMPLES as i64], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut prompt = String::from("The learner's stories for other kanji (keep to their style and primitive names):\n");
    for (ek, kw, s) in &examples {
        prompt.push_str(&format!("- {ek} ({kw}): {s}\n"));
    }
    prompt.push_str(&format!("\nKanji: {kanji}\nKeyword: {}\nCurrent story (hard to remember): {}\n", plain_text(&k.keyword), k.story));
    let v = llm.json(STORY_SYSTEM, &prompt, &story_schema())?;
    serde_json::from_value(v).map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))
}

// --- edits: the story field only, confirmed by the user, backed up, undoable ---

/// The story field of a note, checked against the kanji mappings (ADR 0013): `None` unless the
/// note's type is a mapped kanji type with a story field.
fn story_field_of(store: &Store, anki: &dyn AnkiSource, note_id: i64) -> Result<Option<(String, String)>> {
    let maps = mappings(store)?;
    let Some(n) = anki.notes_info(&[note_id])?.into_iter().next() else { return Ok(None) };
    let Some(m) = maps.iter().find(|m| m.model_name == n.model_name) else { return Ok(None) };
    let Some(field) = m.story_field.clone() else { return Ok(None) };
    let current = n.fields.get(&field).cloned().unwrap_or_default();
    Ok(Some((field, current)))
}

/// Writes a story the user confirmed to every note of the kanji, after saving the old value.
pub fn save_story(store: &Store, anki: &dyn AnkiSource, kanji: &str, story: &str) -> Result<usize> {
    let k = get(store, kanji)?.ok_or_else(|| Error::Anki(format!("{kanji} isn't in your mapped kanji notes")))?;
    let mut written = 0;
    for n in &k.notes {
        let Some((field, old)) = story_field_of(store, anki, n.note_id)? else { continue };
        if old == story {
            continue;
        }
        store.conn.execute(
            "INSERT INTO kanji_edits (id, device_id, kanji, note_id, field, old, new, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![Uuid::new_v4().to_string(), store.device_id(), kanji, n.note_id, field, old, story, now_ms()],
        )?;
        anki.update_note_field(n.note_id, &field, story)?;
        anki.add_tag(&[n.note_id], EDIT_TAG)?;
        written += 1;
    }
    if written == 0 {
        return Err(Error::Anki("no kanji note with a mnemonic field to change; check the kanji mapping".into()));
    }
    store.conn.execute("UPDATE kanji SET story = ?2 WHERE kanji = ?1", params![kanji, story])?;
    Ok(written)
}

/// Undoes Ankura's latest story change for a kanji, on every note it changed, where the field
/// still holds Ankura's text (a story edited in Anki since is left alone).
pub fn undo_story(store: &Store, anki: &dyn AnkiSource, kanji: &str) -> Result<usize> {
    let Some(latest) = last_edit(store, kanji)? else { return Err(Error::Anki("nothing to undo".into())) };
    let mut st = store.conn.prepare(
        "SELECT id, note_id, field, old, new FROM kanji_edits WHERE kanji = ?1 AND undone_at IS NULL AND new = ?2",
    )?;
    let edits = st
        .query_map(params![kanji, latest.new], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut undone = 0;
    let mut kept = 0;
    for (id, note_id, field, old, new) in edits {
        match story_field_of(store, anki, note_id)? {
            Some((f, current)) if f == field && current == new => {
                anki.update_note_field(note_id, &field, &old)?;
                store.conn.execute("UPDATE kanji_edits SET undone_at = ?2 WHERE id = ?1", params![id, now_ms()])?;
                store.conn.execute("UPDATE kanji SET story = ?2 WHERE kanji = ?1", params![kanji, old])?;
                undone += 1;
            }
            _ => kept += 1,
        }
    }
    if undone == 0 && kept > 0 {
        return Err(Error::Anki("the story was changed in Anki since; nothing undone".into()));
    }
    Ok(undone)
}

/// Kanji count per state, for the sync report.
pub fn summary(store: &Store) -> Result<HashMap<String, i64>> {
    let mut st = store.conn.prepare("SELECT state, count(*) FROM kanji GROUP BY state")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::anki::AnkiConnect;
    use crate::anki::fake::{FakeCard, FakeNote, serve_logged};

    struct OneAnswer(Mutex<Vec<(String, Value)>>, Value);
    impl Llm for OneAnswer {
        fn json(&self, _s: &str, user: &str, schema: &Value) -> Result<Value> {
            self.0.lock().unwrap().push((user.to_string(), schema.clone()));
            Ok(self.1.clone())
        }
    }

    fn fields(names: &[&str], values: &[&str]) -> BTreeMap<String, String> {
        names.iter().zip(values).map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn suggests_kanji_fields() {
        let names: Vec<String> = ["Kanji", "Keyword", "Story", "Strokes"].iter().map(|s| s.to_string()).collect();
        let samples = vec![
            fields(&["Kanji", "Keyword", "Story", "Strokes"], &["<div>明</div>", "bright", "The <b>sun</b> and the moon together make it bright.", "8"]),
            fields(&["Kanji", "Keyword", "Story", "Strokes"], &["猫", "cat", "A cat in the rice field, a beast with grass on its head.", "11"]),
        ];
        let m = suggest_mapping(5, "Kanji", &names, &samples).unwrap();
        assert_eq!((m.kanji_field.as_str(), m.keyword_field.as_deref(), m.story_field.as_deref()), ("Kanji", Some("Keyword"), Some("Story")));
        // German names and no telling names: content decides.
        let names: Vec<String> = ["Vorderseite", "Rückseite", "Notiz"].iter().map(|s| s.to_string()).collect();
        let samples = vec![
            fields(&["Vorderseite", "Rückseite", "Notiz"], &["明", "hell", "Sonne und Mond zusammen ergeben etwas Helles am Himmel."]),
            fields(&["Vorderseite", "Rückseite", "Notiz"], &["猫", "Katze", "Ein Tier mit Gras auf dem Kopf jagt im Reisfeld."]),
        ];
        let m = suggest_mapping(6, "Kanji reverse", &names, &samples).unwrap();
        assert_eq!((m.kanji_field.as_str(), m.keyword_field.as_deref(), m.story_field.as_deref()), ("Vorderseite", Some("Rückseite"), Some("Notiz")));
        assert_eq!(kanji_of("<b>猫</b>&nbsp;"), Some("猫".into()));
    }

    fn setup() -> (Store, AnkiConnect, std::sync::Arc<Mutex<Vec<Value>>>) {
        let card = |id| vec![FakeCard { id, interval: 30, flags: &["review"], stability: None, retrievability: None, overdue: 0 }];
        let (url, log) = serve_logged(vec![
            FakeNote { id: 10, model: ("Kanji", 5), fields: vec![("Kanji", "明"), ("Keyword", "bright"), ("Story", "Sun and moon: <b>bright</b>.")], cards: card(1) },
            FakeNote { id: 11, model: ("Kanji reverse", 6), fields: vec![("Kanji", "明"), ("Keyword", "bright"), ("Story", "Sun and moon: <b>bright</b>.")], cards: card(2) },
            FakeNote { id: 12, model: ("Kanji", 5), fields: vec![("Kanji", "猫"), ("Keyword", "cat"), ("Story", "A beast in the <b>cat</b> field.")], cards: card(3) },
        ]);
        let mut s = Store::open_in_memory().unwrap();
        for (id, name) in [(5, "Kanji"), (6, "Kanji reverse")] {
            save_mapping(&s, &KanjiMapping {
                model_id: id,
                model_name: name.into(),
                kanji_field: "Kanji".into(),
                keyword_field: Some("Keyword".into()),
                story_field: Some("Story".into()),
            })
            .unwrap();
        }
        let facts = |card_id, note_id| CardFacts { card_id, note_id, card_type: 2, queue: 2, interval: 30, lapses: 0, stability: None, retrievability: None };
        let notes = vec![
            KanjiNoteFacts { model_id: 5, note_id: 10, kanji: "明".into(), keyword: "bright".into(), story: "Sun and moon: <b>bright</b>.".into(), cards: vec![facts(1, 10)] },
            KanjiNoteFacts { model_id: 6, note_id: 11, kanji: "明".into(), keyword: "bright".into(), story: "Sun and moon: <b>bright</b>.".into(), cards: vec![facts(2, 11)] },
            KanjiNoteFacts { model_id: 5, note_id: 12, kanji: "猫".into(), keyword: "cat".into(), story: "A beast in the <b>cat</b> field.".into(), cards: vec![facts(3, 12)] },
        ];
        assert_eq!(replace_kanji(&mut s, &notes, &KnowledgeSettings::default()).unwrap(), 2);
        (s, AnkiConnect::new(&url), log)
    }

    #[test]
    fn new_story_saved_to_both_notes_and_undone() {
        let (s, anki, log) = setup();
        let c = card(&s, "明").unwrap();
        assert_eq!(c.info.as_ref().unwrap().notes.len(), 2);
        assert_eq!((c.info.as_ref().unwrap().keyword.as_str(), c.story_text.as_str()), ("bright", "Sun and moon: bright."));
        assert_eq!(c.info.unwrap().state, WordState::Known);

        let llm = OneAnswer(Mutex::new(vec![]), json!({"story": "The <b>bright</b> sun beside the moon.", "primitives": [{"component": "日", "name": "sun"}, {"component": "月", "name": "moon"}]}));
        let story = new_story(&s, &llm, "明").unwrap();
        assert_eq!(story.primitives[1].name, "moon");
        let (prompt, _) = &llm.0.lock().unwrap()[0];
        assert!(prompt.contains("猫 (cat): A beast") && prompt.contains("Keyword: bright"), "{prompt}");

        assert_eq!(save_story(&s, &anki, "明", &story.story).unwrap(), 2);
        let edit = card(&s, "明").unwrap().last_edit.unwrap();
        assert_eq!((edit.old.as_str(), edit.new.as_str()), ("Sun and moon: <b>bright</b>.", story.story.as_str()));
        assert_eq!(anki.notes_info(&[11]).unwrap()[0].fields["Story"], story.story);
        {
            let l = log.lock().unwrap();
            let updates: Vec<_> = l.iter().filter(|e| e.get("updateNoteFields").is_some()).collect();
            assert_eq!(updates.len(), 2);
            assert!(updates.iter().all(|u| u["updateNoteFields"]["fields"].as_object().unwrap().keys().eq(["Story"])), "only the story field");
            assert!(l.iter().any(|e| e["addTags"]["tags"] == EDIT_TAG));
        }

        assert_eq!(undo_story(&s, &anki, "明").unwrap(), 2);
        assert_eq!(anki.notes_info(&[10]).unwrap()[0].fields["Story"], "Sun and moon: <b>bright</b>.");
        assert!(card(&s, "明").unwrap().last_edit.is_none());
        assert!(undo_story(&s, &anki, "明").is_err());
    }

    #[test]
    fn undo_leaves_a_story_changed_in_anki_alone() {
        let (s, anki, _) = setup();
        save_story(&s, &anki, "猫", "New cat story.").unwrap();
        anki.update_note_field(12, "Story", "My own fix.").unwrap(); // the user edits it in Anki
        assert!(undo_story(&s, &anki, "猫").unwrap_err().to_string().contains("changed in Anki"));
        assert_eq!(anki.notes_info(&[12]).unwrap()[0].fields["Story"], "My own fix.");
    }
}
