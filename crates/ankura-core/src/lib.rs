//! Ankura's engine. No UI, Tauri or platform code lives here (ADR 0009), so the Android app can
//! reuse it later.

pub mod anki;
pub mod cards;
pub mod counters;
pub mod dict;
pub mod furigana;
pub mod generate;
pub mod interests;
pub mod kanji;
pub mod kana;
pub mod knowledge;
pub mod llm;
pub mod mapping;
pub mod memory;
pub mod normalize;
pub mod practice;
pub mod primitives;
pub mod proficiency;
pub mod reader;
pub mod stats;
pub mod store;
pub mod tokenize;
pub mod voicevox;

use serde::{Deserialize, Serialize};

/// Identifies a word across Anki, the reader and telemetry: language, normalized lemma and the
/// lemma's reading in hiragana.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WordKey {
    pub lang: String,
    pub lemma: String,
    pub reading: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("tokenizer: {0}")]
    Tokenizer(String),
    #[error("{0}")]
    Anki(String),
    #[error("dictionary: {0}")]
    Dictionary(String),
    #[error("AI: {0}")]
    Llm(String),
    #[error("speech: {0}")]
    Tts(String),
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
