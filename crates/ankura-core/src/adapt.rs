//! Simpler versions of a text (ADR 0021): a pasted text with too few known words is retold as
//! one or more texts at about 95% known words; together they use every new word of the original.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::generate::{AdaptOf, GenerateRequest, GenerateResult, TARGET_COVERAGE, generate_text};
use crate::llm::Llm;
use crate::reader::analyze;
use crate::store::Store;
use crate::tokenize::{TokenKind, Tokenizer};
use crate::{Error, Result};

/// Characters of the original one version retells.
const SECTION_CHARS: usize = 600;
/// New words per version: about this share of the section's words, within the limits below.
const NEW_SHARE: f64 = 1.0 - TARGET_COVERAGE;
const MIN_NEW: usize = 2;
const MAX_NEW: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPlan {
    /// From 1.
    pub part: usize,
    /// The part of the original it retells.
    pub source: String,
    /// The original's new words it uses, as they appear in the text (dictionary forms).
    pub words: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Share of the original's words the learner knows, 0-1.
    pub coverage: f64,
    /// Every new word of the original, in order of appearance.
    pub new_words: Vec<String>,
    pub versions: Vec<VersionPlan>,
}

/// Paragraph groups of about `SECTION_CHARS` characters.
fn sections(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for p in body.lines().map(str::trim).filter(|p| !p.is_empty()) {
        if !cur.is_empty() && cur.chars().count() + p.chars().count() > SECTION_CHARS {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push('\n');
        }
        cur.push_str(p);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// How a text could be split into simpler versions. No AI.
pub fn plan(store: &Store, tok: &dyn Tokenizer, body: &str) -> Result<Plan> {
    let (mut words, mut known) = (0usize, 0usize);
    let mut seen: HashSet<String> = HashSet::new();
    let mut new_words = Vec::new();
    let mut versions = Vec::new();
    for source in sections(body) {
        let doc = analyze(store, tok, &source)?;
        let mut section_words = 0usize;
        let mut fresh: Vec<String> = Vec::new();
        for t in doc.paragraphs.iter().flatten() {
            if t.kind != TokenKind::Word || t.key.is_none() {
                continue;
            }
            section_words += 1;
            if t.state.is_known() {
                known += 1;
            } else if seen.insert(t.dictionary_form.clone()) {
                fresh.push(t.dictionary_form.clone());
            }
        }
        words += section_words;
        new_words.extend(fresh.iter().cloned());
        if fresh.is_empty() {
            continue;
        }
        let per = ((section_words as f64 * NEW_SHARE).round() as usize).clamp(MIN_NEW, MAX_NEW);
        for chunk in fresh.chunks(per) {
            versions.push(VersionPlan { part: 0, source: source.clone(), words: chunk.to_vec() });
        }
    }
    for (i, v) in versions.iter_mut().enumerate() {
        v.part = i + 1;
    }
    Ok(Plan { coverage: if words == 0 { 1.0 } else { known as f64 / words as f64 }, new_words, versions })
}

/// Writes version `part` of a plan for the text `text_id` and saves it to the library.
pub fn write_version(store: &Store, tok: &dyn Tokenizer, llm: &dyn Llm, text_id: &str, plan: &Plan, part: usize) -> Result<GenerateResult> {
    let v = plan.versions.iter().find(|v| v.part == part).ok_or_else(|| Error::Llm(format!("no version {part} in the plan")))?;
    let polite = v.source.contains("です") || v.source.contains("ます");
    let req = GenerateRequest {
        targets: v.words.clone(),
        topic: "the same as the original below".into(),
        length: v.source.chars().count().clamp(200, 900),
        polite,
        series: None,
        adapt: Some(AdaptOf { text_id: text_id.into(), source: v.source.clone(), part, of: plan.versions.len() }),
    };
    generate_text(store, tok, llm, &req)
}

#[derive(Debug, Clone, Serialize)]
pub struct Version {
    pub text_id: String,
    pub title: String,
    pub part: usize,
    pub of: usize,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub versions: Vec<Version>,
    /// New words of the original no version uses yet.
    pub missing: Vec<String>,
}

/// The simpler versions written for a text, and which of its new words none of them uses yet.
pub fn status(store: &Store, tok: &dyn Tokenizer, text_id: &str) -> Result<Status> {
    let text = store.open_text(text_id)?.ok_or_else(|| Error::Llm("this text no longer exists".into()))?;
    let mut st = store.conn.prepare(
        "SELECT id, title, extra FROM texts WHERE json_extract(extra, '$.adapted_from') = ?1 ORDER BY json_extract(extra, '$.part'), created_at",
    )?;
    let rows = st.query_map([text_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
    let mut versions = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    for row in rows {
        let (id, title, extra) = row?;
        let v: serde_json::Value = serde_json::from_str(&extra).unwrap_or_default();
        let strs = |k: &str| -> Vec<String> { v[k].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect() };
        let missing = strs("missing");
        let targets = strs("targets");
        used.extend(targets.iter().filter(|t| !missing.contains(t)).cloned());
        versions.push(Version {
            text_id: id,
            title,
            part: v["part"].as_u64().unwrap_or(0) as usize,
            of: v["of"].as_u64().unwrap_or(0) as usize,
            targets,
        });
    }
    let missing = if versions.is_empty() {
        vec![]
    } else {
        plan(store, tok, &text.body)?.new_words.into_iter().filter(|w| !used.contains(w)).collect()
    };
    Ok(Status { versions, missing })
}

/// More versions for the new words no version uses yet, retelling the parts where they appear.
pub fn write_missing(store: &Store, tok: &dyn Tokenizer, llm: &dyn Llm, text_id: &str) -> Result<Vec<GenerateResult>> {
    let st = status(store, tok, text_id)?;
    if st.missing.is_empty() {
        return Ok(vec![]);
    }
    let missing: HashSet<String> = st.missing.into_iter().collect();
    let text = store.open_text(text_id)?.ok_or_else(|| Error::Llm("this text no longer exists".into()))?;
    let mut p = plan(store, tok, &text.body)?;
    for v in &mut p.versions {
        v.words.retain(|w| missing.contains(w));
    }
    p.versions.retain(|v| !v.words.is_empty());
    // Numbered after the versions already written.
    let done = st.versions.iter().map(|v| v.part).max().unwrap_or(0);
    for (i, v) in p.versions.iter_mut().enumerate() {
        v.part = done + i + 1;
    }
    let parts: Vec<usize> = p.versions.iter().map(|v| v.part).collect();
    parts.into_iter().map(|part| write_version(store, tok, llm, text_id, &p, part)).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use serde_json::{Value, json};

    use super::*;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::WordKey;

    /// Writes each version as the targets joined with known words.
    struct Writer(Mutex<Vec<String>>);
    impl Llm for Writer {
        fn json(&self, _s: &str, user: &str, _schema: &Value) -> Result<Value> {
            self.0.lock().unwrap().push(user.to_string());
            let targets: Vec<&str> = user.lines().find_map(|l| l.strip_prefix("Target words: ")).unwrap_or("").split('、').collect();
            // Version 1 (経済、政治) never manages 経済, even when asked again.
            let first = targets.len() == 2 && targets.contains(&"政治");
            let used: Vec<&&str> = targets.iter().filter(|t| !(first && **t == "経済")).collect();
            let body: String = used.iter().map(|t| format!("猫は{t}が好きだ。")).collect();
            Ok(json!({"title": "やさしい版", "paragraphs": [body], "examples": []}))
        }
    }

    #[test]
    fn versions_cover_every_new_word() {
        let Some(tok) = crate::tokenize::test_support::sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        let known = [("猫", "ねこ"), ("好き", "すき")];
        let cards: Vec<(i64, CardFacts)> = (0..2)
            .map(|i| (1, CardFacts { card_id: i, note_id: i, card_type: 2, queue: 2, interval: 100, lapses: 0, stability: None, retrievability: Some(0.95) }))
            .collect();
        let keys: Vec<(i64, WordKey)> = known.iter().enumerate().map(|(i, (l, r))| (i as i64, WordKey { lang: "ja".into(), lemma: l.to_string(), reading: r.to_string() })).collect();
        s.replace_anki_snapshot(&cards, &keys, &HashMap::new(), &KnowledgeSettings::default()).unwrap();
        let body = "猫は経済が好きだ。猫は政治が好きだ。猫は歴史が好きだ。";
        let t = s.add_text("ja", "猫と社会", body, "pasted").unwrap();

        let p = plan(&s, tok, body).unwrap();
        assert_eq!(p.new_words, ["経済", "政治", "歴史"]);
        assert!(p.coverage < 0.95);
        assert_eq!(p.versions.len(), 2, "{p:?}"); // two new words at most per short version
        assert_eq!(p.versions[0].words, ["経済", "政治"]);

        let llm = Writer(Mutex::new(vec![]));
        for v in &p.versions {
            write_version(&s, tok, &llm, &t.id, &p, v.part).unwrap();
        }
        assert!(llm.0.lock().unwrap()[0].contains("Original:\n猫は経済が好きだ"), "the original is in the prompt");
        let st = status(&s, tok, &t.id).unwrap();
        assert_eq!((st.versions.len(), st.versions[0].part, st.versions[0].of), (2, 1, 2));
        assert_eq!(st.missing, ["経済"], "the first version left 経済 out");

        let more = write_missing(&s, tok, &llm, &t.id).unwrap();
        assert_eq!(more.len(), 1);
        let st = status(&s, tok, &t.id).unwrap();
        assert!(st.missing.is_empty(), "{st:?}");
        assert_eq!(st.versions.last().unwrap().part, 3);
    }

    #[test]
    fn sections_group_paragraphs() {
        let p = "あ".repeat(400);
        let body = format!("{p}\n\n{p}\n{}", "い".repeat(100));
        let s = sections(&body);
        assert_eq!(s.len(), 2);
        assert_eq!(s[1], format!("{p}\n{}", "い".repeat(100)));
    }
}
