//! Generated reading texts (ROADMAP M2, architecture-options §6): the model writes a text around
//! target words, Ankura re-tokenizes it and checks coverage, and asks for a rewrite when too many
//! words are unknown or a target is missing.

use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::llm::Llm;
use crate::mapping::keys_for_field;
use crate::reader::analyze;
use crate::store::{Store, TextMeta};
use crate::tokenize::{TokenKind, Tokenizer};
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// Words the text should use, as the user typed or picked them (dictionary forms).
    pub targets: Vec<String>,
    /// What the text should be about; empty lets the model choose.
    pub topic: String,
    /// Rough length in characters.
    pub length: usize,
    /// です/ます style instead of plain style.
    pub polite: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Example {
    pub word: String,
    pub sentence: String,
    pub translation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResult {
    pub text: TextMeta,
    /// Share of word tokens that are known or targets, 0-1.
    pub coverage: f64,
    pub attempts: usize,
    pub missing_targets: Vec<String>,
    pub unknown_words: Vec<String>,
    pub examples: Vec<Example>,
}

/// Coverage the text should reach (architecture-options §6).
pub const TARGET_COVERAGE: f64 = 0.95;
const MAX_ATTEMPTS: usize = 3;
const KNOWN_SAMPLE: usize = 400;

const SYSTEM: &str = "You write short Japanese reading texts for one adult learner, who reads them in an app with a \
pop-up dictionary. The goal is comprehensible input: almost every word should be one the learner already knows, so \
that the few target words can be learned from context.

Write natural, engaging Japanese: a short story, a scene, a diary entry, a letter or a short article, whatever suits \
the topic. Use every target word at least once, in any form (conjugated is fine), in a context that makes its meaning \
guessable. Apart from the target words, use words from the known-word list, plus basic grammar words (particles, \
auxiliaries, pronouns, numbers, and very common verbs such as する, ある, いる, なる, 言う, 思う). Avoid rare words, \
long compound nouns and names that need explaining. Write the text only: no furigana, no romaji, no translations, no \
notes.

Also write one short, natural example sentence for each target word, with an English translation.";

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title": {"type": "string", "description": "A short Japanese title"},
            "paragraphs": {"type": "array", "items": {"type": "string"}},
            "examples": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "word": {"type": "string"},
                        "sentence": {"type": "string"},
                        "translation": {"type": "string"}
                    },
                    "required": ["word", "sentence", "translation"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["title", "paragraphs", "examples"],
        "additionalProperties": false
    })
}

#[derive(Debug, Clone, Deserialize)]
struct Draft {
    title: String,
    paragraphs: Vec<String>,
    examples: Vec<Example>,
}

#[derive(Debug, Clone)]
struct Check {
    coverage: f64,
    missing: Vec<String>,
    unknown: Vec<String>,
}

/// Coverage of a text, counting target words as covered, and which targets are missing.
fn check(store: &Store, tok: &dyn Tokenizer, text: &str, targets: &[(String, HashSet<String>)]) -> Result<Check> {
    let doc = analyze(store, tok, text)?;
    let all_targets: HashSet<&String> = targets.iter().flat_map(|(_, lemmas)| lemmas).collect();
    let (mut words, mut covered) = (0usize, 0usize);
    let mut seen_lemmas: HashSet<String> = HashSet::new();
    let mut unknown: BTreeSet<String> = BTreeSet::new();
    for t in doc.paragraphs.iter().flatten() {
        let Some(key) = &t.key else { continue };
        seen_lemmas.insert(key.lemma.clone());
        seen_lemmas.insert(t.dictionary_form.clone());
        if t.kind != TokenKind::Word {
            continue;
        }
        words += 1;
        if t.state.is_known() || all_targets.contains(&key.lemma) || all_targets.contains(&t.dictionary_form) {
            covered += 1;
        } else {
            unknown.insert(t.dictionary_form.clone());
        }
    }
    let missing = targets
        .iter()
        .filter(|(_, lemmas)| !lemmas.iter().any(|l| seen_lemmas.contains(l)))
        .map(|(t, _)| t.clone())
        .collect();
    let coverage = if words == 0 { 0.0 } else { covered as f64 / words as f64 };
    Ok(Check { coverage, missing, unknown: unknown.into_iter().collect() })
}

fn user_prompt(req: &GenerateRequest, known: &[String], fading: &[String]) -> String {
    let mut p = String::new();
    p.push_str(&format!("Target words: {}\n", req.targets.join("、")));
    p.push_str(&format!(
        "Topic: {}\n",
        if req.topic.trim().is_empty() { "your choice, something an adult would enjoy reading" } else { req.topic.trim() }
    ));
    p.push_str(&format!("Length: about {} Japanese characters.\n", req.length));
    p.push_str(if req.polite { "Style: です/ます.\n" } else { "Style: plain form (だ/である).\n" });
    if !fading.is_empty() {
        p.push_str(&format!(
            "Words the learner is starting to forget; use a few of them if they fit naturally: {}\n",
            fading.join("、")
        ));
    }
    p.push_str(&format!(
        "\nKnown words (the {} most familiar; the learner knows these well):\n{}\n",
        known.len(),
        known.join("、")
    ));
    p
}

/// Writes a text, checks it, retries with feedback, and saves the best attempt to the library.
pub fn generate_text(store: &Store, tok: &dyn Tokenizer, llm: &dyn Llm, req: &GenerateRequest) -> Result<GenerateResult> {
    let targets: Vec<String> = req.targets.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    if targets.is_empty() {
        return Err(Error::Llm("pick at least one target word".into()));
    }
    // Each target's lemmas as the tokenizer sees them, so conjugated uses count.
    let mut target_lemmas = Vec::new();
    for t in &targets {
        let mut lemmas: HashSet<String> = keys_for_field(tok, t, None)?.keys.into_iter().map(|k| k.lemma).collect();
        lemmas.insert(t.clone());
        target_lemmas.push((t.clone(), lemmas));
    }
    let lang = tok.lang();
    let known = store.known_lemmas(lang, KNOWN_SAMPLE)?;
    let fading: Vec<String> = store
        .study_words(30)?
        .into_iter()
        .filter(|w| w.reason == "fading" && !targets.contains(&w.key.lemma))
        .take(8)
        .map(|w| w.key.lemma)
        .collect();
    let req = GenerateRequest { targets: targets.clone(), ..req.clone() };
    let base_prompt = user_prompt(&req, &known, &fading);

    let mut best: Option<(Draft, Check)> = None;
    let mut attempts = 0;
    let mut prompt = base_prompt.clone();
    while attempts < MAX_ATTEMPTS {
        attempts += 1;
        let draft: Draft = serde_json::from_value(llm.json(SYSTEM, &prompt, &schema())?)
            .map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?;
        let text = draft.paragraphs.join("\n\n");
        let c = check(store, tok, &text, &target_lemmas)?;
        let good = c.coverage >= TARGET_COVERAGE && c.missing.is_empty();
        let better = best.as_ref().is_none_or(|(_, b)| (c.missing.len(), -c.coverage) < (b.missing.len(), -b.coverage));
        if better {
            best = Some((draft.clone(), c.clone()));
        }
        if good {
            break;
        }
        let mut feedback = format!(
            "{base_prompt}\nYour previous version:\n{text}\n\nIt reached {:.0}% known words; it needs at least {:.0}%.",
            c.coverage * 100.0,
            TARGET_COVERAGE * 100.0
        );
        if !c.unknown.is_empty() {
            feedback.push_str(&format!(
                " Replace these words the learner doesn't know with simpler known words: {}.",
                c.unknown.iter().take(40).cloned().collect::<Vec<_>>().join("、")
            ));
        }
        if !c.missing.is_empty() {
            feedback.push_str(&format!(" It doesn't use these target words; use each of them: {}.", c.missing.join("、")));
        }
        feedback.push_str(" Rewrite the whole text.");
        prompt = feedback;
    }

    let (draft, c) = best.expect("at least one attempt");
    let body = draft.paragraphs.join("\n\n");
    let extra = json!({
        "targets": targets,
        "topic": req.topic,
        "coverage": c.coverage,
        "attempts": attempts,
        "examples": draft.examples,
    });
    let title = if draft.title.trim().is_empty() { targets.join("・") } else { draft.title.trim().to_string() };
    let text = store.add_text_with_extra(lang, &title, &body, "generated", Some(&extra))?;
    Ok(GenerateResult {
        text,
        coverage: c.coverage,
        attempts,
        missing_targets: c.missing,
        unknown_words: c.unknown,
        examples: draft.examples,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::WordKey;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::tokenize::test_support::sudachi;

    /// Returns canned drafts in order and records the prompts it was given.
    struct ScriptedLlm {
        drafts: Mutex<Vec<Value>>,
        prompts: Mutex<Vec<String>>,
    }

    impl Llm for ScriptedLlm {
        fn json(&self, _system: &str, user: &str, _schema: &Value) -> Result<Value> {
            self.prompts.lock().unwrap().push(user.to_string());
            Ok(self.drafts.lock().unwrap().remove(0))
        }
    }

    fn draft(paragraphs: &[&str]) -> Value {
        json!({"title": "テスト", "paragraphs": paragraphs, "examples": [{"word": "石油", "sentence": "石油は高い。", "translation": "Oil is expensive."}]})
    }

    fn store_with_known(words: &[(&str, &str)]) -> Store {
        let mut s = Store::open_in_memory().unwrap();
        let cards: Vec<(i64, CardFacts)> = (0..words.len() as i64)
            .map(|i| {
                (1, CardFacts { card_id: i, note_id: i, card_type: 2, queue: 2, interval: 100, lapses: 0, stability: None, retrievability: None })
            })
            .collect();
        let keys: Vec<(i64, WordKey)> = words
            .iter()
            .enumerate()
            .map(|(i, (l, r))| (i as i64, WordKey { lang: "ja".into(), lemma: l.to_string(), reading: r.to_string() }))
            .collect();
        s.replace_anki_snapshot(&cards, &keys, &std::collections::HashMap::new(), &KnowledgeSettings::default()).unwrap();
        s
    }

    #[test]
    fn retries_until_coverage_and_targets_are_met() {
        let Some(t) = sudachi() else { return };
        let s = store_with_known(&[("猫", "ねこ"), ("好き", "すき"), ("毎日", "まいにち"), ("見る", "みる"), ("日本", "にっぽん")]);
        let llm = ScriptedLlm {
            drafts: Mutex::new(vec![
                // Misses the target and uses an unknown word.
                draft(&["猫が好きだ。毎日、猫を観察する。"]),
                // Uses the target (conjugation-free) and only known words.
                draft(&["猫が好きだ。", "毎日、日本の石油を見る。"]),
            ]),
            prompts: Mutex::new(vec![]),
        };
        let req = GenerateRequest { targets: vec!["石油".into()], topic: "猫".into(), length: 200, polite: false };
        let r = generate_text(&s, t, &llm, &req).unwrap();
        assert_eq!(r.attempts, 2);
        assert!(r.missing_targets.is_empty(), "{r:?}");
        assert!(r.coverage >= TARGET_COVERAGE, "{r:?}");
        assert_eq!(r.examples[0].word, "石油");

        let prompts = llm.prompts.lock().unwrap();
        assert!(prompts[0].contains("Target words: 石油") && prompts[0].contains("猫"));
        assert!(prompts[1].contains("観察") && prompts[1].contains("石油"), "{}", prompts[1]);

        let saved = s.open_text(&r.text.id).unwrap().unwrap();
        assert_eq!(saved.meta.source, "generated");
        assert_eq!(saved.body, "猫が好きだ。\n\n毎日、日本の石油を見る。");
        assert_eq!(saved.extra.unwrap()["targets"][0], "石油");
    }

    #[test]
    fn keeps_the_best_attempt_when_none_is_good_enough() {
        let Some(t) = sudachi() else { return };
        let s = store_with_known(&[("猫", "ねこ")]);
        let llm = ScriptedLlm {
            drafts: Mutex::new(vec![
                // Target used, but half the words are unknown.
                draft(&["石油の価格が急騰した。"]),
                // Target used, one unknown word: the best of the three.
                draft(&["猫と石油を分析した。"]),
                // No target at all.
                draft(&["経済学者が分析した。"]),
            ]),
            prompts: Mutex::new(vec![]),
        };
        let req = GenerateRequest { targets: vec!["石油".into()], topic: String::new(), length: 100, polite: true };
        let r = generate_text(&s, t, &llm, &req).unwrap();
        assert_eq!(r.attempts, 3);
        assert_eq!(s.open_text(&r.text.id).unwrap().unwrap().body, "猫と石油を分析した。");
        assert!(r.coverage < TARGET_COVERAGE && r.missing_targets.is_empty(), "{r:?}");
        assert!(llm.prompts.lock().unwrap()[0].contains("です/ます"));
    }

    #[test]
    fn needs_a_target() {
        let Some(t) = sudachi() else { return };
        let s = Store::open_in_memory().unwrap();
        let llm = ScriptedLlm { drafts: Mutex::new(vec![]), prompts: Mutex::new(vec![]) };
        let req = GenerateRequest { targets: vec![" ".into()], topic: String::new(), length: 100, polite: false };
        assert!(generate_text(&s, t, &llm, &req).is_err());
    }
}
