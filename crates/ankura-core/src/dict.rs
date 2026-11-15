//! Yomitan-format dictionaries (term banks, format 3): import and lookup.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::store::{Store, now_ms};
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryInfo {
    pub id: i64,
    pub title: String,
    pub revision: String,
    pub terms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictEntry {
    pub expression: String,
    pub reading: String,
    /// One plain-text string per sense.
    pub senses: Vec<String>,
    pub tags: String,
    pub score: i64,
    pub dictionary: String,
}

/// Flattens a Yomitan glossary item (plain string, `{type: "text"}`, or structured content) to text.
fn flatten(v: &Value, out: &mut String) {
    match v {
        Value::String(s) => out.push_str(s),
        Value::Array(items) => items.iter().for_each(|i| flatten(i, out)),
        Value::Object(map) => {
            match map.get("type").and_then(Value::as_str) {
                Some("text") => {
                    if let Some(t) = map.get("text") {
                        flatten(t, out);
                    }
                    return;
                }
                Some("image") => return,
                _ => {}
            }
            let tag = map.get("tag").and_then(Value::as_str).unwrap_or("");
            if tag == "rt" || tag == "rp" {
                return;
            }
            let block = matches!(tag, "li" | "div" | "p" | "br" | "tr" | "ol" | "ul" | "details" | "summary");
            if block && !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            if let Some(c) = map.get("content") {
                flatten(c, out);
            }
            if block && !out.ends_with('\n') {
                out.push('\n');
            }
        }
        _ => {}
    }
}

/// What a part of a Wiktionary entry is (wiktionary-to-yomitan marks them): "glosses",
/// "Etymology-content", "details-entry-examples", "tags", "backlink"…
fn marked(v: &Value) -> &str {
    v.get("data").and_then(|d| d.get("content")).and_then(Value::as_str).unwrap_or("")
}

/// The text of one gloss: its words and sub-glosses, without examples, tags and other marked parts.
fn gloss_text(v: &Value, out: &mut String) {
    match v {
        Value::String(s) => out.push_str(s),
        Value::Array(items) => items.iter().for_each(|i| gloss_text(i, out)),
        Value::Object(map) => {
            if !marked(v).is_empty() {
                return;
            }
            if map.get("tag").and_then(Value::as_str) == Some("li") && !out.trim().is_empty() && !out.trim_end().ends_with(';') {
                out.push_str("; ");
            }
            if let Some(c) = map.get("content") {
                gloss_text(c, out);
            }
        }
        _ => {}
    }
}

/// The glosses of a Wiktionary entry, one per sense; empty for other dictionaries.
fn wiktionary_glosses(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Array(items) => items.iter().for_each(|i| wiktionary_glosses(i, out)),
        Value::Object(map) => {
            if marked(v) == "glosses" {
                let items: Vec<&Value> = match map.get("content") {
                    Some(Value::Array(a)) => a.iter().collect(),
                    Some(one) => vec![one],
                    None => vec![],
                };
                for li in items {
                    let mut s = String::new();
                    gloss_text(li.get("content").unwrap_or(li), &mut s);
                    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !s.is_empty() {
                        out.push(s);
                    }
                }
            } else if let Some(c) = map.get("content") {
                wiktionary_glosses(c, out);
            }
        }
        _ => {}
    }
}

static FORM_OF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:person|singular|plural|indicative|subjunctive|imperative|conditional|participle|gerund|infinitive|form|feminine|masculine|superlative|historic|imperfect|preterite|future|present) of ([^\s,:;()]+)",
    )
    .unwrap()
});
static COMPOUND_OF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^compound of the (?:infinitive|gerund|imperative|past participle|present participle) ([^\s,:;()]+)").unwrap());

/// The word a gloss says this entry is a form of, when the gloss is nothing but that: "third-person
/// singular present indicative of avere", "compound of dando, the gerund of dare, with lo"
/// (the last word named), "compound of the infinitive parlare with ne", "apocopic form of poco".
fn form_of(gloss: &str) -> Option<String> {
    let first = gloss.split_whitespace().next().unwrap_or("");
    if matches!(first, "a" | "an" | "the" | "to" | "used" | "used," | "any" | "one" | "synonym" | "suffix" | "prefix" | "stem,") {
        return None;
    }
    let target = match FORM_OF.captures_iter(gloss).last() {
        Some(c) => c[1].to_string(),
        None => COMPOUND_OF.captures(gloss)?[1].to_string(),
    };
    let t = target.trim_matches(|c: char| !c.is_alphabetic() && c != '\'');
    (!t.is_empty()).then(|| t.to_string())
}

/// The senses of an entry, and whether they are a Wiktionary entry's glosses.
fn senses_of(glossary: &Value) -> (Vec<String>, bool) {
    let mut glosses = Vec::new();
    wiktionary_glosses(glossary, &mut glosses);
    if glosses.is_empty() { (gloss_to_senses(glossary), false) } else { (glosses, true) }
}

fn gloss_to_senses(glossary: &Value) -> Vec<String> {
    let mut senses = Vec::new();
    for item in glossary.as_array().into_iter().flatten() {
        let mut s = String::new();
        flatten(item, &mut s);
        let text = s.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("; ");
        if !text.is_empty() {
            senses.push(text);
        }
    }
    senses
}

/// The words a form row points to, with what form it is: Wiktionary dictionaries in Yomitan
/// format (wiktionary-to-yomitan) tag such rows `non-lemma` and give `[lemma, [inflections]]`
/// pairs instead of a meaning (case → casa, plural).
fn form_targets(glossary: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for item in glossary.as_array().into_iter().flatten() {
        let Some(pair) = item.as_array() else { continue };
        let Some(lemma) = pair.first().and_then(Value::as_str) else { continue };
        let lemma = crate::romance::clean_lemma(lemma);
        if lemma.is_empty() {
            continue;
        }
        let info = pair
            .get(1)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; "))
            .unwrap_or_default();
        out.push((lemma, info));
    }
    out
}

/// Imports a Yomitan dictionary zip. Replaces an earlier import with the same title. Rows for
/// inflected forms go into the form table (ADR 0023), not into the entries.
pub fn import_yomitan_zip(store: &mut Store, path: &Path) -> Result<DictionaryInfo> {
    let mut zip = zip::ZipArchive::new(File::open(path)?).map_err(|e| Error::Dictionary(e.to_string()))?;
    let read = |zip: &mut zip::ZipArchive<File>, name: &str| -> Result<String> {
        let mut f = zip.by_name(name).map_err(|e| Error::Dictionary(format!("{name}: {e}")))?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Ok(s)
    };
    let index: Value = serde_json::from_str(&read(&mut zip, "index.json")?)?;
    let title = index["title"].as_str().unwrap_or("Dictionary").to_string();
    let revision = index["revision"].as_str().unwrap_or("").to_string();
    let format = index["format"].as_i64().or_else(|| index["version"].as_i64()).unwrap_or(3);
    if format != 3 {
        return Err(Error::Dictionary(format!("unsupported Yomitan dictionary format {format}")));
    }
    let mut banks: Vec<String> = zip
        .file_names()
        .filter_map(|n| n.ok().map(|n| n.into_owned()))
        .filter(|n| n.starts_with("term_bank_") && n.ends_with(".json"))
        .collect();
    if banks.is_empty() {
        return Err(Error::Dictionary("no term banks in this zip (is it a kanji or frequency dictionary?)".into()));
    }
    banks.sort_by_key(|n| n.trim_start_matches("term_bank_").trim_end_matches(".json").parse::<u32>().unwrap_or(0));

    let tx = store.conn.transaction()?;
    tx.execute("DELETE FROM dictionaries WHERE title = ?1", [&title])?;
    tx.execute(
        "INSERT INTO dictionaries (title, revision, imported_at) VALUES (?1, ?2, ?3)",
        params![title, revision, now_ms()],
    )?;
    let dict_id = tx.last_insert_rowid();
    let mut terms = 0i64;
    {
        let mut ins_form = tx.prepare("INSERT INTO word_forms (dict_id, form, lemma, info) VALUES (?1, ?2, ?3, ?4)")?;
        let mut ins = tx.prepare(
            "INSERT INTO dict_terms (dict_id, expression, reading, tags, score, glossary, sequence, term_tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for name in &banks {
            let rows: Vec<Value> = serde_json::from_str(&read(&mut zip, name)?)?;
            for row in rows {
                let Some(r) = row.as_array() else { continue };
                let expression = r.first().and_then(Value::as_str).unwrap_or("");
                if expression.is_empty() {
                    continue;
                }
                let reading = r.get(1).and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or(expression);
                let tags = r.get(2).and_then(Value::as_str).unwrap_or("");
                if tags.split_whitespace().any(|t| t == "non-lemma") {
                    for (lemma, info) in form_targets(r.get(5).unwrap_or(&Value::Null)) {
                        if lemma != expression {
                            ins_form.execute(params![dict_id, expression, lemma, info])?;
                        }
                    }
                    continue;
                }
                let score = r.get(4).and_then(Value::as_i64).unwrap_or(0);
                let (senses, wiktionary) = senses_of(r.get(5).unwrap_or(&Value::Null));
                if senses.is_empty() {
                    continue;
                }
                // An entry that only says which form it is (sono: "first-person singular present
                // indicative of essere") is a form, not a word of its own.
                if wiktionary {
                    let targets: Vec<String> = senses.iter().filter_map(|s| form_of(s)).collect();
                    if targets.len() == senses.len() {
                        for (lemma, info) in targets.iter().zip(&senses) {
                            if lemma != expression {
                                ins_form.execute(params![dict_id, expression, lemma, info])?;
                            }
                        }
                        continue;
                    }
                }
                let sequence = r.get(6).and_then(Value::as_i64).unwrap_or(0);
                let term_tags = r.get(7).and_then(Value::as_str).unwrap_or("");
                ins.execute(params![
                    dict_id,
                    expression,
                    reading,
                    tags,
                    score,
                    serde_json::to_string(&senses)?,
                    sequence,
                    term_tags
                ])?;
                terms += 1;
            }
        }
    }
    tx.commit()?;
    Ok(DictionaryInfo { id: dict_id, title, revision, terms })
}

pub fn dictionaries(store: &Store) -> Result<Vec<DictionaryInfo>> {
    let mut st = store.conn.prepare(
        "SELECT d.id, d.title, d.revision, (SELECT count(*) FROM dict_terms t WHERE t.dict_id = d.id)
         FROM dictionaries d ORDER BY d.id",
    )?;
    let rows = st.query_map([], |r| {
        Ok(DictionaryInfo { id: r.get(0)?, title: r.get(1)?, revision: r.get(2)?, terms: r.get(3)? })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Entries for a word, best first: spelling and reading both match, then spelling only, then
/// (for words written in kana) entries whose reading matches.
pub fn lookup(store: &Store, lemma: &str, reading: &str) -> Result<Vec<DictEntry>> {
    let mut st = store.conn.prepare_cached(
        "SELECT t.expression, t.reading, t.glossary, t.tags, t.score, d.title,
                (t.expression = ?1 AND t.reading = ?2) AS exact,
                (t.expression = ?1) AS spelled
         FROM dict_terms t JOIN dictionaries d ON d.id = t.dict_id
         WHERE t.expression = ?1 OR t.reading = ?1
         ORDER BY exact DESC, spelled DESC, t.score DESC, t.sequence
         LIMIT 12",
    )?;
    let rows = st.query_map(params![lemma, reading], |r| {
        let glossary: String = r.get(2)?;
        Ok(DictEntry {
            expression: r.get(0)?,
            reading: r.get(1)?,
            senses: serde_json::from_str(&glossary).unwrap_or_default(),
            tags: r.get(3)?,
            score: r.get(4)?,
            dictionary: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Other readings the dictionary lists for this spelling (for correctable furigana).
pub fn readings(store: &Store, expression: &str) -> Result<Vec<String>> {
    let mut st = store.conn.prepare_cached(
        "SELECT reading FROM dict_terms WHERE expression = ?1 GROUP BY reading ORDER BY max(score) DESC",
    )?;
    let rows = st.query_map([expression], |r| r.get(0))?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// Writes a Yomitan zip with the given term banks (tests).
#[cfg(test)]
pub(crate) fn write_zip(path: &Path, banks: &[(&str, Value)]) {
    use std::io::Write;
    let mut z = zip::ZipWriter::new(File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    z.start_file("index.json", opts).unwrap();
    z.write_all(serde_json::json!({"title": "Test", "revision": "1", "format": 3}).to_string().as_bytes()).unwrap();
    for (name, v) in banks {
        z.start_file(*name, opts).unwrap();
        z.write_all(v.to_string().as_bytes()).unwrap();
    }
    z.finish().unwrap();
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;


    #[test]
    fn wiktionary_entries_keep_their_glosses_and_forms_go_to_the_form_table() {
        let entry = |glosses: serde_json::Value| {
            json!({"type": "structured-content", "content": [
                {"tag": "div", "content": [{"tag": "div", "data": {"content": "preamble"}, "content": [
                    {"tag": "details", "data": {"content": "details-entry-Etymology"}, "content": [
                        {"tag": "summary", "data": {"content": "summary-entry"}, "content": "Etymology"},
                        {"tag": "div", "data": {"content": "Etymology-content"}, "content": "Inherited from Latin."}]}]}]},
                {"tag": "ol", "data": {"content": "glosses"}, "lang": "en", "content": glosses},
                {"tag": "div", "data": {"content": "backlink"}, "content": "Wiktionary"}
            ]})
        };
        let li = |text: &str| json!({"tag": "li", "content": [{"tag": "div", "content": [text]}]});
        let house = json!({"tag": "li", "content": [
            {"tag": "div", "content": ["homeland", {"tag": "details", "data": {"content": "details-entry-examples"}, "content": "an example"}]},
            {"tag": "ul", "content": [{"tag": "li", "content": [{"tag": "div", "data": {"content": "tags"}, "content": "fig"}, "one's customs"]}]}
        ]});
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("w.zip");
        write_zip(
            &path,
            &[(
                "term_bank_1.json",
                json!([
                    ["casa", "", "n fem", "", 0, [entry(json!([li("house"), house]))], 0, ""],
                    ["sono", "", "v sg", "", 0, [entry(json!([li("first-person singular present indicative of essere: (I) am, I'm")]))], 0, ""],
                    ["dandolo", "", "v", "", 0, [entry(json!([li("compound of dando, the gerund of dare, with lo")]))], 0, ""],
                    ["parlarne", "", "v", "", 0, [entry(json!([li("compound of the infinitive parlare with ne")]))], 0, ""],
                    ["amo", "", "n masc", "", 0, [entry(json!([li("hook")]))], 0, ""],
                    ["governo", "", "n", "", 0, [entry(json!([li("a form of rule"), li("government")]))], 0, ""],
                    ["case", "", "non-lemma", "", 0, [["casa", ["plural"]]], 0, ""],
                    ["dímelo", "", "non-lemma", "v", 0, [["decir combined with indirect object me and direct object lo", ["imperative"]]], 0, ""]
                ]),
            )],
        );
        let mut s = Store::open_in_memory().unwrap();
        let info = import_yomitan_zip(&mut s, &path).unwrap();
        assert_eq!(info.terms, 3, "casa, amo, governo");
        assert_eq!(lookup(&s, "casa", "").unwrap()[0].senses, ["house", "homeland; one's customs"]);
        assert!(lookup(&s, "sono", "").unwrap().is_empty() && lookup(&s, "case", "").unwrap().is_empty());
        let mut st = s.conn.prepare("SELECT form, lemma FROM word_forms ORDER BY form").unwrap();
        let forms: Vec<(String, String)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(|r| r.unwrap()).collect();
        let want = [("case", "casa"), ("dandolo", "dare"), ("dímelo", "decir"), ("parlarne", "parlare"), ("sono", "essere")];
        assert_eq!(forms, want.map(|(f, l)| (f.to_string(), l.to_string())));
        assert_eq!(form_of("apocopic form of poco"), Some("poco".to_string()));
        assert_eq!(form_of("a form of government"), None);
        assert_eq!(form_of("hook"), None);
    }

    #[test]
    fn import_and_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.zip");
        let structured = json!({"type": "structured-content", "content": [
            {"tag": "ul", "content": [
                {"tag": "li", "content": "popularity"},
                {"tag": "li", "content": ["public ", {"tag": "span", "content": "favor"}]}
            ]},
            {"tag": "ruby", "content": ["人", {"tag": "rt", "content": "ひと"}]}
        ]});
        write_zip(
            &path,
            &[
                ("term_bank_1.json", json!([
                    ["分かる", "わかる", "v5", "v5", 100, ["to understand"], 1, "P"],
                    ["人気", "にんき", "n", "", 100, [structured], 2, "P"],
                    ["人気", "ひとけ", "n", "", 0, [{"type": "text", "text": "sign of life"}], 3, ""]
                ])),
                ("term_bank_2.json", json!([["ねこ", "", "n", "", 0, ["cat"], 4, ""]])),
            ],
        );
        let mut s = Store::open_in_memory().unwrap();
        let info = import_yomitan_zip(&mut s, &path).unwrap();
        assert_eq!(info.terms, 4);

        // A kana word finds the kanji entry through its reading.
        let e = lookup(&s, "わかる", "わかる").unwrap();
        assert_eq!(e[0].expression, "分かる");

        let e = lookup(&s, "人気", "ひとけ").unwrap();
        assert_eq!(e[0].reading, "ひとけ");
        assert_eq!(e[1].senses, vec!["popularity; public favor; 人".to_string()]);
        assert_eq!(readings(&s, "人気").unwrap(), vec!["にんき", "ひとけ"]);

        // An entry without a reading uses the expression.
        assert_eq!(lookup(&s, "ねこ", "ねこ").unwrap()[0].reading, "ねこ");

        // Re-importing the same title replaces it.
        import_yomitan_zip(&mut s, &path).unwrap();
        assert_eq!(dictionaries(&s).unwrap().len(), 1);
        assert_eq!(dictionaries(&s).unwrap()[0].terms, 4);
    }
}
