//! The tokenizer interface (ADR 0001) and its Sudachi implementation (ADR 0008).

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::{MorphemeList, MorphemeView, Mode};

use crate::kana::to_hiragana;
use crate::{Error, Result};

/// What a token is, for colouring and for the comprehension count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    /// Content word: counted and coloured.
    Word,
    /// Proper noun: coloured as a name, not counted.
    Name,
    /// Particles, auxiliaries, affixes: grammar, not counted.
    Grammar,
    /// Numbers.
    Number,
    /// Punctuation and symbols.
    Symbol,
    Space,
}

/// A sub-unit of a long token (Sudachi mode A inside mode C), used as an extra match candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Part {
    pub surface: String,
    pub lemma: String,
    pub lemma_reading: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    pub surface: String,
    /// Dictionary form as written (行っ → 行く).
    pub dictionary_form: String,
    /// Normalized dictionary form (わかり → 分かる); the word key's lemma.
    pub lemma: String,
    /// Reading of the lemma, in hiragana (the word key's reading).
    pub lemma_reading: String,
    /// Reading of the surface as written, in hiragana (for furigana).
    pub reading: String,
    pub pos: Vec<String>,
    pub kind: TokenKind,
    /// Mode A parts when the token is a compound; empty otherwise.
    pub parts: Vec<Part>,
}

pub trait Tokenizer: Send + Sync {
    /// ISO 639-1 code of the language this tokenizer handles.
    fn lang(&self) -> &'static str;
    fn tokenize(&self, text: &str) -> Result<Vec<Token>>;
}

pub struct SudachiTokenizer {
    dict: Arc<JapaneseDictionary>,
}

impl SudachiTokenizer {
    /// `system_dic` is SudachiDict's `system.dic` (core edition, ADR 0008).
    pub fn load(system_dic: &Path) -> Result<Self> {
        let config = Config::new(None, None, Some(system_dic.to_path_buf()))
            .map_err(|e| Error::Tokenizer(e.to_string()))?;
        let dict = JapaneseDictionary::from_cfg(&config).map_err(|e| Error::Tokenizer(e.to_string()))?;
        Ok(SudachiTokenizer { dict: Arc::new(dict) })
    }
}

fn kind_of(pos: &[String]) -> TokenKind {
    let p0 = pos.first().map(String::as_str).unwrap_or("");
    let p1 = pos.get(1).map(String::as_str).unwrap_or("");
    match p0 {
        "空白" => TokenKind::Space,
        "補助記号" | "記号" => TokenKind::Symbol,
        "助詞" | "助動詞" | "接尾辞" | "接頭辞" => TokenKind::Grammar,
        "名詞" if p1 == "固有名詞" => TokenKind::Name,
        "名詞" if p1 == "数詞" => TokenKind::Number,
        _ => TokenKind::Word,
    }
}

fn lemma_of<M: MorphemeView>(m: &M) -> (String, String)
where
    M: MorphemeView,
{
    let lemma = m.normalized_form().to_string();
    let reading = to_hiragana(m.reading_form());
    (lemma, reading)
}

impl Tokenizer for SudachiTokenizer {
    fn lang(&self) -> &'static str {
        "ja"
    }

    fn tokenize(&self, text: &str) -> Result<Vec<Token>> {
        let err = |e: sudachi::error::SudachiError| Error::Tokenizer(e.to_string());
        let mut tokenizer = StatefulTokenizer::new(self.dict.clone(), Mode::C);
        tokenizer.reset().push_str(text);
        tokenizer.do_tokenize().map_err(err)?;
        let mut list = MorphemeList::empty(self.dict.clone());
        list.collect_results(&mut tokenizer).map_err(err)?;

        let mut out = Vec::with_capacity(list.len());
        let mut split = MorphemeList::empty(self.dict.clone());
        for m in list.iter() {
            let pos: Vec<String> = m.part_of_speech().to_vec();
            let kind = kind_of(&pos);
            let (lemma, lemma_reading) = match m.normalized_form_morpheme() {
                Ok(n) if !m.is_oov() => lemma_of(&n),
                _ => (m.normalized_form().to_string(), to_hiragana(m.reading_form())),
            };
            let mut parts = Vec::new();
            if matches!(kind, TokenKind::Word | TokenKind::Name) {
                split.clear();
                if m.split_into(Mode::A, &mut split).map_err(err)? && split.len() > 1 {
                    for p in split.iter() {
                        let (plemma, preading) = match p.normalized_form_morpheme() {
                            Ok(n) if !p.is_oov() => lemma_of(&n),
                            _ => (p.normalized_form().to_string(), to_hiragana(p.reading_form())),
                        };
                        parts.push(Part { surface: p.surface().to_string(), lemma: plemma, lemma_reading: preading });
                    }
                }
            }
            let surface = m.surface().to_string();
            let mut reading = to_hiragana(m.reading_form());
            let mut lemma_reading = lemma_reading;
            if let Some((_, _, preferred)) = PREFERRED_READINGS.iter().find(|(s, l, _)| *s == surface && *l == lemma) {
                reading = preferred.to_string();
                lemma_reading = preferred.to_string();
            }
            // ている, てしまう, てみる, ておく, てほしい…: an auxiliary, not a word of its own.
            let after_te = out.last().is_some_and(|p: &Token| p.kind == TokenKind::Grammar && (p.surface == "て" || p.surface == "で"));
            let kind = if after_te && pos.get(1).is_some_and(|p| p == "非自立可能") && matches!(pos[0].as_str(), "動詞" | "形容詞") {
                TokenKind::Grammar
            } else {
                kind
            };
            out.push(Token {
                surface,
                dictionary_form: m.dictionary_form().to_string(),
                lemma,
                lemma_reading,
                reading,
                pos,
                kind,
                parts,
            });
        }
        crate::counters::fix_readings(&mut out);
        Ok(out)
    }
}

/// Sudachi readings that are right but too formal for ordinary prose: (surface, lemma, reading).
/// Applied to both the furigana and the word key, so the key matches how Anki cards spell the
/// reading. The reader's correctable furigana (ROADMAP M3) handles the rest.
const PREFERRED_READINGS: &[(&str, &str, &str)] = &[("私", "私", "わたし")];

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use super::SudachiTokenizer;

    /// Tests that need the real dictionary read its path from `ANKURA_SUDACHI_DIC`. They are
    /// skipped (pass with a message) when it isn't set, so `cargo test` works without the 193 MB file.
    pub fn sudachi() -> Option<&'static SudachiTokenizer> {
        static TOK: OnceLock<Option<SudachiTokenizer>> = OnceLock::new();
        TOK.get_or_init(|| {
            let path = PathBuf::from(std::env::var_os("ANKURA_SUDACHI_DIC")?);
            Some(SudachiTokenizer::load(&path).expect("load Sudachi dictionary"))
        })
        .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::sudachi;
    use super::*;

    #[test]
    fn lemmas_and_readings() {
        let Some(t) = sudachi() else {
            eprintln!("ANKURA_SUDACHI_DIC not set; skipping");
            return;
        };
        let toks = t.tokenize("意味がわかりませんでした。").unwrap();
        let w = toks.iter().find(|t| t.surface == "わかり").unwrap();
        assert_eq!(w.lemma, "分かる");
        assert_eq!(w.lemma_reading, "わかる");
        assert_eq!(w.kind, TokenKind::Word);
        assert_eq!(toks.iter().find(|t| t.surface == "が").unwrap().kind, TokenKind::Grammar);
        assert_eq!(toks.last().unwrap().kind, TokenKind::Symbol);

        let toks = t.tokenize("私は雨が降っていた").unwrap();
        assert_eq!((toks[0].reading.as_str(), toks[0].lemma_reading.as_str()), ("わたし", "わたし"));
        let i = toks.iter().find(|t| t.surface == "い").unwrap();
        assert_eq!(i.kind, TokenKind::Grammar);

        let toks = t.tokenize("東京へ行った").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Name);
        let v = toks.iter().find(|t| t.surface == "行っ").unwrap();
        assert_eq!((v.lemma.as_str(), v.reading.as_str(), v.lemma_reading.as_str()), ("行く", "いっ", "いく"));
    }

    #[test]
    fn compound_parts() {
        let Some(t) = sudachi() else { return };
        let toks = t.tokenize("海外旅行に行きたい").unwrap();
        assert_eq!(toks[0].surface, "海外旅行");
        let parts: Vec<_> = toks[0].parts.iter().map(|p| p.lemma.as_str()).collect();
        assert_eq!(parts, vec!["海外", "旅行"]);
    }
}

