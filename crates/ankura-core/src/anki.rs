//! Talking to Anki. `AnkiSource` is the interface (ADR 0009); AnkiConnect is the desktop
//! implementation (ADR 0004). AnkiDroid's provider becomes a second implementation later.
//! Writing is add-only (ADR 0007): the client refuses any action not on its allow-list.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteInfo {
    pub note_id: i64,
    pub model_name: String,
    /// Field name → raw value (HTML).
    pub fields: BTreeMap<String, String>,
    pub cards: Vec<i64>,
}

pub trait AnkiSource {
    fn version(&self) -> Result<i64>;
    /// (name, id) of every note type.
    fn note_types(&self) -> Result<Vec<(String, i64)>>;
    fn field_names(&self, note_type: &str) -> Result<Vec<String>>;
    fn find_notes(&self, query: &str) -> Result<Vec<i64>>;
    fn find_cards(&self, query: &str) -> Result<Vec<i64>>;
    fn notes_info(&self, note_ids: &[i64]) -> Result<Vec<NoteInfo>>;
    /// Current interval per card: days for review cards, negative seconds for learning cards.
    fn intervals(&self, card_ids: &[i64]) -> Result<Vec<i64>>;
    fn deck_names(&self) -> Result<Vec<String>>;
    /// Adds a note unless Anki says it can't (a duplicate, an empty first field); returns its ID.
    fn add_note(&self, note: &NewNote) -> Result<i64>;
    /// Sets one field of an existing note. Only `kanji::save_story` calls this, and only for the
    /// mnemonic field of a mapped kanji note type (ADR 0013).
    fn update_note_field(&self, note_id: i64, field: &str, value: &str) -> Result<()>;
    fn add_tag(&self, note_ids: &[i64], tag: &str) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewNote {
    pub deck: String,
    pub note_type: String,
    pub fields: BTreeMap<String, String>,
    pub tags: Vec<String>,
}

/// Every AnkiConnect action Ankura may send. The writes: adding notes (ADR 0007), and changing the
/// mnemonic field of kanji notes plus tagging them (ADR 0013).
const ALLOWED_ACTIONS: [&str; 12] = [
    "version",
    "modelNamesAndIds",
    "modelFieldNames",
    "findNotes",
    "findCards",
    "notesInfo",
    "getIntervals",
    "deckNames",
    "canAddNotesWithErrorDetail",
    "addNote",
    "updateNoteFields",
    "addTags",
];

pub struct AnkiConnect {
    url: String,
    agent: ureq::Agent,
}

impl AnkiConnect {
    pub const DEFAULT_URL: &'static str = "http://127.0.0.1:8765";

    pub fn new(url: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(3)))
            .timeout_global(Some(Duration::from_secs(300)))
            .build()
            .new_agent();
        AnkiConnect { url: url.to_string(), agent }
    }

    fn call(&self, action: &str, params: Value) -> Result<Value> {
        if !ALLOWED_ACTIONS.contains(&action) {
            return Err(Error::Anki(format!("Ankura doesn't send the AnkiConnect action {action}")));
        }
        let body = json!({ "action": action, "version": 6, "params": params });
        let mut resp = self
            .agent
            .post(&self.url)
            .send_json(&body)
            .map_err(|e| Error::Anki(format!("can't reach AnkiConnect at {}: {e}", self.url)))?;
        let v: Value = resp.body_mut().read_json().map_err(|e| Error::Anki(format!("bad AnkiConnect reply: {e}")))?;
        if let Some(err) = v.get("error").and_then(Value::as_str) {
            return Err(Error::Anki(format!("AnkiConnect {action}: {err}")));
        }
        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    }

    fn ids(v: Value) -> Result<Vec<i64>> {
        serde_json::from_value(v).map_err(|e| Error::Anki(e.to_string()))
    }
}

const BATCH: usize = 500;

impl AnkiSource for AnkiConnect {
    fn version(&self) -> Result<i64> {
        self.call("version", json!({}))?.as_i64().ok_or_else(|| Error::Anki("no version".into()))
    }

    fn note_types(&self) -> Result<Vec<(String, i64)>> {
        let v = self.call("modelNamesAndIds", json!({}))?;
        let map: BTreeMap<String, i64> = serde_json::from_value(v).map_err(|e| Error::Anki(e.to_string()))?;
        Ok(map.into_iter().collect())
    }

    fn field_names(&self, note_type: &str) -> Result<Vec<String>> {
        let v = self.call("modelFieldNames", json!({ "modelName": note_type }))?;
        serde_json::from_value(v).map_err(|e| Error::Anki(e.to_string()))
    }

    fn find_notes(&self, query: &str) -> Result<Vec<i64>> {
        Self::ids(self.call("findNotes", json!({ "query": query }))?)
    }

    fn find_cards(&self, query: &str) -> Result<Vec<i64>> {
        Self::ids(self.call("findCards", json!({ "query": query }))?)
    }

    fn notes_info(&self, note_ids: &[i64]) -> Result<Vec<NoteInfo>> {
        let mut out = Vec::with_capacity(note_ids.len());
        for chunk in note_ids.chunks(BATCH) {
            let v = self.call("notesInfo", json!({ "notes": chunk }))?;
            for n in v.as_array().into_iter().flatten() {
                let Some(note_id) = n.get("noteId").and_then(Value::as_i64) else { continue };
                let fields = n
                    .get("fields")
                    .and_then(Value::as_object)
                    .map(|m| {
                        m.iter()
                            .map(|(k, f)| (k.clone(), f.get("value").and_then(Value::as_str).unwrap_or("").to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let cards = n
                    .get("cards")
                    .and_then(|c| serde_json::from_value(c.clone()).ok())
                    .ok_or_else(|| Error::Anki("notesInfo returned no card IDs; please update AnkiConnect".into()))?;
                out.push(NoteInfo {
                    note_id,
                    model_name: n.get("modelName").and_then(Value::as_str).unwrap_or("").to_string(),
                    fields,
                    cards,
                });
            }
        }
        Ok(out)
    }

    fn intervals(&self, card_ids: &[i64]) -> Result<Vec<i64>> {
        let mut out = Vec::with_capacity(card_ids.len());
        for chunk in card_ids.chunks(5000) {
            out.extend(Self::ids(self.call("getIntervals", json!({ "cards": chunk, "complete": false }))?)?);
        }
        Ok(out)
    }

    fn deck_names(&self) -> Result<Vec<String>> {
        serde_json::from_value(self.call("deckNames", json!({}))?).map_err(|e| Error::Anki(e.to_string()))
    }

    fn add_note(&self, note: &NewNote) -> Result<i64> {
        let n = json!({
            "deckName": note.deck,
            "modelName": note.note_type,
            "fields": note.fields,
            "tags": note.tags,
            "options": {"allowDuplicate": false},
        });
        let check = self.call("canAddNotesWithErrorDetail", json!({ "notes": [n] }))?;
        if check[0]["canAdd"] == false {
            let why = check[0]["error"].as_str().unwrap_or("Anki refused the note");
            return Err(Error::Anki(format!("can't add the note: {why}")));
        }
        self.call("addNote", json!({ "note": n }))?.as_i64().ok_or_else(|| Error::Anki("addNote returned no ID".into()))
    }

    fn update_note_field(&self, note_id: i64, field: &str, value: &str) -> Result<()> {
        self.call("updateNoteFields", json!({ "note": { "id": note_id, "fields": { field: value } } }))?;
        Ok(())
    }

    fn add_tag(&self, note_ids: &[i64], tag: &str) -> Result<()> {
        self.call("addTags", json!({ "notes": note_ids, "tags": tag }))?;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! An in-process AnkiConnect stand-in for tests: a tiny HTTP server over a fixed collection.

    use std::collections::{BTreeMap, HashSet};
    use std::sync::{Arc, Mutex};
    use std::thread;

    use serde_json::{Value, json};

    pub struct FakeCard {
        pub id: i64,
        pub interval: i64,
        pub flags: &'static [&'static str],
        pub stability: Option<f64>,
        pub retrievability: Option<f64>,
        /// Days overdue (review cards).
        pub overdue: i64,
    }

    pub struct FakeNote {
        pub id: i64,
        pub model: (&'static str, i64),
        pub fields: Vec<(&'static str, &'static str)>,
        pub cards: Vec<FakeCard>,
    }

    /// Answers the subset of queries Ankura sends: `mid:N`, optional `is:new|learn|suspended|buried`,
    /// optional `prop:s>=X` / `prop:r>=X`, joined by spaces, with `(...)` wrapping.
    fn matches(note: &FakeNote, card: &FakeCard, query: &str) -> bool {
        let q = query.replace(['(', ')'], " ");
        let mut mids: Vec<i64> = Vec::new();
        let mut ok = true;
        let mut words = q.split_whitespace().peekable();
        while let Some(w) = words.next() {
            if w == "OR" {
                continue;
            }
            if let Some(m) = w.strip_prefix("mid:") {
                mids.push(m.parse().unwrap());
            } else if let Some(flag) = w.strip_prefix("is:") {
                ok &= card.flags.contains(&flag);
            } else if let Some(v) = w.strip_prefix("-is:") {
                ok &= !card.flags.contains(&v);
            } else if let Some(v) = w.strip_prefix("prop:s>=") {
                ok &= card.stability.is_some_and(|s| s >= v.parse::<f64>().unwrap());
            } else if let Some(v) = w.strip_prefix("prop:r>=") {
                ok &= card.retrievability.is_some_and(|r| r >= v.parse::<f64>().unwrap());
            } else if let Some(v) = w.strip_prefix("prop:due<=") {
                ok &= -card.overdue <= v.parse::<i64>().unwrap() && card.flags.contains(&"review");
            } else if w.starts_with("deck:") {
            } else {
                panic!("fake AnkiConnect can't handle query word {w:?} in {query:?}");
            }
        }
        ok && (mids.is_empty() || mids.contains(&note.model.1))
    }

    pub fn serve(notes: Vec<FakeNote>) -> String {
        serve_logged(notes).0
    }

    /// Like `serve`, and also returns the notes added through `addNote`.
    pub fn serve_logged(notes: Vec<FakeNote>) -> (String, Arc<Mutex<Vec<Value>>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr().to_ip().unwrap());
        let added = Arc::new(Mutex::new(Vec::<Value>::new()));
        let log = added.clone();
        // A duplicate shares the first field of a note of the same type.
        let first_fields: Vec<(&'static str, &'static str)> =
            notes.iter().map(|n| (n.model.0, n.fields.first().map(|f| f.1).unwrap_or(""))).collect();
        // Field values changed through updateNoteFields, and tags added.
        let mut changed: BTreeMap<(i64, String), String> = BTreeMap::new();
        thread::spawn(move || {
            for mut req in server.incoming_requests() {
                let mut body = String::new();
                req.as_reader().read_to_string(&mut body).unwrap();
                let v: Value = serde_json::from_str(&body).unwrap();
                let p = &v["params"];
                let result = match v["action"].as_str().unwrap() {
                    "version" => json!(6),
                    "modelNamesAndIds" => {
                        let m: BTreeMap<&str, i64> = notes.iter().map(|n| n.model).collect();
                        json!(m)
                    }
                    "modelFieldNames" => {
                        let name = p["modelName"].as_str().unwrap();
                        let n = notes.iter().find(|n| n.model.0 == name).unwrap();
                        json!(n.fields.iter().map(|(k, _)| *k).collect::<Vec<_>>())
                    }
                    "findNotes" => {
                        let q = p["query"].as_str().unwrap();
                        json!(notes.iter().filter(|n| n.cards.iter().any(|c| matches(n, c, q))).map(|n| n.id).collect::<Vec<_>>())
                    }
                    "findCards" => {
                        let q = p["query"].as_str().unwrap();
                        let ids: Vec<i64> = notes
                            .iter()
                            .flat_map(|n| n.cards.iter().filter(|c| matches(n, c, q)).map(|c| c.id))
                            .collect();
                        json!(ids)
                    }
                    "notesInfo" => {
                        let want: HashSet<i64> = p["notes"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
                        json!(notes
                            .iter()
                            .filter(|n| want.contains(&n.id))
                            .map(|n| {
                                let fields: serde_json::Map<String, Value> = n
                                    .fields
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (k, val))| {
                                        let v = changed.get(&(n.id, k.to_string())).map(String::as_str).unwrap_or(val);
                                        (k.to_string(), json!({"value": v, "order": i}))
                                    })
                                    .collect();
                                json!({"noteId": n.id, "modelName": n.model.0, "fields": fields,
                                       "cards": n.cards.iter().map(|c| c.id).collect::<Vec<_>>()})
                            })
                            .collect::<Vec<_>>())
                    }
                    "getIntervals" => {
                        let ids: Vec<i64> = p["cards"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
                        json!(ids
                            .iter()
                            .map(|id| notes.iter().flat_map(|n| &n.cards).find(|c| c.id == *id).unwrap().interval)
                            .collect::<Vec<_>>())
                    }
                    "deckNames" => json!(["Default", "Japanisch"]),
                    "canAddNotesWithErrorDetail" => {
                        let n = &p["notes"][0];
                        let model = n["modelName"].as_str().unwrap();
                        let fields = n["fields"].as_object().unwrap();
                        let first = fields.values().next().and_then(Value::as_str).unwrap_or("");
                        let dup = first_fields.iter().any(|(m, f)| *m == model && *f == first)
                            || log.lock().unwrap().iter().any(|a| {
                                a["modelName"] == model
                                    && a["fields"].as_object().and_then(|f| f.values().next()).and_then(Value::as_str) == Some(first)
                            });
                        if dup {
                            json!([{"canAdd": false, "error": "cannot create note because it is a duplicate"}])
                        } else {
                            json!([{"canAdd": true}])
                        }
                    }
                    "updateNoteFields" => {
                        let id = p["note"]["id"].as_i64().unwrap();
                        for (k, v) in p["note"]["fields"].as_object().unwrap() {
                            changed.insert((id, k.clone()), v.as_str().unwrap().to_string());
                        }
                        log.lock().unwrap().push(json!({"updateNoteFields": p["note"].clone()}));
                        Value::Null
                    }
                    "addTags" => {
                        log.lock().unwrap().push(json!({"addTags": p.clone()}));
                        Value::Null
                    }
                    "addNote" => {
                        let mut l = log.lock().unwrap();
                        l.push(p["note"].clone());
                        json!(9000 + l.len() as i64)
                    }
                    other => panic!("fake AnkiConnect: unexpected action {other}"),
                };
                let reply = json!({ "result": result, "error": null }).to_string();
                req.respond(tiny_http::Response::from_string(reply)).unwrap();
            }
        });
        (url, added)
    }
}

#[cfg(test)]
mod tests {
    use super::fake::*;
    use super::*;

    #[test]
    fn talks_to_ankiconnect() {
        let url = serve(vec![FakeNote {
            id: 100,
            model: ("Core", 7),
            fields: vec![("Vocab", "<b>猫</b>"), ("Reading", "ねこ")],
            cards: vec![FakeCard { id: 1, interval: 30, flags: &["review"], stability: None, retrievability: None, overdue: 0 }],
        }]);
        let a = AnkiConnect::new(&url);
        assert_eq!(a.version().unwrap(), 6);
        assert_eq!(a.note_types().unwrap(), vec![("Core".to_string(), 7)]);
        assert_eq!(a.field_names("Core").unwrap(), vec!["Vocab", "Reading"]);
        assert_eq!(a.find_notes("mid:7").unwrap(), vec![100]);
        let n = a.notes_info(&[100]).unwrap();
        assert_eq!(n[0].fields["Vocab"], "<b>猫</b>");
        assert_eq!(n[0].cards, vec![1]);
        assert_eq!(a.intervals(&[1]).unwrap(), vec![30]);
    }

    #[test]
    fn adds_notes_and_refuses_duplicates() {
        let (url, added) = serve_logged(vec![FakeNote {
            id: 100,
            model: ("Core", 7),
            fields: vec![("Vocab", "猫"), ("Reading", "ねこ")],
            cards: vec![],
        }]);
        let a = AnkiConnect::new(&url);
        assert_eq!(a.deck_names().unwrap(), vec!["Default", "Japanisch"]);
        let note = |vocab: &str| NewNote {
            deck: "Default".into(),
            note_type: "Core".into(),
            fields: BTreeMap::from([("Vocab".into(), vocab.into()), ("Reading".into(), "x".into())]),
            tags: vec!["ankura".into()],
        };
        assert_eq!(a.add_note(&note("犬")).unwrap(), 9001);
        let e = a.add_note(&note("猫")).unwrap_err().to_string();
        assert!(e.contains("duplicate"), "{e}");
        let log = added.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0]["tags"][0], "ankura");
        assert_eq!(log[0]["options"]["allowDuplicate"], false);
    }

    #[test]
    fn only_sends_allowed_actions() {
        let a = AnkiConnect::new("http://127.0.0.1:9");
        let e = a.call("deleteNotes", json!({"notes": [1]})).unwrap_err().to_string();
        assert!(e.contains("doesn't send"), "{e}");
    }

    #[test]
    fn reports_unreachable_anki() {
        let a = AnkiConnect::new("http://127.0.0.1:9");
        let e = a.version().unwrap_err().to_string();
        assert!(e.contains("can't reach AnkiConnect"), "{e}");
    }
}
