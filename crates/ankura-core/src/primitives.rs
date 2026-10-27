//! Consistent primitive names across a kanji set (ADR 0013 point 5). The AI reads every story and
//! records which primitives it uses under which names. Ankura lists, per component, the names in
//! use and where they clash. The user approves one name per component (two where it really is two
//! things, like 月 moon / flesh). Stories that use other names are then rewritten, names only, and
//! saved through `kanji::save_story` (confirmed, backed up, undoable).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::kanji;
use crate::llm::Llm;
use crate::normalize::plain_text;
use crate::store::{Store, now_ms};
use crate::{Error, Result};

pub(crate) const PRIMITIVES_SCHEMA: &str = r#"
-- What the AI found in each story: the primitives it uses and the names it gives them.
CREATE TABLE kanji_primitives (
    kanji TEXT NOT NULL,
    component TEXT NOT NULL,
    name TEXT NOT NULL,
    rtk_name TEXT NOT NULL,
    PRIMARY KEY (kanji, component, name)
);
-- The story each kanji was analysed with (a changed story is analysed again).
CREATE TABLE kanji_analysed (
    kanji TEXT PRIMARY KEY,
    story TEXT NOT NULL,
    analysed_at INTEGER NOT NULL
);
-- The names the user approved, per component (more than one where the component is two things).
CREATE TABLE primitive_names (
    component TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (component, name)
);
"#;

const BATCH: usize = 40;

const ANALYSE_SYSTEM: &str = "You analyse kanji mnemonic stories written in the style of Heisig's Remembering the \
Kanji (RTK). For each kanji, list the primitives (components) its story uses and the name the story gives each one, \
as a short base form (singular, no article; in the story's language). Give the component as a character: the \
kanji or radical form that the primitive stands for (for example 氵 for water as a left-hand radical, 月 for moon \
or flesh). List only primitives the story actually names; leave out the keyword itself. Also give RTK's usual \
name for each component (in English).";

fn analyse_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kanji": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "kanji": {"type": "string"},
                        "primitives": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "component": {"type": "string"},
                                    "name": {"type": "string"},
                                    "rtk_name": {"type": "string"}
                                },
                                "required": ["component", "name", "rtk_name"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["kanji", "primitives"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["kanji"],
        "additionalProperties": false
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalysisStatus {
    /// Kanji with a story.
    pub with_story: usize,
    /// Of those, analysed with their current story.
    pub analysed: usize,
}

/// Kanji whose current story hasn't been analysed yet, with keyword and story.
fn pending(store: &Store) -> Result<Vec<(String, String, String)>> {
    let mut st = store.conn.prepare(
        "SELECT k.kanji, k.keyword, k.story FROM kanji k
         LEFT JOIN kanji_analysed a ON a.kanji = k.kanji
         WHERE trim(k.story) != '' AND (a.story IS NULL OR a.story != k.story)
         ORDER BY k.kanji",
    )?;
    let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn status(store: &Store) -> Result<AnalysisStatus> {
    let with_story: i64 = store.conn.query_row("SELECT count(*) FROM kanji WHERE trim(story) != ''", [], |r| r.get(0))?;
    let todo = pending(store)?.len();
    Ok(AnalysisStatus { with_story: with_story as usize, analysed: with_story as usize - todo })
}

/// Analyses the stories not analysed yet, in batches; `progress(done, total)` after each batch.
/// Stops after the batch in progress when `stop()` returns true; run again to continue.
pub fn analyse(
    store: &Store,
    llm: &dyn Llm,
    mut progress: impl FnMut(usize, usize),
    stop: impl Fn() -> bool,
) -> Result<AnalysisStatus> {
    let todo = pending(store)?;
    let total = todo.len();
    let mut done = 0;
    for batch in todo.chunks(BATCH) {
        if stop() {
            break;
        }
        let mut prompt = String::from("Stories (kanji | keyword | story):\n");
        for (k, kw, story) in batch {
            prompt.push_str(&format!("{k} | {} | {}\n", plain_text(kw), plain_text(story).replace('\n', " ")));
        }
        let v = llm.json(ANALYSE_SYSTEM, &prompt, &analyse_schema())?;
        #[derive(Deserialize)]
        struct Prim {
            component: String,
            name: String,
            rtk_name: String,
        }
        #[derive(Deserialize)]
        struct Item {
            kanji: String,
            primitives: Vec<Prim>,
        }
        #[derive(Deserialize)]
        struct Answer {
            kanji: Vec<Item>,
        }
        let a: Answer = serde_json::from_value(v).map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?;
        let by_kanji: HashMap<&str, &Item> = a.kanji.iter().map(|i| (i.kanji.as_str(), i)).collect();
        let tx = store.conn.unchecked_transaction()?;
        for (k, _, story) in batch {
            tx.execute("DELETE FROM kanji_primitives WHERE kanji = ?1", [k])?;
            if let Some(item) = by_kanji.get(k.as_str()) {
                for p in &item.primitives {
                    let (c, n) = (p.component.trim(), p.name.trim().to_lowercase());
                    if c.is_empty() || n.is_empty() {
                        continue;
                    }
                    tx.execute(
                        "INSERT OR IGNORE INTO kanji_primitives (kanji, component, name, rtk_name) VALUES (?1, ?2, ?3, ?4)",
                        params![k, c, n, p.rtk_name.trim().to_lowercase()],
                    )?;
                }
            }
            // A kanji the model skipped counts as analysed too (with nothing found), so a bad batch
            // doesn't loop forever; "Analyse again" redoes everything.
            tx.execute(
                "INSERT INTO kanji_analysed (kanji, story, analysed_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (kanji) DO UPDATE SET story = excluded.story, analysed_at = excluded.analysed_at",
                params![k, story, now_ms()],
            )?;
        }
        tx.commit()?;
        done += batch.len();
        progress(done, total);
    }
    status(store)
}

/// Forgets the analysis, so the next `analyse` reads every story again.
pub fn reset_analysis(store: &Store) -> Result<()> {
    store.conn.execute_batch("DELETE FROM kanji_primitives; DELETE FROM kanji_analysed;")?;
    Ok(())
}

// --- the list ---

#[derive(Debug, Clone, Serialize)]
pub struct NameUse {
    pub name: String,
    pub count: usize,
    /// Up to 12 kanji whose stories use this name.
    pub kanji: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrimitiveRow {
    pub component: String,
    /// Names in use, most used first.
    pub names: Vec<NameUse>,
    /// RTK's name, as the model gave it most often.
    pub rtk_name: String,
    /// The approved names, or (before approval) the proposal: the most used name, or RTK's.
    pub approved: Vec<String>,
    pub is_approved: bool,
    /// More than one name in use.
    pub clash: bool,
}

pub fn list(store: &Store) -> Result<Vec<PrimitiveRow>> {
    let mut uses: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    let mut rtk: HashMap<String, HashMap<String, usize>> = HashMap::new();
    {
        let mut st = store.conn.prepare("SELECT component, name, kanji, rtk_name FROM kanji_primitives ORDER BY kanji")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)))?;
        for row in rows {
            let (c, n, k, r) = row?;
            uses.entry(c.clone()).or_default().entry(n).or_default().push(k);
            if !r.is_empty() {
                *rtk.entry(c).or_default().entry(r).or_default() += 1;
            }
        }
    }
    let approved = approved_names(store)?;
    let mut out: Vec<PrimitiveRow> = uses
        .into_iter()
        .map(|(component, names)| {
            let mut names: Vec<NameUse> = names
                .into_iter()
                .map(|(name, kanji)| NameUse { name, count: kanji.len(), kanji: kanji.into_iter().take(12).collect() })
                .collect();
            names.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
            let rtk_name = rtk
                .get(&component)
                .and_then(|m| m.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(n, _)| n.clone()))
                .unwrap_or_default();
            let (approved, is_approved) = match approved.get(&component) {
                Some(a) => (a.iter().cloned().collect(), true),
                None => (names.first().map(|n| vec![n.name.clone()]).unwrap_or_else(|| vec![rtk_name.clone()]), false),
            };
            PrimitiveRow { clash: names.len() > 1, component, names, rtk_name, approved, is_approved }
        })
        .collect();
    // Clashes first, then the most used components.
    out.sort_by(|a, b| {
        b.clash.cmp(&a.clash).then_with(|| {
            let n = |r: &PrimitiveRow| r.names.iter().map(|n| n.count).sum::<usize>();
            n(b).cmp(&n(a))
        })
    });
    Ok(out)
}

pub fn approved_names(store: &Store) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut st = store.conn.prepare("SELECT component, name FROM primitive_names")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in rows {
        let (c, n) = row?;
        out.entry(c).or_default().insert(n);
    }
    Ok(out)
}

/// Saves the approved list (replaces it): component → names.
pub fn approve(store: &Store, names: &BTreeMap<String, Vec<String>>) -> Result<()> {
    let tx = store.conn.unchecked_transaction()?;
    tx.execute("DELETE FROM primitive_names", [])?;
    for (c, ns) in names {
        for n in ns.iter().map(|n| n.trim().to_lowercase()).filter(|n| !n.is_empty()) {
            tx.execute("INSERT OR IGNORE INTO primitive_names (component, name) VALUES (?1, ?2)", params![c, n])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// The approved names as a prompt line for new stories ("日 sun; 月 moon / flesh; …").
pub fn approved_for_prompt(store: &Store) -> Result<String> {
    Ok(approved_names(store)?
        .iter()
        .map(|(c, ns)| format!("{c} {}", ns.iter().cloned().collect::<Vec<_>>().join(" / ")))
        .collect::<Vec<_>>()
        .join("; "))
}

// --- stories to update ---

#[derive(Debug, Clone, Serialize)]
pub struct Replacement {
    pub component: String,
    pub old: String,
    /// The approved names for the component (the model picks the one that fits).
    pub new: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub kanji: String,
    pub keyword: String,
    pub story: String,
    pub replacements: Vec<Replacement>,
}

/// Kanji whose stories use a name that isn't approved for an approved component.
pub fn candidates(store: &Store) -> Result<Vec<Candidate>> {
    let approved = approved_names(store)?;
    let mut by_kanji: BTreeMap<String, Vec<Replacement>> = BTreeMap::new();
    {
        let mut st = store.conn.prepare(
            "SELECT p.kanji, p.component, p.name FROM kanji_primitives p
             JOIN kanji_analysed a ON a.kanji = p.kanji JOIN kanji k ON k.kanji = p.kanji AND k.story = a.story",
        )?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
        for row in rows {
            let (k, c, n) = row?;
            if let Some(ok) = approved.get(&c) {
                if !ok.contains(&n) {
                    by_kanji.entry(k).or_default().push(Replacement { component: c, old: n, new: ok.iter().cloned().collect() });
                }
            }
        }
    }
    let mut out = Vec::new();
    for (k, replacements) in by_kanji {
        if let Some(info) = kanji::get(store, &k)? {
            out.push(Candidate { kanji: k, keyword: plain_text(&info.keyword), story: info.story, replacements });
        }
    }
    Ok(out)
}

const REWRITE_SYSTEM: &str = "You edit kanji mnemonic stories (Remembering the Kanji style). In each story, \
replace the old names of the listed primitives with the new names. Where a component has two new names, use the \
one that fits this kanji. Change only what the new names need (articles, grammar around them); keep everything \
else, including the language, the HTML formatting and the keyword, exactly as it is.";

fn rewrite_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "stories": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {"kanji": {"type": "string"}, "story": {"type": "string"}},
                    "required": ["kanji", "story"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["stories"],
        "additionalProperties": false
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rewrite {
    pub kanji: String,
    pub old: String,
    pub new: String,
}

/// New versions of the given stories with approved names; nothing is written to Anki here.
pub fn rewrite(store: &Store, llm: &dyn Llm, kanji: &[String]) -> Result<Vec<Rewrite>> {
    let wanted: BTreeSet<&String> = kanji.iter().collect();
    let todo: Vec<Candidate> = candidates(store)?.into_iter().filter(|c| wanted.contains(&c.kanji)).collect();
    let mut out = Vec::new();
    for batch in todo.chunks(12) {
        let mut prompt = String::new();
        for c in batch {
            prompt.push_str(&format!("Kanji: {} (keyword: {})\nStory: {}\nReplace: ", c.kanji, c.keyword, c.story));
            let r: Vec<String> = c.replacements.iter().map(|r| format!("{} \"{}\" → \"{}\"", r.component, r.old, r.new.join("\" or \""))).collect();
            prompt.push_str(&r.join("; "));
            prompt.push_str("\n\n");
        }
        let v = llm.json(REWRITE_SYSTEM, &prompt, &rewrite_schema())?;
        let answers: HashMap<String, String> = v["stories"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| Some((s["kanji"].as_str()?.to_string(), s["story"].as_str()?.to_string())))
            .collect();
        for c in batch {
            if let Some(new) = answers.get(&c.kanji).filter(|n| n.trim() != c.story.trim()) {
                out.push(Rewrite { kanji: c.kanji.clone(), old: c.story.clone(), new: new.clone() });
            }
        }
    }
    Ok(out)
}

/// Saves a rewritten story (names only) and moves its analysis to the new text: each old name is
/// replaced by the approved name the new story uses, so the kanji needn't be analysed again.
pub fn save_rewrite(store: &Store, anki: &dyn crate::anki::AnkiSource, kanji: &str, story: &str) -> Result<usize> {
    let written = kanji::save_story(store, anki, kanji, story)?;
    let approved = approved_names(store)?;
    let text = plain_text(story).to_lowercase();
    let mut st = store.conn.prepare("SELECT component, name FROM kanji_primitives WHERE kanji = ?1")?;
    let rows = st.query_map([kanji], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for (c, old) in rows {
        let Some(ok) = approved.get(&c) else { continue };
        if ok.contains(&old) {
            continue;
        }
        if let Some(new) = ok.iter().find(|n| text.contains(n.as_str())) {
            store.conn.execute("DELETE FROM kanji_primitives WHERE kanji = ?1 AND component = ?2 AND name = ?3", params![kanji, c, old])?;
            store.conn.execute(
                "INSERT OR IGNORE INTO kanji_primitives (kanji, component, name, rtk_name)
                 SELECT ?1, ?2, ?3, coalesce((SELECT rtk_name FROM kanji_primitives WHERE component = ?2 LIMIT 1), '')",
                params![kanji, c, new],
            )?;
        }
    }
    store.conn.execute("UPDATE kanji_analysed SET story = ?2, analysed_at = ?3 WHERE kanji = ?1", params![kanji, story, now_ms()])?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::knowledge::{CardFacts, KnowledgeSettings};

    struct Scripted(Mutex<Vec<Value>>, Mutex<Vec<String>>);
    impl Llm for Scripted {
        fn json(&self, _s: &str, user: &str, _schema: &Value) -> Result<Value> {
            self.1.lock().unwrap().push(user.to_string());
            Ok(self.0.lock().unwrap().remove(0))
        }
    }

    fn store() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        let facts = |id| CardFacts { card_id: id, note_id: id, card_type: 2, queue: 2, interval: 30, lapses: 0, stability: None, retrievability: None };
        let note = |id, k: &str, kw: &str, story: &str| kanji::KanjiNoteFacts {
            model_id: 5,
            note_id: id,
            kanji: k.into(),
            keyword: kw.into(),
            story: story.into(),
            cards: vec![facts(id)],
        };
        kanji::replace_kanji(
            &mut s,
            &[
                note(1, "明", "bright", "The <b>sun</b> and the moon make it bright."),
                note(2, "朝", "morning", "Mist and the moon at dawn: morning."),
                note(3, "肝", "liver", "The flesh with a dry pole in it: the liver."),
                note(4, "胴", "trunk", "A body part and a monk's cap: the trunk."),
                note(5, "木", "tree", ""),
            ],
            &KnowledgeSettings::default(),
        )
        .unwrap();
        s
    }

    fn found(items: &[(&str, &[(&str, &str, &str)])]) -> Value {
        json!({"kanji": items.iter().map(|(k, ps)| json!({
            "kanji": k,
            "primitives": ps.iter().map(|(c, n, r)| json!({"component": c, "name": n, "rtk_name": r})).collect::<Vec<_>>()
        })).collect::<Vec<_>>()})
    }

    #[test]
    fn analyse_list_approve_rewrite() {
        let s = store();
        assert_eq!((status(&s).unwrap().with_story, status(&s).unwrap().analysed), (4, 0));
        let llm = Scripted(
            Mutex::new(vec![found(&[
                ("明", &[("日", "sun", "sun"), ("月", "moon", "moon")]),
                ("朝", &[("月", "moon", "moon"), ("十", "mist", "needle")]),
                ("肝", &[("月", "Flesh", "flesh"), ("干", "dry pole", "dry")]),
                ("胴", &[("月", "body part", "flesh")]),
            ])]),
            Mutex::new(vec![]),
        );
        let mut seen = vec![];
        let st = analyse(&s, &llm, |d, t| seen.push((d, t)), || false).unwrap();
        assert_eq!((st.analysed, seen), (4, vec![(4, 4)]));
        assert!(llm.1.lock().unwrap()[0].contains("明 | bright | The sun and the moon make it bright."));
        // Nothing left to do; the empty story isn't sent.
        assert_eq!(analyse(&s, &llm, |_, _| {}, || false).unwrap().analysed, 4);

        let rows = list(&s).unwrap();
        let moon = rows.iter().find(|r| r.component == "月").unwrap();
        assert!(moon.clash && rows[0].component == "月", "clashes first");
        let names: Vec<(&str, usize)> = moon.names.iter().map(|n| (n.name.as_str(), n.count)).collect();
        assert_eq!(names, vec![("moon", 2), ("body part", 1), ("flesh", 1)]);
        assert_eq!((moon.approved.clone(), moon.is_approved, moon.rtk_name.as_str()), (vec!["moon".to_string()], false, "flesh"));

        // 月 is two things: moon and flesh. "body part" becomes "flesh".
        approve(&s, &BTreeMap::from([("月".into(), vec!["moon".into(), "flesh".into()]), ("日".into(), vec!["sun".into()])])).unwrap();
        assert_eq!(approved_for_prompt(&s).unwrap(), "日 sun; 月 flesh / moon");
        let c = candidates(&s).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].kanji.as_str(), c[0].replacements[0].old.as_str()), ("胴", "body part"));

        let llm = Scripted(
            Mutex::new(vec![json!({"stories": [{"kanji": "胴", "story": "Flesh and a monk's cap: the trunk."}]})]),
            Mutex::new(vec![]),
        );
        let r = rewrite(&s, &llm, &["胴".into()]).unwrap();
        assert_eq!(r[0].new, "Flesh and a monk's cap: the trunk.");

        // Saving it moves the analysis along: no new analysis needed, no longer a candidate.
        let (url, _) = crate::anki::fake::serve_logged(vec![crate::anki::fake::FakeNote {
            id: 4,
            model: ("Kanji", 5),
            fields: vec![("Kanji", "胴"), ("Keyword", "trunk"), ("Story", "A body part and a monk's cap: the trunk.")],
            cards: vec![],
        }]);
        kanji::save_mapping(&s, &kanji::KanjiMapping {
            model_id: 5,
            model_name: "Kanji".into(),
            kanji_field: "Kanji".into(),
            keyword_field: Some("Keyword".into()),
            story_field: Some("Story".into()),
        })
        .unwrap();
        assert_eq!(save_rewrite(&s, &crate::anki::AnkiConnect::new(&url), "胴", &r[0].new).unwrap(), 1);
        assert_eq!(status(&s).unwrap().analysed, 4);
        assert!(candidates(&s).unwrap().is_empty());
        let moon = list(&s).unwrap().into_iter().find(|r| r.component == "月").unwrap();
        assert!(moon.names.iter().any(|n| n.name == "flesh" && n.count == 2), "{:?}", moon.names);
        assert!(llm.1.lock().unwrap()[0].contains("月 \"body part\" → \"flesh\" or \"moon\""), "{}", llm.1.lock().unwrap()[0]);

        // A story changed elsewhere is analysed again.
        s.conn.execute("UPDATE kanji SET story = 'New.' WHERE kanji = '肝'", []).unwrap();
        assert_eq!(status(&s).unwrap().analysed, 3);
    }
}
