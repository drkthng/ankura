//! Yomitan-format dictionaries (term banks, format 3): import and lookup.

use std::fs::File;
use std::io::Read;
use std::path::Path;

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

/// Imports a Yomitan dictionary zip. Replaces an earlier import with the same title.
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
                let score = r.get(4).and_then(Value::as_i64).unwrap_or(0);
                let senses = gloss_to_senses(r.get(5).unwrap_or(&Value::Null));
                if senses.is_empty() {
                    continue;
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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use serde_json::json;

    use super::*;

    fn write_zip(path: &Path, banks: &[(&str, Value)]) {
        let mut z = zip::ZipWriter::new(File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("index.json", opts).unwrap();
        z.write_all(json!({"title": "Test", "revision": "1", "format": 3}).to_string().as_bytes()).unwrap();
        for (name, v) in banks {
            z.start_file(*name, opts).unwrap();
            z.write_all(v.to_string().as_bytes()).unwrap();
        }
        z.finish().unwrap();
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
