//! Help with a sentence and a word while reading (ADR 0020): a translation of the sentence as it
//! is meant in its context, with what the clicked word means there; and an explanation of the word
//! in simple Japanese, in known words where possible. Both are kept (ADR 0016).

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::generate::check;
use crate::llm::Llm;
use crate::reader::{ReaderToken, analyze};
use crate::store::Store;
use crate::tokenize::Tokenizer;
use crate::{Error, Result, WordKey};

const KNOWN_SAMPLE: usize = 400;
/// Share of known words the Japanese explanation should reach.
const EXPLAIN_COVERAGE: f64 = 0.9;

// --- translation in context ---

const TRANSLATE_SYSTEM: &str = "You help an adult learner read Japanese. Translate the sentence into natural English \
as it is meant in its context: who does what (fill in subjects Japanese leaves out), what refers to what, the tone. \
Translate only that sentence, not the text around it. If a word is given, also say in one short line what that \
word means here and, when it helps, how it is used (for example a grammar pattern it is part of), the way you would \
put it on a flashcard.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Translation {
    pub translation: String,
    /// What the clicked word means in this sentence; empty without a word.
    pub word_note: String,
}

/// The sentence's translation in its context (`context`: the paragraph around it), with a note on
/// `word` when given. Kept by sentence and word.
pub fn translate(store: &Store, llm: &dyn Llm, sentence: &str, context: &str, word: Option<(&WordKey, &str)>) -> Result<Translation> {
    let sentence = sentence.trim();
    if sentence.is_empty() {
        return Err(Error::Llm("no sentence to translate".into()));
    }
    let cache_key = format!("{sentence}|{}", word.map(|(k, _)| format!("{}|{}", k.lemma, k.reading)).unwrap_or_default());
    if let Some(v) = crate::memory::cache_get(store, "translate", &cache_key)?.into_iter().next() {
        if let Ok(t) = serde_json::from_value::<Translation>(v) {
            crate::memory::reused(store, "translate", 1)?;
            return Ok(t);
        }
    }
    let mut prompt = String::new();
    if !context.trim().is_empty() && context.trim() != sentence {
        prompt.push_str(&format!("The text around it:\n{}\n\n", context.trim()));
    }
    prompt.push_str(&format!("Sentence to translate: {sentence}\n"));
    if let Some((k, display)) = word {
        prompt.push_str(&format!("Word: {display} (dictionary form {}, read {})\n", k.lemma, k.reading));
    } else {
        prompt.push_str("No word: leave word_note empty.\n");
    }
    let schema = json!({
        "type": "object",
        "properties": {"translation": {"type": "string"}, "word_note": {"type": "string"}},
        "required": ["translation", "word_note"],
        "additionalProperties": false
    });
    let v = llm.json(TRANSLATE_SYSTEM, &prompt, &schema)?;
    let t = Translation {
        translation: v["translation"].as_str().unwrap_or_default().trim().to_string(),
        word_note: v["word_note"].as_str().unwrap_or_default().trim().to_string(),
    };
    if t.translation.is_empty() {
        return Err(Error::Llm("the AI gave no translation; try again".into()));
    }
    crate::memory::cache_put(store, "translate", &cache_key, &serde_json::to_value(&t)?)?;
    Ok(t)
}

// --- the word in simple Japanese ---

const EXPLAIN_SYSTEM: &str = "You write entries for a monolingual Japanese learner's dictionary. Explain in simple \
Japanese (やさしい日本語) what the word means in the given sentence: one or two short sentences, as a learner's \
dictionary would (〜こと。〜という意味。), using very common words and the learner's known words. Don't explain \
it with the word itself or with harder words. Plain style (だ・である) or a dictionary style is fine. No English, no \
readings in brackets.";

#[derive(Debug, Clone, Serialize)]
pub struct JaExplanation {
    pub text: String,
    /// The explanation as the reader shows text: words with furigana and their state.
    pub tokens: Vec<ReaderToken>,
    /// Share of words in it the learner knows, 0-1.
    pub coverage: f64,
}

fn explanation(store: &Store, tok: &dyn Tokenizer, text: &str, word: &WordKey) -> Result<JaExplanation> {
    let targets = vec![(word.lemma.clone(), [word.lemma.clone()].into_iter().collect())];
    let c = check(store, tok, text, &targets)?;
    let doc = analyze(store, tok, text)?;
    Ok(JaExplanation { text: text.to_string(), tokens: doc.paragraphs.into_iter().flatten().collect(), coverage: c.coverage })
}

/// The word's meaning in this sentence, explained in simple Japanese. Asks once more with the
/// unknown words when too few words are known; kept by word and sentence.
pub fn explain_ja(store: &Store, tok: &dyn Tokenizer, llm: &dyn Llm, word: &WordKey, display: &str, sentence: &str) -> Result<JaExplanation> {
    let cache_key = format!("{}|{}|{}", word.lemma, word.reading, sentence.trim());
    if let Some(text) = crate::memory::cache_get(store, "explain_ja", &cache_key)?.into_iter().next().and_then(|v| v.as_str().map(String::from)) {
        crate::memory::reused(store, "explain_ja", 1)?;
        return explanation(store, tok, &text, word);
    }
    let known = store.known_lemmas(&word.lang, KNOWN_SAMPLE)?;
    let mut base = format!("Word: {display} (dictionary form {}, read {})\n", word.lemma, word.reading);
    if !sentence.trim().is_empty() {
        base.push_str(&format!("Sentence: {}\n", sentence.trim()));
    }
    base.push_str(&format!("\nKnown words (the learner knows these well):\n{}\n", known.join("、")));
    let schema = json!({
        "type": "object",
        "properties": {"explanation": {"type": "string"}},
        "required": ["explanation"],
        "additionalProperties": false
    });
    let mut prompt = base.clone();
    let mut best: Option<JaExplanation> = None;
    for _ in 0..2 {
        let v: Value = llm.json(EXPLAIN_SYSTEM, &prompt, &schema)?;
        let text = v["explanation"].as_str().unwrap_or_default().trim().to_string();
        if text.is_empty() {
            continue;
        }
        let e = explanation(store, tok, &text, word)?;
        let good = e.coverage >= EXPLAIN_COVERAGE;
        if !good {
            let unknown = check(store, tok, &text, &[])?.unknown;
            prompt = format!("{base}\nYour previous explanation: {text}\nThe learner doesn't know these words in it: {}. Explain it again with simpler words.", unknown.join("、"));
        }
        if best.as_ref().is_none_or(|b| e.coverage > b.coverage) {
            best = Some(e);
        }
        if good {
            break;
        }
    }
    let best = best.ok_or_else(|| Error::Llm("the AI gave no explanation; try again".into()))?;
    crate::memory::cache_put(store, "explain_ja", &cache_key, &Value::String(best.text.clone()))?;
    Ok(best)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
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

    fn key(l: &str, r: &str) -> WordKey {
        WordKey { lang: "ja".into(), lemma: l.into(), reading: r.into() }
    }

    #[test]
    fn translates_in_context_once() {
        let s = Store::open_in_memory().unwrap();
        let llm = Scripted(Mutex::new(vec![json!({"translation": "She didn't come after all.", "word_note": "結局: in the end"})]), Mutex::new(vec![]));
        let k = key("結局", "けっきょく");
        let t = translate(&s, &llm, "結局来なかった。", "姉を待っていた。結局来なかった。", Some((&k, "結局"))).unwrap();
        assert_eq!(t.translation, "She didn't come after all.");
        let prompts = llm.1.lock().unwrap().clone();
        assert!(prompts[0].contains("The text around it:\n姉を待っていた。") && prompts[0].contains("Word: 結局"), "{}", prompts[0]);
        // Again: kept, no AI (the script is empty now).
        assert_eq!(translate(&s, &llm, "結局来なかった。", "", Some((&k, "結局"))).unwrap(), t);
    }

    #[test]
    fn explains_in_known_words() {
        let Some(tok) = crate::tokenize::test_support::sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        let known = [("最後", "さいご"), ("こと", "こと"), ("意味", "いみ")];
        let cards: Vec<(i64, CardFacts)> = (0..known.len() as i64)
            .map(|i| (1, CardFacts { card_id: i, note_id: i, card_type: 2, queue: 2, interval: 100, lapses: 0, stability: None, retrievability: Some(0.95) }))
            .collect();
        let keys: Vec<(i64, WordKey)> = known.iter().enumerate().map(|(i, (l, r))| (i as i64, key(l, r))).collect();
        s.replace_anki_snapshot(&cards, &keys, &HashMap::new(), &KnowledgeSettings::default()).unwrap();
        let llm = Scripted(
            Mutex::new(vec![
                json!({"explanation": "いろいろ検討した帰結という意味。"}), // hard words
                json!({"explanation": "最後にという意味。"}),
            ]),
            Mutex::new(vec![]),
        );
        let k = key("結局", "けっきょく");
        let e = explain_ja(&s, tok, &llm, &k, "結局", "結局来なかった。").unwrap();
        assert_eq!(e.text, "最後にという意味。");
        assert!(e.coverage >= EXPLAIN_COVERAGE, "{}", e.coverage);
        assert!(llm.1.lock().unwrap()[1].contains("検討"), "asked again with the unknown words");
        // Kept: no AI the second time.
        assert_eq!(explain_ja(&s, tok, &llm, &k, "結局", "結局来なかった。").unwrap().text, e.text);
    }
}
