//! Questions after a text (ADR 0019): two comprehension questions and a cloze on up to two of its
//! target words. Retrieval right after reading; missed words come back as practice words.
//! Questions are checked like the tests' (ADR 0014) and kept in the bank (ADR 0016), so a text
//! asks the AI once.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::llm::Llm;
use crate::proficiency::{Mc, Rng, numbered, shuffled, solve, valid};
use crate::store::{NewEvent, Store};
use crate::tokenize::Tokenizer;
use crate::{Error, Result};

const COMPREHENSION: usize = 2;
const CLOZES: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuizItem {
    pub id: String,
    /// 'comprehension' or 'cloze'.
    pub kind: String,
    pub question: String,
    pub options: Vec<String>,
    pub answer: usize,
    pub explanation: String,
    /// The target word a cloze asks for.
    #[serde(default)]
    pub word: Option<String>,
}

const SYSTEM: &str = "You write quick questions to check that a learner understood a Japanese text they just \
read. Questions and options are in Japanese at the text's level; explanations are short and in English.
- comprehension: about what happens or what is meant in the text, four options, exactly one right; it must need the \
text (not answerable from general knowledge).
- cloze: take a sentence from the text that uses the target word, replace the word (in the form it has there) with \
（　）, and give four options: the right form of the target word and three words of the same kind that don't fit.";

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "questions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "kind": {"type": "string", "enum": ["comprehension", "cloze"]},
                        "question": {"type": "string"},
                        "options": {"type": "array", "items": {"type": "string"}},
                        "answer": {"type": "integer"},
                        "explanation": {"type": "string"},
                        "word": {"type": "string"}
                    },
                    "required": ["kind", "question", "options", "answer", "explanation", "word"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["questions"],
        "additionalProperties": false
    })
}

fn from_bank(store: &Store, text_id: &str) -> Result<Vec<QuizItem>> {
    let mut out = Vec::new();
    for r in crate::memory::bank_candidates(store, "text_quiz", "text", 0, true)? {
        if r.group.as_deref() == Some(text_id) {
            if let Ok(it) = serde_json::from_str::<QuizItem>(&r.item) {
                out.push(it);
            }
        }
    }
    // Comprehension first, then the clozes.
    out.sort_by_key(|i| i.kind != "comprehension");
    Ok(out)
}

/// The questions for a text: from the bank when it was asked before, else written and checked.
pub fn questions(store: &Store, llm: &dyn Llm, text_id: &str) -> Result<Vec<QuizItem>> {
    let banked = from_bank(store, text_id)?;
    if !banked.is_empty() {
        crate::memory::reused(store, "text_quiz", banked.len())?;
        return Ok(banked);
    }
    let text = store.open_text(text_id)?.ok_or_else(|| Error::Llm("this text no longer exists".into()))?;
    let targets: Vec<String> = text
        .extra
        .as_ref()
        .and_then(|e| e["targets"].as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| t.as_str().map(String::from))
        .collect();
    let clozes: Vec<&String> = targets.iter().take(CLOZES).collect();
    let mut prompt = format!("Text:\n{}\n\nWrite {} comprehension questions", text.body, COMPREHENSION + 1);
    if clozes.is_empty() {
        prompt.push_str(" (no cloze: the text has no target words).\n");
    } else {
        prompt.push_str(&format!(
            " and one cloze for each of these target words: {}. For comprehension questions, word = empty.\n",
            clozes.iter().map(|t| t.as_str()).collect::<Vec<_>>().join("、")
        ));
    }
    let v = llm.json(SYSTEM, &prompt, &schema())?;
    #[derive(Deserialize)]
    struct Raw {
        kind: String,
        question: String,
        options: Vec<String>,
        answer: usize,
        explanation: String,
        word: String,
    }
    let raw: Vec<Raw> = serde_json::from_value(v["questions"].clone())
        .map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?;
    let mut items: Vec<QuizItem> = raw
        .into_iter()
        .filter_map(|r| {
            let mc = Mc { question: r.question.trim().to_string(), options: r.options.iter().map(|o| o.trim().to_string()).collect(), answer: r.answer };
            let cloze = r.kind == "cloze";
            let ok = valid(&mc) && (!cloze || (mc.question.contains("（　）") || mc.question.contains("（ ）")) && !r.word.trim().is_empty());
            ok.then(|| QuizItem {
                id: Uuid::new_v4().to_string(),
                kind: if cloze { "cloze".into() } else { "comprehension".into() },
                question: mc.question.replace("（ ）", "（　）"),
                options: mc.options,
                answer: mc.answer,
                explanation: r.explanation.trim().to_string(),
                word: cloze.then(|| r.word.trim().to_string()),
            })
        })
        .collect();
    // Right with the text; comprehension questions also not answerable without it.
    let mcs: Vec<Mc> = items.iter().map(|i| Mc { question: i.question.clone(), options: i.options.clone(), answer: i.answer }).collect();
    let with = solve(llm, &numbered(&mcs, Some(&text.body)))?;
    let without = solve(llm, &numbered(&mcs, None))?;
    let keep: HashSet<String> = items
        .iter()
        .enumerate()
        .filter(|(i, it)| {
            let right = with.get(i).is_some_and(|(c, amb)| *c == it.answer && !amb);
            let guessable = it.kind == "comprehension" && without.get(i).is_some_and(|(c, _)| *c == it.answer);
            right && !guessable
        })
        .map(|(_, it)| it.id.clone())
        .collect();
    items.retain(|i| keep.contains(&i.id));
    let mut rng = Rng::new();
    for it in &mut items {
        let (options, answer) = shuffled(&Mc { question: String::new(), options: it.options.clone(), answer: it.answer }, &mut rng);
        it.options = options;
        it.answer = answer;
    }
    let mut chosen: Vec<QuizItem> = items.iter().filter(|i| i.kind == "comprehension").take(COMPREHENSION).cloned().collect();
    let mut words = HashSet::new();
    chosen.extend(items.iter().filter(|i| i.kind == "cloze" && words.insert(i.word.clone())).take(CLOZES).cloned());
    let rows: Vec<(String, Option<String>, String)> =
        chosen.iter().map(|i| Ok((i.id.clone(), Some(text_id.to_string()), serde_json::to_string(i)?))).collect::<Result<_>>()?;
    crate::memory::bank_add(store, "text_quiz", "text", 0, &rows)?;
    Ok(chosen)
}

#[derive(Debug, Clone, Serialize)]
pub struct QuizResult {
    pub right: usize,
    pub total: usize,
    /// Target words of missed clozes: they come back as practice words.
    pub missed_words: Vec<String>,
}

/// Records the answers (item id → choice): missed cloze words become 'quiz_missed' events.
pub fn submit(store: &mut Store, tok: &dyn Tokenizer, text_id: &str, answers: &[(String, usize)]) -> Result<QuizResult> {
    let items = from_bank(store, text_id)?;
    let mut r = QuizResult { right: 0, total: 0, missed_words: vec![] };
    let mut events = Vec::new();
    for (id, choice) in answers {
        let Some(it) = items.iter().find(|i| &i.id == id) else { continue };
        r.total += 1;
        if *choice == it.answer {
            r.right += 1;
            continue;
        }
        if let Some(w) = &it.word {
            r.missed_words.push(w.clone());
            if let Some(key) = crate::mapping::keys_for_field(tok, w, None)?.keys.into_iter().next() {
                events.push(NewEvent { kind: "quiz_missed".into(), key, text_id: Some(text_id.into()), display: Some(w.clone()) });
            }
        }
    }
    store.record_events(&events)?;
    crate::memory::bank_used(store, &answers.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>())?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Writes questions; the checker answers 0 (the writer's key) with the text, 1 without it.
    struct Fake(Mutex<usize>);
    impl Llm for Fake {
        fn json(&self, _s: &str, u: &str, schema: &Value) -> Result<Value> {
            *self.0.lock().unwrap() += 1;
            if schema["properties"].get("answers").is_some() {
                let n = u.lines().filter(|l| l.split_once(". ").is_some_and(|(i, _)| i.parse::<usize>().is_ok())).count();
                let with = u.starts_with("Passage:");
                return Ok(json!({"answers": (0..n).map(|i| json!({"index": i, "choice": if with { 0 } else { 1 }, "ambiguous": false})).collect::<Vec<_>>()}));
            }
            Ok(json!({"questions": [
                {"kind": "comprehension", "question": "猫はどこにいましたか。", "options": ["駅", "家", "店", "山"], "answer": 0, "explanation": "At the station.", "word": ""},
                {"kind": "comprehension", "question": "だれが来ましたか。", "options": ["母", "父", "兄", "姉"], "answer": 0, "explanation": "Mother.", "word": ""},
                {"kind": "comprehension", "question": "天気は？", "options": ["雨", "晴れ", "雪", "風"], "answer": 0, "explanation": "Rain.", "word": ""},
                {"kind": "cloze", "question": "（　）を忘れた。", "options": ["傘", "猫", "駅", "本"], "answer": 0, "explanation": "傘: umbrella.", "word": "傘"},
                {"kind": "cloze", "question": "no blank here", "options": ["a", "b", "c", "d"], "answer": 0, "explanation": "", "word": "x"}
            ]}))
        }
    }

    #[test]
    fn questions_are_checked_kept_and_reused() {
        let s = Store::open_in_memory().unwrap();
        let t = s.add_text_with_extra("ja", "t", "雨の日、猫は駅にいた。傘を忘れた。", "generated", Some(&json!({"targets": ["傘"]}))).unwrap();
        let llm = Fake(Mutex::new(0));
        let q = questions(&s, &llm, &t.id).unwrap();
        assert_eq!(q.iter().map(|i| i.kind.as_str()).collect::<Vec<_>>(), ["comprehension", "comprehension", "cloze"]);
        let cloze = &q[2];
        assert_eq!(cloze.options[cloze.answer], "傘", "the key follows the shuffle");
        assert_eq!(*llm.0.lock().unwrap(), 3, "write + check with and without the text");
        // Asked again: from the bank, no AI.
        let again = questions(&s, &llm, &t.id).unwrap();
        assert_eq!(again.len(), 3);
        assert_eq!(*llm.0.lock().unwrap(), 3);
    }

    #[test]
    fn missed_cloze_words_come_back() {
        let Some(tok) = crate::tokenize::test_support::sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        let t = s.add_text_with_extra("ja", "t", "傘を忘れた。", "generated", Some(&json!({"targets": ["傘"]}))).unwrap();
        let q = questions(&s, &Fake(Mutex::new(0)), &t.id).unwrap();
        let answers: Vec<(String, usize)> = q.iter().map(|i| (i.id.clone(), if i.kind == "cloze" { (i.answer + 1) % 4 } else { i.answer })).collect();
        let r = submit(&mut s, tok, &t.id, &answers).unwrap();
        assert_eq!((r.right, r.total, r.missed_words.clone()), (2, 3, vec!["傘".to_string()]));
        // Recorded for the practice-word picker (which waits a day after a word was a target).
        let n: i64 = s.conn.query_row("SELECT count(*) FROM events WHERE kind = 'quiz_missed' AND lemma = '傘'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }
}
