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

/// The model's answer: a title, the text, and example sentences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    #[serde(default)]
    pub title: String,
    pub paragraphs: Vec<String>,
    #[serde(default)]
    pub examples: Vec<Example>,
}

impl Draft {
    fn body(&self) -> String {
        self.paragraphs.iter().map(|p| p.trim()).filter(|p| !p.is_empty()).collect::<Vec<_>>().join("\n\n")
    }
}

/// How a draft did: coverage, targets it doesn't use, and the unknown words it does.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub coverage: f64,
    pub missing: Vec<String>,
    pub unknown: Vec<String>,
}

impl Check {
    pub fn good(&self) -> bool {
        self.coverage >= TARGET_COVERAGE && self.missing.is_empty()
    }
}

/// Coverage of a text, counting target words as covered, and which targets are missing.
pub(crate) fn check(store: &Store, tok: &dyn Tokenizer, text: &str, targets: &[(String, HashSet<String>)]) -> Result<Check> {
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

/// A request worked out once: cleaned targets, the lemmas each one counts as, and the prompt.
pub struct Prepared {
    pub req: GenerateRequest,
    target_lemmas: Vec<(String, HashSet<String>)>,
    pub prompt: String,
    lang: String,
}

/// Each target's lemmas as the tokenizer sees them, so conjugated uses count.
pub(crate) fn target_lemmas(tok: &dyn Tokenizer, targets: &[String]) -> Result<Vec<(String, HashSet<String>)>> {
    let mut out = Vec::new();
    for t in targets {
        let mut lemmas: HashSet<String> = keys_for_field(tok, t, None)?.keys.into_iter().map(|k| k.lemma).collect();
        lemmas.insert(t.clone());
        out.push((t.clone(), lemmas));
    }
    Ok(out)
}

pub fn prepare(store: &Store, tok: &dyn Tokenizer, req: &GenerateRequest) -> Result<Prepared> {
    let targets: Vec<String> = req.targets.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    if targets.is_empty() {
        return Err(Error::Llm("pick at least one target word".into()));
    }
    let target_lemmas = target_lemmas(tok, &targets)?;
    let lang = tok.lang().to_string();
    let known = store.known_lemmas(&lang, KNOWN_SAMPLE)?;
    let fading: Vec<String> = store
        .study_words(30)?
        .into_iter()
        .filter(|w| w.reason == "fading" && !targets.contains(&w.key.lemma))
        .take(8)
        .map(|w| w.key.lemma)
        .collect();
    let req = GenerateRequest { targets, ..req.clone() };
    let prompt = user_prompt(&req, &known, &fading);
    Ok(Prepared { req, target_lemmas, prompt, lang })
}

pub fn check_draft(store: &Store, tok: &dyn Tokenizer, prep: &Prepared, draft: &Draft) -> Result<Check> {
    check(store, tok, &draft.body(), &prep.target_lemmas)
}

/// What to fix, for the rewrite request.
pub fn feedback(c: &Check) -> String {
    let mut f = format!(
        "It reached {:.0}% known words; it needs at least {:.0}%.",
        c.coverage * 100.0,
        TARGET_COVERAGE * 100.0
    );
    if !c.unknown.is_empty() {
        f.push_str(&format!(
            " Replace these words the learner doesn't know with simpler known words: {}.",
            c.unknown.iter().take(40).cloned().collect::<Vec<_>>().join("、")
        ));
    }
    if !c.missing.is_empty() {
        f.push_str(&format!(" It doesn't use these target words; use each of them: {}.", c.missing.join("、")));
    }
    f.push_str(" Rewrite the whole text.");
    f
}

/// Saves a draft to the library.
pub fn save(store: &Store, prep: &Prepared, draft: &Draft, c: &Check, attempts: usize) -> Result<GenerateResult> {
    let targets = &prep.req.targets;
    let extra = json!({
        "targets": targets,
        "topic": prep.req.topic,
        "coverage": c.coverage,
        "attempts": attempts,
        "examples": draft.examples,
    });
    let title = if draft.title.trim().is_empty() { targets.join("・") } else { draft.title.trim().to_string() };
    let text = store.add_text_with_extra(&prep.lang, &title, &draft.body(), "generated", Some(&extra))?;
    Ok(GenerateResult {
        text,
        coverage: c.coverage,
        attempts,
        missing_targets: c.missing.clone(),
        unknown_words: c.unknown.clone(),
        examples: draft.examples.clone(),
    })
}

/// Writes a text, checks it, retries with feedback, and saves the best attempt to the library.
pub fn generate_text(store: &Store, tok: &dyn Tokenizer, llm: &dyn Llm, req: &GenerateRequest) -> Result<GenerateResult> {
    let prep = prepare(store, tok, req)?;
    let mut best: Option<(Draft, Check)> = None;
    let mut attempts = 0;
    let mut prompt = prep.prompt.clone();
    while attempts < MAX_ATTEMPTS {
        attempts += 1;
        let draft: Draft = serde_json::from_value(llm.json(SYSTEM, &prompt, &schema())?)
            .map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?;
        let c = check_draft(store, tok, &prep, &draft)?;
        let good = c.good();
        let better = best.as_ref().is_none_or(|(_, b)| (c.missing.len(), -c.coverage) < (b.missing.len(), -b.coverage));
        if !good {
            prompt = format!("{}\nYour previous version:\n{}\n\n{}", prep.prompt, draft.body(), feedback(&c));
        }
        if better {
            best = Some((draft, c));
        }
        if good {
            break;
        }
    }
    let (draft, c) = best.expect("at least one attempt");
    save(store, &prep, &draft, &c, attempts)
}

// --- "Simplify this sentence" (ROADMAP M2) ---

const SIMPLIFY_SYSTEM: &str = "You help an adult learner read Japanese. Rewrite the sentence they found hard in \
simpler Japanese with the same meaning, using words from their known-word list and basic grammar. Keep it natural; \
you may split it into two short sentences. Keep names as they are. Also give a natural English translation of the \
original sentence.";

fn simplify_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"simple": {"type": "string"}, "translation": {"type": "string"}},
        "required": ["simple", "translation"],
        "additionalProperties": false
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct Simplified {
    /// The sentence in simpler Japanese, as the reader shows it.
    pub tokens: Vec<crate::reader::ReaderToken>,
    pub simple: String,
    /// English translation of the original sentence.
    pub translation: String,
    /// Share of words in the simple version the learner knows, 0-1.
    pub coverage: f64,
}

/// Rewrites a sentence with known words; asks once more if the first try isn't familiar enough.
pub fn simplify_sentence(store: &Store, tok: &dyn Tokenizer, llm: &dyn Llm, sentence: &str) -> Result<Simplified> {
    let sentence = sentence.trim();
    if sentence.is_empty() {
        return Err(Error::Llm("no sentence to simplify".into()));
    }
    let known = store.known_lemmas(tok.lang(), KNOWN_SAMPLE)?;
    let base = format!("Sentence: {sentence}\n\nKnown words (the learner knows these well):\n{}\n", known.join("、"));
    let mut prompt = base.clone();
    let mut best: Option<(String, String, Check)> = None;
    for _ in 0..2 {
        #[derive(Deserialize)]
        struct Answer {
            simple: String,
            translation: String,
        }
        let a: Answer = serde_json::from_value(llm.json(SIMPLIFY_SYSTEM, &prompt, &simplify_schema())?)
            .map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?;
        let c = check(store, tok, &a.simple, &[])?;
        let good = c.coverage >= TARGET_COVERAGE;
        if !good {
            prompt = format!(
                "{base}\nYour previous version: {}\nThe learner doesn't know these words in it: {}. Simplify it further.",
                a.simple,
                c.unknown.join("、")
            );
        }
        if best.as_ref().is_none_or(|(_, _, b)| c.coverage > b.coverage) {
            best = Some((a.simple.trim().to_string(), a.translation.trim().to_string(), c));
        }
        if good {
            break;
        }
    }
    let (simple, translation, c) = best.expect("at least one attempt");
    let doc = analyze(store, tok, &simple)?;
    Ok(Simplified { tokens: doc.paragraphs.into_iter().flatten().collect(), simple, translation, coverage: c.coverage })
}

// --- Copy & paste through claude.ai (ADR 0011) ---

const ANSWER_FORMAT: &str = "Answer with only a JSON object in this shape, with nothing before or after it:
{\"title\": \"a short Japanese title\", \"paragraphs\": [\"first paragraph\", \"second paragraph\"], \"examples\": [{\"word\": \"a target word\", \"sentence\": \"an example sentence\", \"translation\": \"its English translation\"}]}";

/// The whole request to paste into a chat: instructions, the prompt, and the answer format.
pub fn chat_prompt(prep: &Prepared) -> String {
    format!("{SYSTEM}\n\n{}\n{ANSWER_FORMAT}", prep.prompt)
}

/// The follow-up to paste into the same chat when a draft isn't good enough.
pub fn chat_rewrite(c: &Check) -> String {
    format!("{}\n\nAnswer with only the JSON object again, in the same shape.", feedback(c))
}

/// Reads a pasted reply: JSON, possibly in a ```json fence or with text around it. A reply
/// without JSON counts as the text itself, with a `# Title` first line as its title.
pub fn parse_answer(answer: &str) -> Result<Draft> {
    let answer = answer.trim();
    if let (Some(start), Some(end)) = (answer.find('{'), answer.rfind('}')) {
        if start < end {
            let mut v: Value = serde_json::from_str(&answer[start..=end])
                .map_err(|e| Error::Llm(format!("the reply looks like JSON but can't be read ({e}); copy the whole reply")))?;
            // A single string instead of a list of paragraphs.
            if let Some(s) = v["paragraphs"].as_str() {
                v["paragraphs"] = json!(s.split("\n\n").collect::<Vec<_>>());
            }
            return serde_json::from_value(v)
                .map_err(|e| Error::Llm(format!("the reply doesn't have the expected shape: {e}")));
        }
    }
    let mut lines = answer.lines().peekable();
    let title = match lines.peek() {
        Some(l) if l.starts_with('#') => {
            let t = l.trim_start_matches('#').trim().to_string();
            lines.next();
            t
        }
        _ => String::new(),
    };
    let body = lines.collect::<Vec<_>>().join("\n");
    let paragraphs: Vec<String> =
        body.split("\n\n").map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
    if paragraphs.is_empty() {
        return Err(Error::Llm("paste Claude's reply first".into()));
    }
    Ok(Draft { title, paragraphs, examples: vec![] })
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatOutcome {
    pub coverage: f64,
    pub missing_targets: Vec<String>,
    pub unknown_words: Vec<String>,
    /// The follow-up to paste when the draft isn't good enough and wasn't saved.
    pub rewrite: Option<String>,
    /// The saved text, when the draft was good enough or the user chose to keep it.
    pub saved: Option<GenerateResult>,
}

/// Checks a pasted reply; saves it when it's good enough or `keep` is set, otherwise returns the
/// rewrite request.
pub fn submit_answer(
    store: &Store,
    tok: &dyn Tokenizer,
    req: &GenerateRequest,
    answer: &str,
    attempts: usize,
    keep: bool,
) -> Result<ChatOutcome> {
    let prep = prepare(store, tok, req)?;
    let draft = parse_answer(answer)?;
    let c = check_draft(store, tok, &prep, &draft)?;
    let (rewrite, saved) = if c.good() || keep {
        (None, Some(save(store, &prep, &draft, &c, attempts)?))
    } else {
        (Some(chat_rewrite(&c)), None)
    };
    Ok(ChatOutcome {
        coverage: c.coverage,
        missing_targets: c.missing,
        unknown_words: c.unknown,
        rewrite,
        saved,
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
    fn parses_pasted_replies_leniently() {
        let fenced = "Here it is:\n```json\n{\"title\": \"猫\", \"paragraphs\": [\"一。\", \"二。\"], \"examples\": []}\n```\nEnjoy!";
        let d = parse_answer(fenced).unwrap();
        assert_eq!((d.title.as_str(), d.paragraphs.len()), ("猫", 2));

        let one_string = "{\"title\": \"t\", \"paragraphs\": \"一。\\n\\n二。\"}";
        assert_eq!(parse_answer(one_string).unwrap().paragraphs, vec!["一。", "二。"]);

        let plain = "# 猫の日\n\n猫が好きだ。\n\n毎日見る。\n";
        let d = parse_answer(plain).unwrap();
        assert_eq!(d.title, "猫の日");
        assert_eq!(d.paragraphs, vec!["猫が好きだ。", "毎日見る。"]);

        assert!(parse_answer("  ").is_err());
        assert!(parse_answer("{\"title\": oops}").unwrap_err().to_string().contains("JSON"));
    }

    #[test]
    fn chat_flow_asks_for_a_rewrite_then_saves() {
        let Some(t) = sudachi() else { return };
        let s = store_with_known(&[("猫", "ねこ"), ("好き", "すき"), ("毎日", "まいにち"), ("見る", "みる"), ("日本", "にっぽん")]);
        let req = GenerateRequest { targets: vec!["石油".into()], topic: "猫".into(), length: 200, polite: false };

        let prompt = chat_prompt(&prepare(&s, t, &req).unwrap());
        assert!(prompt.contains("comprehensible input") && prompt.contains("Target words: 石油") && prompt.contains("\"paragraphs\""));

        let weak = draft(&["猫が好きだ。毎日、猫を観察する。"]).to_string();
        let r = submit_answer(&s, t, &req, &weak, 1, false).unwrap();
        assert!(r.saved.is_none());
        let rewrite = r.rewrite.unwrap();
        assert!(rewrite.contains("観察") && rewrite.contains("石油") && rewrite.contains("JSON"), "{rewrite}");
        assert!(s.list_texts().unwrap().is_empty());

        let good = format!("```json\n{}\n```", draft(&["猫が好きだ。", "毎日、日本の石油を見る。"]));
        let r = submit_answer(&s, t, &req, &good, 2, false).unwrap();
        let saved = r.saved.unwrap();
        assert_eq!(saved.attempts, 2);
        assert!(r.rewrite.is_none());

        // "Use it anyway" keeps a weak draft.
        let r = submit_answer(&s, t, &req, &weak, 3, true).unwrap();
        assert!(r.saved.is_some() && r.coverage < TARGET_COVERAGE);
        assert_eq!(s.list_texts().unwrap().len(), 2);
    }

    #[test]
    fn simplifies_with_known_words() {
        let Some(t) = sudachi() else { return };
        let s = store_with_known(&[("猫", "ねこ"), ("好き", "すき"), ("見る", "みる")]);
        let llm = ScriptedLlm {
            drafts: Mutex::new(vec![
                json!({"simple": "猫を観察するのが好きだ。", "translation": "I like watching cats."}),
                json!({"simple": "猫を見るのが好きだ。", "translation": "I like watching cats."}),
            ]),
            prompts: Mutex::new(vec![]),
        };
        let r = simplify_sentence(&s, t, &llm, "猫の生態を観察することを好む。").unwrap();
        assert_eq!(r.simple, "猫を見るのが好きだ。");
        assert_eq!(r.translation, "I like watching cats.");
        assert!(r.coverage >= TARGET_COVERAGE, "{r:?}");
        assert!(r.tokens.iter().any(|x| x.surface == "猫"));
        let prompts = llm.prompts.lock().unwrap();
        assert!(prompts[0].contains("Sentence: 猫の生態を観察することを好む。") && prompts[0].contains("好き"));
        assert!(prompts[1].contains("観察"), "{}", prompts[1]);
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
