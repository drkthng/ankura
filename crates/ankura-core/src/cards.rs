//! Anki cards from reading (ADR 0012): drafts in a card list, new i+1 sentences, fields filled by
//! the AI in the style of the user's own notes, and add-only writes to Anki (ADR 0007).

use std::collections::{BTreeMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::anki::{AnkiSource, NewNote};
use crate::dict;
use crate::generate::{check, target_lemmas};
use crate::llm::Llm;
use crate::store::{CardDraft, NewCardDraft, Store};
use crate::tokenize::{TokenKind, Tokenizer};
use crate::{Error, Result, WordKey};

/// Every note Ankura adds carries this tag (ADR 0007).
pub const CARD_TAG: &str = "ankura";
const SAMPLE_NOTES: usize = 3;
const SENTENCE_ATTEMPTS: usize = 3;
const KNOWN_SAMPLE: usize = 400;
/// Dictionary entries and senses per entry given to the model when filling a card.
const MAX_ENTRIES: usize = 4;
const MAX_SENSES: usize = 8;

/// Where new cards go, chosen once in Settings.
#[derive(Debug, Clone)]
pub struct CardTarget {
    pub note_type: String,
    pub deck: String,
}

// --- sentences ---

/// Splits text into sentences, keeping the closing punctuation (and a closing quote) with each.
pub fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_string());
            }
            cur.clear();
            continue;
        }
        cur.push(c);
        if matches!(c, '。' | '！' | '？' | '!' | '?') {
            while let Some(&q) = chars.peek() {
                if matches!(q, '」' | '』' | '）' | ')') {
                    cur.push(q);
                    chars.next();
                } else {
                    break;
                }
            }
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_string());
            }
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

const SENTENCE_SYSTEM: &str = "You write one example sentence for a Japanese learner's flashcard. Use the target word \
once, in any form, in a context that makes its meaning clear. Every other word must be one the learner knows: words \
from the known-word list, or basic grammar (particles, auxiliaries, pronouns, numbers, and very common verbs such as \
する, ある, いる, なる, 言う, 思う). Keep it natural and short: one sentence, at most about 30 characters. No \
furigana, no romaji, no translation.";

fn sentence_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"sentence": {"type": "string"}},
        "required": ["sentence"],
        "additionalProperties": false
    })
}

/// One sentence around the word in which every other word is known (i+1). With `reuse`, a
/// sentence written for this word before is used if it still passes the check (ADR 0016);
/// otherwise the AI writes one, asked again with feedback when it isn't, and the best try is
/// returned and kept for later.
pub fn new_sentence(store: &Store, tok: &dyn Tokenizer, llm: &dyn Llm, key: &WordKey, display: &str, reuse: bool) -> Result<String> {
    let mut targets = target_lemmas(tok, &[display.to_string()])?;
    targets[0].1.insert(key.lemma.clone());
    let cache_key = format!("{}|{}", key.lemma, key.reading);
    if reuse {
        for v in crate::memory::cache_get(store, "card_sentence", &cache_key)? {
            let Some(sentence) = v.as_str() else { continue };
            let c = check(store, tok, sentence, &targets)?;
            if c.missing.is_empty() && c.unknown.is_empty() {
                crate::memory::reused(store, "card_sentence", 1)?;
                return Ok(sentence.to_string());
            }
        }
    }
    let known = store.known_lemmas(&key.lang, KNOWN_SAMPLE)?;
    let base = format!(
        "Target word: {display} ({})\n\nKnown words (the learner knows these well):\n{}\n",
        key.reading,
        known.join("、")
    );
    let mut prompt = base.clone();
    let mut best: Option<(String, usize)> = None;
    for _ in 0..SENTENCE_ATTEMPTS {
        #[derive(Deserialize)]
        struct Answer {
            sentence: String,
        }
        let a: Answer = serde_json::from_value(llm.json(SENTENCE_SYSTEM, &prompt, &sentence_schema())?)
            .map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?;
        let sentence = a.sentence.trim().to_string();
        let c = check(store, tok, &sentence, &targets)?;
        // Missing target counts as worse than any number of unknown words.
        let misses = c.missing.len() * 100 + c.unknown.len();
        if best.as_ref().is_none_or(|(_, b)| misses < *b) {
            best = Some((sentence.clone(), misses));
        }
        if misses == 0 {
            break;
        }
        let mut f = format!("{base}\nYour previous sentence: {sentence}\n");
        if !c.missing.is_empty() {
            f.push_str(&format!("It doesn't use the target word {display}. "));
        }
        if !c.unknown.is_empty() {
            f.push_str(&format!("The learner doesn't know these words in it: {}. ", c.unknown.join("、")));
        }
        f.push_str("Write a new sentence.");
        prompt = f;
    }
    let (sentence, misses) = best.expect("at least one attempt");
    if misses < 100 {
        crate::memory::cache_put(store, "card_sentence", &cache_key, &serde_json::Value::String(sentence.clone()))?;
    }
    Ok(sentence)
}

// --- filling the note's fields ---

const FILL_SYSTEM: &str = "You fill in a new Anki note for a Japanese learner. Copy the style of the example notes \
exactly: which information goes into which field, the HTML formatting (for example <b> or <i> around the word, <br> \
line breaks), the language of each field and the amount of detail. Build the note from the facts given; take \
readings and meanings from them rather than guessing. Words have several meanings, often spread over several \
dictionary entries: decide which one the word has in this sentence, and put that meaning on the card (shorten or \
rephrase it in the examples' style), not the first listed or the whole entry. If the examples put a sentence's \
reading in a field, use the sentence reading from the facts and fix it only where it's clearly wrong. Leave fields that hold audio or images \
([sound:...] or <img ...>) empty, and leave a field empty when the examples leave it empty.";

/// Audio and images can't be made up; drop them if the model copies the pattern anyway.
static MEDIA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[sound:[^\]]*\]|<img[^>]*>").unwrap());

/// The sentence's reading in hiragana, from the tokenizer (a hint for the model).
fn sentence_reading(tok: &dyn Tokenizer, sentence: &str) -> Result<String> {
    Ok(tok
        .tokenize(sentence)?
        .into_iter()
        .map(|t| if t.reading.is_empty() || t.kind == TokenKind::Symbol { t.surface } else { t.reading })
        .collect())
}

/// The word as it's written in the sentence (conjugated), if it's there.
fn surface_in(tok: &dyn Tokenizer, sentence: &str, key: &WordKey, display: &str) -> Result<Option<String>> {
    Ok(tok
        .tokenize(sentence)?
        .into_iter()
        .find(|t| t.lemma == key.lemma || t.dictionary_form == display || t.surface == display)
        .map(|t| t.surface))
}

fn facts(store: &Store, tok: &dyn Tokenizer, d: &CardDraft) -> Result<String> {
    let mut f = format!("Word: {}\nDictionary form: {}\nReading of the word: {}\n", d.display, d.key.lemma, d.key.reading);
    if let Some(s) = surface_in(tok, &d.sentence, &d.key, &d.display)? {
        f.push_str(&format!("The word as written in the sentence: {s}\n"));
    }
    f.push_str(&format!("Sentence: {}\n", d.sentence));
    f.push_str(&format!("Sentence reading (from a tokenizer): {}\n", sentence_reading(tok, &d.sentence)?));
    let mut entries = dict::lookup(store, &d.display, &d.key.reading)?;
    if entries.is_empty() {
        entries = dict::lookup(store, &d.key.lemma, &d.key.reading)?;
    }
    // Every entry and sense, so the model can pick the one the sentence uses (the first one often isn't).
    if !entries.is_empty() {
        f.push_str("Dictionary entries (pick the meaning that fits the sentence):\n");
        for e in entries.iter().take(MAX_ENTRIES) {
            f.push_str(&format!("{} 【{}】\n", e.expression, e.reading));
            for (i, m) in e.senses.iter().take(MAX_SENSES).enumerate() {
                f.push_str(&format!("  {}. {m}\n", i + 1));
            }
        }
    }
    Ok(f)
}

/// Fills a draft's fields for the chosen note type, copying the style of its newest notes.
pub fn fill_fields(
    store: &Store,
    tok: &dyn Tokenizer,
    llm: &dyn Llm,
    anki: &dyn AnkiSource,
    draft: &CardDraft,
    note_type: &str,
) -> Result<BTreeMap<String, String>> {
    let model_id = anki
        .note_types()?
        .into_iter()
        .find(|(name, _)| name == note_type)
        .map(|(_, id)| id)
        .ok_or_else(|| Error::Anki(format!("Anki has no note type called {note_type}; choose another in Settings")))?;
    let fields = anki.field_names(note_type)?;
    let mut ids = anki.find_notes(&format!("mid:{model_id}"))?;
    ids.sort_unstable_by(|a, b| b.cmp(a)); // note IDs are creation times: newest first
    ids.truncate(SAMPLE_NOTES);
    let samples = anki.notes_info(&ids)?;

    let mut prompt = format!("Note type: {note_type}\nFields, in order: {}\n", fields.join(", "));
    if samples.is_empty() {
        prompt.push_str("\nThere are no example notes yet; fill the fields as their names suggest.\n");
    }
    for (i, n) in samples.iter().enumerate() {
        prompt.push_str(&format!("\nExample note {}:\n", i + 1));
        for f in &fields {
            prompt.push_str(&format!("- {f}: {}\n", n.fields.get(f).map(String::as_str).unwrap_or("")));
        }
    }
    prompt.push_str(&format!("\nFacts for the new note:\n{}", facts(store, tok, draft)?));

    let props: serde_json::Map<String, Value> = fields.iter().map(|f| (f.clone(), json!({"type": "string"}))).collect();
    let schema = json!({"type": "object", "properties": props, "required": fields, "additionalProperties": false});
    let v = llm.json(FILL_SYSTEM, &prompt, &schema)?;
    Ok(fields
        .iter()
        .map(|f| {
            let value = v[f].as_str().unwrap_or("");
            (f.clone(), MEDIA.replace_all(value, "").trim().to_string())
        })
        .collect())
}

// --- drafts and adding ---

/// Drafts for a generated text's target words, each with the first sentence of the text that uses
/// it (or the example sentence). Skips words that already have a draft.
pub fn drafts_for_text(store: &Store, tok: &dyn Tokenizer, text_id: &str) -> Result<Vec<CardDraft>> {
    let text = store.open_text(text_id)?.ok_or_else(|| Error::Anki("this text no longer exists".into()))?;
    let extra = text.extra.unwrap_or(Value::Null);
    let targets: Vec<String> =
        extra["targets"].as_array().into_iter().flatten().filter_map(|t| t.as_str().map(String::from)).collect();
    let pending: HashSet<String> =
        store.card_drafts()?.into_iter().filter(|d| d.status == "draft").map(|d| d.key.lemma).collect();
    let all = sentences(&text.body);
    let mut out = Vec::new();
    for (target, lemmas) in target_lemmas(tok, &targets)? {
        let found = all.iter().find_map(|s| {
            let toks = tok.tokenize(s).ok()?;
            let t = toks.into_iter().find(|t| lemmas.contains(&t.lemma) || lemmas.contains(&t.dictionary_form))?;
            Some((s.clone(), WordKey { lang: text.meta.lang.clone(), lemma: t.lemma, reading: t.lemma_reading }))
        });
        let (sentence, key) = match found {
            Some(f) => f,
            None => {
                let example = extra["examples"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|e| e["word"] == target.as_str())
                    .and_then(|e| e["sentence"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let t = tok.tokenize(&target)?.into_iter().find(|t| t.kind == TokenKind::Word);
                let key = WordKey {
                    lang: text.meta.lang.clone(),
                    lemma: t.as_ref().map(|t| t.lemma.clone()).unwrap_or(target.clone()),
                    reading: t.map(|t| t.lemma_reading).unwrap_or_default(),
                };
                (example, key)
            }
        };
        if sentence.is_empty() || pending.contains(&key.lemma) {
            continue;
        }
        out.push(store.add_card_draft(&NewCardDraft {
            key,
            display: target,
            sentence,
            sentence_source: "text".into(),
            text_id: Some(text_id.to_string()),
        })?);
    }
    Ok(out)
}

/// Sends a filled draft to Anki and marks it added.
pub fn add_to_anki(store: &Store, anki: &dyn AnkiSource, draft_id: &str, target: &CardTarget) -> Result<i64> {
    let d = store.card_draft(draft_id)?.ok_or_else(|| Error::Anki("this card is no longer in the list".into()))?;
    if d.status == "added" {
        return d.note_id.ok_or_else(|| Error::Anki("already added".into()));
    }
    let fields = d.fields.ok_or_else(|| Error::Anki("fill in the card's fields first".into()))?;
    let note_id = anki.add_note(&NewNote {
        deck: target.deck.clone(),
        note_type: target.note_type.clone(),
        fields,
        tags: vec![CARD_TAG.into()],
    })?;
    store.mark_card_added(draft_id, note_id)?;
    Ok(note_id)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::anki::fake::{FakeCard, FakeNote, serve_logged};
    use crate::anki::AnkiConnect;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::tokenize::test_support::sudachi;

    struct ScriptedLlm {
        answers: Mutex<Vec<Value>>,
        prompts: Mutex<Vec<(String, Value)>>,
    }

    impl ScriptedLlm {
        fn new(answers: Vec<Value>) -> Self {
            ScriptedLlm { answers: Mutex::new(answers), prompts: Mutex::new(vec![]) }
        }
    }

    impl Llm for ScriptedLlm {
        fn json(&self, _system: &str, user: &str, schema: &Value) -> Result<Value> {
            self.prompts.lock().unwrap().push((user.to_string(), schema.clone()));
            Ok(self.answers.lock().unwrap().remove(0))
        }
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

    fn kasa() -> WordKey {
        WordKey { lang: "ja".into(), lemma: "傘".into(), reading: "かさ".into() }
    }

    #[test]
    fn splits_sentences() {
        assert_eq!(
            sentences("雨だ。「傘は？」と聞いた。\n家に帰った"),
            vec!["雨だ。", "「傘は？」", "と聞いた。", "家に帰った"]
        );
    }

    #[test]
    fn new_sentences_use_only_known_words() {
        let Some(t) = sudachi() else { return };
        let s = store_with_known(&[("雨", "あめ"), ("日", "ひ"), ("忘れる", "わすれる")]);
        let llm = ScriptedLlm::new(vec![
            json!({"sentence": "豪雨の日に傘を忘れた。"}), // 豪雨 is unknown
            json!({"sentence": "雨の日に傘を忘れた。"}),
            json!({"sentence": "日に傘を忘れた。"}),
        ]);
        assert_eq!(new_sentence(&s, t, &llm, &kasa(), "傘", true).unwrap(), "雨の日に傘を忘れた。");
        let prompts = llm.prompts.lock().unwrap();
        assert!(prompts[0].0.contains("Target word: 傘 (かさ)") && prompts[0].0.contains("忘れる"));
        assert!(prompts[1].0.contains("豪雨"), "{}", prompts[1].0);
        drop(prompts);
        // Asked again for the same word: the kept sentence, no AI (the script is empty now).
        assert_eq!(new_sentence(&s, t, &llm, &kasa(), "傘", true).unwrap(), "雨の日に傘を忘れた。");
        assert_eq!(llm.prompts.lock().unwrap().len(), 2);
        assert_eq!(new_sentence(&s, t, &llm, &kasa(), "傘", false).unwrap(), "日に傘を忘れた。", "a fresh one asks the AI");
    }

    #[test]
    fn fills_fields_like_the_users_notes_and_adds_them() {
        let Some(t) = sudachi() else { return };
        let s = Store::open_in_memory().unwrap();
        let card = || vec![FakeCard { id: 1, interval: 30, flags: &["review"], stability: None, retrievability: None, overdue: 0 }];
        let note = |id, sentence, reading| FakeNote {
            id,
            model: ("Japanisch", 7),
            fields: vec![("Original Sentence", sentence), ("Reading", reading), ("Definition", "x"), ("Audio", "[sound:a.mp3]")],
            cards: card(),
        };
        let (url, added) = serve_logged(vec![
            note(1, "<b>古い</b>本", "ふるい<br>ふるいほん"),
            note(5, "<b>猫</b>がいる", "ねこ<br>ねこがいる"),
            note(3, "<b>犬</b>だ", "いぬ<br>いぬだ"),
            note(2, "<b>空</b>", "そら"),
        ]);
        let anki = AnkiConnect::new(&url);
        let d = s
            .add_card_draft(&NewCardDraft {
                key: kasa(),
                display: "傘".into(),
                sentence: "傘を忘れた。".into(),
                sentence_source: "text".into(),
                text_id: None,
            })
            .unwrap();
        let llm = ScriptedLlm::new(vec![json!({
            "Original Sentence": "<b>傘</b>を忘れた。",
            "Reading": "かさ<br>かさをわすれた。",
            "Definition": "umbrella",
            "Audio": "[sound:kasa.mp3]"
        })]);
        let fields = fill_fields(&s, t, &llm, &anki, &d, "Japanisch").unwrap();
        assert_eq!(fields["Original Sentence"], "<b>傘</b>を忘れた。");
        assert_eq!(fields["Audio"], "", "media is dropped");
        let (prompt, schema) = &llm.prompts.lock().unwrap()[0];
        // The three newest notes are the examples.
        assert!(prompt.contains("<b>猫</b>") && prompt.contains("<b>犬</b>") && prompt.contains("<b>空</b>"), "{prompt}");
        assert!(!prompt.contains("<b>古い</b>"), "{prompt}");
        assert!(prompt.contains("Sentence reading (from a tokenizer): かさをわすれた。"), "{prompt}");
        assert!(prompt.contains("The word as written in the sentence: 傘"), "{prompt}");
        assert_eq!(schema["required"][0], "Original Sentence");

        let target = CardTarget { note_type: "Japanisch".into(), deck: "Japanisch".into() };
        assert!(add_to_anki(&s, &anki, &d.id, &target).unwrap_err().to_string().contains("fill"));
        s.set_card_fields(&d.id, &fields).unwrap();
        let note_id = add_to_anki(&s, &anki, &d.id, &target).unwrap();
        assert_eq!(s.card_draft(&d.id).unwrap().unwrap().note_id, Some(note_id));
        let log = added.lock().unwrap();
        assert_eq!(log[0]["deckName"], "Japanisch");
        assert_eq!(log[0]["tags"][0], CARD_TAG);
        assert_eq!(log[0]["fields"]["Reading"], "かさ<br>かさをわすれた。");
    }

    #[test]
    fn the_model_sees_every_meaning_and_picks_by_context() {
        let Some(t) = sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("d.zip");
        crate::dict::write_zip(&zip, &[("term_bank_1.json", json!([
            ["気になる", "きになる", "exp", "", 100, ["to weigh on one's mind", "to be curious about"], 1, ""],
            ["気になる", "きになる", "exp", "", 50, ["to be interested in"], 2, ""]
        ]))]);
        crate::dict::import_yomitan_zip(&mut s, &zip).unwrap();
        let d = s
            .add_card_draft(&NewCardDraft {
                key: WordKey { lang: "ja".into(), lemma: "気になる".into(), reading: "きになる".into() },
                display: "気になる".into(),
                sentence: "世界の歴史が、すごく気になります。".into(),
                sentence_source: "text".into(),
                text_id: None,
            })
            .unwrap();
        let f = facts(&s, t, &d).unwrap();
        assert!(f.contains("pick the meaning that fits the sentence"), "{f}");
        assert!(f.contains("2. to be curious about") && f.contains("1. to be interested in"), "{f}");
        assert!(FILL_SYSTEM.contains("decide which one the word has in this sentence"));
    }

    #[test]
    fn drafts_for_a_texts_targets() {
        let Some(t) = sudachi() else { return };
        let s = Store::open_in_memory().unwrap();
        let extra = json!({
            "targets": ["傘", "忘れる", "石油"],
            "examples": [{"word": "石油", "sentence": "石油は高い。", "translation": "Oil is expensive."}]
        });
        let text = s.add_text_with_extra("ja", "雨", "雨が降った。\n\n傘を忘れたので、走った。", "generated", Some(&extra)).unwrap();
        let drafts = drafts_for_text(&s, t, &text.id).unwrap();
        let got: Vec<(&str, &str)> = drafts.iter().map(|d| (d.display.as_str(), d.sentence.as_str())).collect();
        assert_eq!(got, vec![("傘", "傘を忘れたので、走った。"), ("忘れる", "傘を忘れたので、走った。"), ("石油", "石油は高い。")]);
        assert_eq!(drafts[1].key.reading, "わすれる");
        // Asking again doesn't duplicate them.
        assert!(drafts_for_text(&s, t, &text.id).unwrap().is_empty());
    }
}
