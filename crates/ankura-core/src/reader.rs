//! A text as the reader shows it: paragraphs of tokens with furigana, word state and coverage.

use serde::{Deserialize, Serialize};

use crate::furigana::{Segment, segments};
use crate::kana::has_kanji;
use crate::knowledge::WordState;
use crate::store::Store;
use crate::tokenize::{TokenKind, Tokenizer};
use crate::{Result, WordKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderToken {
    pub surface: String,
    /// Dictionary form as written in the text (ぬいぐるみ), for display; `key.lemma` is the
    /// normalized spelling (縫い包み) used for matching.
    pub dictionary_form: String,
    pub segments: Vec<Segment>,
    pub kind: TokenKind,
    /// Set for words and names.
    pub key: Option<WordKey>,
    pub state: WordState,
    /// Where the state came from: "mark", "anki", "parts" (all parts of a compound known) or "none".
    pub source: String,
    pub has_kanji: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Coverage {
    /// Counted word tokens (not grammar, names, numbers or symbols).
    pub words: usize,
    pub known: usize,
    pub learning: usize,
    pub new: usize,
    pub unknown: usize,
    /// known / words, 0-1.
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderDoc {
    pub paragraphs: Vec<Vec<ReaderToken>>,
    pub coverage: Coverage,
}

fn state_of(store: &Store, key: &WordKey) -> Result<(WordState, &'static str)> {
    if let Some(s) = store.word_override(key)? {
        return Ok((s, "mark"));
    }
    match store.knowledge(key)? {
        Some(k) if k.state != WordState::Unknown => Ok((k.state, "anki")),
        _ => Ok((WordState::Unknown, "none")),
    }
}

pub fn analyze(store: &Store, tok: &dyn Tokenizer, text: &str) -> Result<ReaderDoc> {
    let mut paragraphs = Vec::new();
    let mut cov = Coverage::default();
    for line in text.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let mut out = Vec::new();
        for t in tok.tokenize(line)? {
            let is_word = matches!(t.kind, TokenKind::Word | TokenKind::Name);
            let key = is_word.then(|| WordKey { lang: tok.lang().into(), lemma: t.lemma.clone(), reading: t.lemma_reading.clone() });
            let (mut state, mut source) = match &key {
                Some(k) => state_of(store, k)?,
                None => (WordState::Unknown, "none"),
            };
            if state == WordState::Unknown && !t.parts.is_empty() {
                let mut all_known = true;
                for p in &t.parts {
                    let pk = WordKey { lang: tok.lang().into(), lemma: p.lemma.clone(), reading: p.lemma_reading.clone() };
                    all_known &= state_of(store, &pk)?.0.is_known();
                }
                if all_known {
                    state = WordState::Known;
                    source = "parts";
                }
            }
            if t.kind == TokenKind::Word {
                cov.words += 1;
                match state {
                    s if s.is_known() => cov.known += 1,
                    WordState::Learning => cov.learning += 1,
                    WordState::New => cov.new += 1,
                    _ => cov.unknown += 1,
                }
            }
            out.push(ReaderToken {
                segments: segments(&t.surface, &t.reading),
                has_kanji: has_kanji(&t.surface),
                dictionary_form: t.dictionary_form,
                surface: t.surface,
                kind: t.kind,
                key,
                state,
                source: source.to_string(),
            });
        }
        paragraphs.push(out);
    }
    cov.ratio = if cov.words == 0 { 0.0 } else { cov.known as f64 / cov.words as f64 };
    Ok(ReaderDoc { paragraphs, coverage: cov })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::tokenize::test_support::sudachi;

    #[test]
    fn colours_and_coverage() {
        let Some(t) = sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        let key = |l: &str, r: &str| WordKey { lang: "ja".into(), lemma: l.into(), reading: r.into() };
        let card = |id| CardFacts { card_id: id, note_id: id, card_type: 2, queue: 2, interval: 60, lapses: 0, stability: None, retrievability: None };
        s.replace_anki_snapshot(
            &[(1, card(1)), (1, card(2)), (1, card(3))],
            &[(1, key("猫", "ねこ")), (2, key("海外", "かいがい")), (3, key("旅行", "りょこう"))],
            &KnowledgeSettings::default(),
        )
        .unwrap();
        s.set_word_override(&key("好き", "すき"), Some(WordState::MarkedKnown)).unwrap();

        let doc = analyze(&s, t, "猫が好きです。\n\n海外旅行に行きたい。").unwrap();
        assert_eq!(doc.paragraphs.len(), 2);
        let p0 = &doc.paragraphs[0];
        assert_eq!((p0[0].surface.as_str(), p0[0].state, p0[0].source.as_str()), ("猫", WordState::Known, "anki"));
        assert_eq!(p0[2].state, WordState::MarkedKnown);
        let p1 = &doc.paragraphs[1];
        assert_eq!((p1[0].surface.as_str(), p1[0].source.as_str()), ("海外旅行", "parts"));
        let 行き = p1.iter().find(|x| x.surface == "行き").unwrap();
        assert_eq!(行き.state, WordState::Unknown);
        assert_eq!(行き.segments[0].ruby.as_deref(), Some("い"));
        // 猫, 好き, 海外旅行 known; 行き unknown.
        assert_eq!((doc.coverage.words, doc.coverage.known), (4, 3));
    }
}
