//! JLPT-style practice tests per level (ADR 0015): the JLPT's question types (大問) in its order
//! and times, without listening. Ankura picks the target words from the bundled lists; the AI
//! writes the items around them, a second call checks them, and kanji readings are made by rule.
//! Scores follow the JLPT's score sections and pass lines, scaled to the sections tested.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::kana::is_kanji;
use crate::llm::Llm;
use crate::proficiency::{ListWord, Mc, Rng, VOCAB, distractors, mc_schema, numbered, shuffled, solve, valid, valid_n};
use crate::store::{Store, now_ms};
use crate::tokenize::Tokenizer;
use crate::{Error, Result};

// --- layout ---

/// A JLPT question type (大問).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mondai {
    KanjiReading,
    Orthography,
    WordFormation,
    Context,
    Paraphrase,
    Usage,
    GrammarForm,
    SentenceOrder,
    TextGrammar,
    ShortText,
    MidText,
    LongText,
    Integrated,
    Argument,
    InfoSearch,
    // Listening (聴解, ADR 0018).
    TaskListening,
    PointListening,
    GistListening,
    QuickResponse,
}

/// The score-report area a question type belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Vocab,
    Grammar,
    Reading,
    Listening,
}

impl Mondai {
    pub fn name_ja(self) -> &'static str {
        match self {
            Mondai::KanjiReading => "漢字読み",
            Mondai::Orthography => "表記",
            Mondai::WordFormation => "語形成",
            Mondai::Context => "文脈規定",
            Mondai::Paraphrase => "言い換え類義",
            Mondai::Usage => "用法",
            Mondai::GrammarForm => "文法形式の判断",
            Mondai::SentenceOrder => "文の組み立て",
            Mondai::TextGrammar => "文章の文法",
            Mondai::ShortText => "内容理解（短文）",
            Mondai::MidText => "内容理解（中文）",
            Mondai::LongText => "内容理解（長文）",
            Mondai::Integrated => "統合理解",
            Mondai::Argument => "主張理解（長文）",
            Mondai::InfoSearch => "情報検索",
            Mondai::TaskListening => "課題理解",
            Mondai::PointListening => "ポイント理解",
            Mondai::GistListening => "概要理解",
            Mondai::QuickResponse => "即時応答",
        }
    }

    /// The instruction above the questions, as on the exam.
    pub fn instruction(self) -> &'static str {
        match self {
            Mondai::KanjiReading => "＿＿のことばの読み方として最もよいものを、１・２・３・４から一つえらんでください。",
            Mondai::Orthography => "＿＿のことばを漢字で書くとき、最もよいものを、１・２・３・４から一つえらんでください。",
            Mondai::WordFormation | Mondai::Context | Mondai::GrammarForm => {
                "（　）に入れるのに最もよいものを、１・２・３・４から一つえらんでください。"
            }
            Mondai::Paraphrase => "＿＿に意味が最も近いものを、１・２・３・４から一つえらんでください。",
            Mondai::Usage => "次のことばの使い方として最もよいものを、１・２・３・４から一つえらんでください。",
            Mondai::SentenceOrder => "次の文の ★ に入る最もよいものを、１・２・３・４から一つえらんでください。",
            Mondai::TextGrammar => "次の文章を読んで、文章全体の内容を考えて、［ ］の中に入る最もよいものを、１・２・３・４から一つえらんでください。",
            Mondai::Integrated => "次のAとBの文章を読んで、後の問いに対する答えとして最もよいものを、１・２・３・４から一つえらんでください。",
            Mondai::InfoSearch => "次の案内を見て、下の質問に答えてください。答えは、１・２・３・４から最もよいものを一つえらんでください。",
            Mondai::TaskListening | Mondai::PointListening => "まず質問を聞いてください。それから話を聞いて、１から４の中から、最もよいものを一つえらんでください。",
            Mondai::GistListening => "質問はありません。まず話を聞いてください。それから、質問と選択肢を聞いて、１から４の中から、最もよいものを一つえらんでください。",
            Mondai::QuickResponse => "まず文を聞いてください。それから、その返事を聞いて、１から３の中から、最もよいものを一つえらんでください。",
            _ => "次の文章を読んで、質問に答えてください。答えは、１・２・３・４から最もよいものを一つえらんでください。",
        }
    }

    /// A one-line English hint for the learner.
    pub fn hint(self) -> &'static str {
        match self {
            Mondai::KanjiReading => "Choose the reading of the underlined word.",
            Mondai::Orthography => "Choose the kanji spelling of the underlined word.",
            Mondai::WordFormation => "Choose the prefix or suffix that fits the blank.",
            Mondai::Context => "Choose the word that fits the blank.",
            Mondai::Paraphrase => "Choose the closest meaning to the underlined part.",
            Mondai::Usage => "Choose the sentence that uses the word correctly.",
            Mondai::GrammarForm => "Choose the grammar form that fits the blank.",
            Mondai::SentenceOrder => "Put the four pieces in order; choose the one that goes in the ★ slot.",
            Mondai::TextGrammar => "Choose what fits each numbered blank in the text.",
            Mondai::ShortText | Mondai::MidText | Mondai::LongText => "Read the text and answer.",
            Mondai::Integrated => "Read texts A and B and answer.",
            Mondai::Argument => "Read the text and answer about the writer's view.",
            Mondai::InfoSearch => "Find the information the question asks for.",
            Mondai::TaskListening => "Listen: what will the person do next?",
            Mondai::PointListening => "Listen for the point the question asks about (a reason, a feeling, a detail).",
            Mondai::GistListening => "Listen to the whole talk; the question and the options are only spoken.",
            Mondai::QuickResponse => "Listen to the line and choose the best reply; the replies are only spoken.",
        }
    }

    pub fn area(self) -> Area {
        match self {
            Mondai::KanjiReading | Mondai::Orthography | Mondai::WordFormation | Mondai::Context | Mondai::Paraphrase | Mondai::Usage => {
                Area::Vocab
            }
            Mondai::GrammarForm | Mondai::SentenceOrder | Mondai::TextGrammar => Area::Grammar,
            Mondai::TaskListening | Mondai::PointListening | Mondai::GistListening | Mondai::QuickResponse => Area::Listening,
            _ => Area::Reading,
        }
    }

    fn passage_based(self) -> bool {
        matches!(self, Mondai::TextGrammar) || self.area() == Area::Reading
    }

    /// Options are only spoken (概要理解, 即時応答): the sheet shows their numbers.
    pub fn spoken_options(self) -> bool {
        matches!(self, Mondai::GistListening | Mondai::QuickResponse)
    }

    fn options(self) -> usize {
        if self == Mondai::QuickResponse { 3 } else { 4 }
    }

    /// Questions per passage, given how many questions of this type the test has.
    fn per_passage(self, level: u8, n: usize) -> usize {
        match self {
            Mondai::ShortText => 1,
            Mondai::MidText => n.min(if level == 5 { 2 } else { 3 }),
            _ => n,
        }
        .max(1)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PartSpec {
    pub name: &'static str,
    pub minutes: u32,
    pub mondai: Vec<(Mondai, usize)>,
}

/// The test's parts for a level (5 = N5 … 1 = N1): the JLPT's approximate item counts (目安)
/// and minutes, without listening; `half` halves items (at least one) and time.
pub fn layout(level: u8, half: bool, listening: bool) -> Vec<PartSpec> {
    use Mondai::*;
    let parts: Vec<(&'static str, u32, Vec<(Mondai, usize)>)> = match level {
        5 => vec![
            ("言語知識（文字・語彙）", 20, vec![(KanjiReading, 7), (Orthography, 5), (Context, 6), (Paraphrase, 3)]),
            (
                "言語知識（文法）・読解",
                40,
                vec![(GrammarForm, 9), (SentenceOrder, 4), (TextGrammar, 4), (ShortText, 2), (MidText, 2), (InfoSearch, 1)],
            ),
        ],
        4 => vec![
            ("言語知識（文字・語彙）", 25, vec![(KanjiReading, 7), (Orthography, 5), (Context, 8), (Paraphrase, 4), (Usage, 4)]),
            (
                "言語知識（文法）・読解",
                55,
                vec![(GrammarForm, 13), (SentenceOrder, 4), (TextGrammar, 4), (ShortText, 3), (MidText, 3), (InfoSearch, 2)],
            ),
        ],
        3 => vec![
            ("言語知識（文字・語彙）", 30, vec![(KanjiReading, 8), (Orthography, 6), (Context, 11), (Paraphrase, 5), (Usage, 5)]),
            (
                "言語知識（文法）・読解",
                70,
                vec![
                    (GrammarForm, 13),
                    (SentenceOrder, 5),
                    (TextGrammar, 5),
                    (ShortText, 4),
                    (MidText, 6),
                    (LongText, 4),
                    (InfoSearch, 2),
                ],
            ),
        ],
        2 => vec![(
            "言語知識（文字・語彙・文法）・読解",
            105,
            vec![
                (KanjiReading, 5),
                (Orthography, 5),
                (WordFormation, 3),
                (Context, 7),
                (Paraphrase, 5),
                (Usage, 5),
                (GrammarForm, 12),
                (SentenceOrder, 5),
                (TextGrammar, 5),
                (ShortText, 5),
                (MidText, 9),
                (Integrated, 2),
                (Argument, 3),
                (InfoSearch, 2),
            ],
        )],
        _ => vec![(
            "言語知識（文字・語彙・文法）・読解",
            110,
            vec![
                (KanjiReading, 6),
                (Context, 7),
                (Paraphrase, 6),
                (Usage, 6),
                (GrammarForm, 10),
                (SentenceOrder, 5),
                (TextGrammar, 5),
                (ShortText, 4),
                (MidText, 9),
                (LongText, 4),
                (Integrated, 2),
                (Argument, 4),
                (InfoSearch, 2),
            ],
        )],
    };
    let mut parts = parts;
    if listening {
        // 聴解 with its official minutes; 発話表現 (pictures) and 統合理解 aren't written yet.
        let (minutes, l): (u32, Vec<(Mondai, usize)>) = match level {
            5 => (30, vec![(TaskListening, 7), (PointListening, 6), (QuickResponse, 6)]),
            4 => (35, vec![(TaskListening, 8), (PointListening, 7), (QuickResponse, 8)]),
            3 => (40, vec![(TaskListening, 6), (PointListening, 6), (GistListening, 3), (QuickResponse, 9)]),
            2 => (50, vec![(TaskListening, 5), (PointListening, 6), (GistListening, 5), (QuickResponse, 12)]),
            _ => (55, vec![(TaskListening, 6), (PointListening, 7), (GistListening, 6), (QuickResponse, 14)]),
        };
        parts.push(("聴解", minutes, l));
    }
    parts
        .into_iter()
        .map(|(name, minutes, mondai)| PartSpec {
            name,
            minutes: if half { minutes.div_ceil(2) } else { minutes },
            mondai: mondai.into_iter().map(|(m, n)| (m, if half { n.div_ceil(2) } else { n })).collect(),
        })
        .collect()
}

// --- items ---

/// One spoken line of a listening question.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Line {
    /// 'woman', 'man' or 'narrator'.
    pub speaker: String,
    pub text: String,
}

/// What a listening question plays: the narrator's introduction, the talk, the question.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Script {
    pub intro: String,
    pub lines: Vec<Line>,
    pub question: String,
}

impl Script {
    fn transcript(&self) -> String {
        let mut t = String::new();
        if !self.intro.is_empty() {
            t.push_str(&format!("（ナレーター）{}\n", self.intro));
        }
        for l in &self.lines {
            let who = match l.speaker.as_str() {
                "woman" => "女",
                "man" => "男",
                _ => "ナレーター",
            };
            t.push_str(&format!("（{who}）{}\n", l.text));
        }
        if !self.question.is_empty() {
            t.push_str(&format!("（ナレーター）{}\n", self.question));
        }
        t
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WordRef {
    pub expression: String,
    pub reading: String,
}

/// A practice item with its key and explanation; the test sheet gets `ItemView`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub mondai: Mondai,
    pub level: u8,
    pub part: usize,
    pub prompt: String,
    /// Part of the prompt to underline.
    #[serde(default)]
    pub underline: Option<String>,
    pub options: Vec<String>,
    pub answer: usize,
    #[serde(default)]
    pub passage: Option<String>,
    /// Items on the same passage share a group.
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub explanation: String,
    /// The tested word, for Anki cards from missed items.
    #[serde(default)]
    pub word: Option<WordRef>,
    /// The question's id in the bank (ADR 0016); several tests can use one question.
    #[serde(default)]
    pub origin: Option<String>,
    /// For listening questions: what is played (ADR 0018).
    #[serde(default)]
    pub script: Option<Script>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemView {
    pub id: String,
    pub number: usize,
    pub prompt: String,
    pub underline: Option<String>,
    pub options: Vec<String>,
    pub passage: Option<String>,
    pub group: Option<String>,
    /// Listening: what to play; options are numbers only when `spoken_options`.
    pub script: Option<Script>,
    pub spoken_options: bool,
}

fn new_item(mondai: Mondai, level: u8, prompt: String, options: Vec<String>, answer: usize) -> Item {
    Item {
        id: Uuid::new_v4().to_string(),
        mondai,
        level,
        part: 0,
        prompt,
        underline: None,
        options,
        answer,
        passage: None,
        group: None,
        explanation: String::new(),
        word: None,
        origin: None,
        script: None,
    }
}

/// The easiest list level of each word (5 = N5).
static LEVEL_OF: LazyLock<HashMap<&'static str, u8>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, u8> = HashMap::new();
    for w in VOCAB.iter() {
        let e = m.entry(w.expression.as_str()).or_insert(w.level);
        *e = (*e).max(w.level);
    }
    m
});

/// List words in a text two or more levels above `level` (other than `except`).
fn drift(tok: &dyn Tokenizer, text: &str, level: u8, except: Option<&str>) -> usize {
    let Ok(tokens) = tok.tokenize(text) else { return 0 };
    let mut seen = HashSet::new();
    tokens
        .iter()
        // Words with kanji only: kana-only list entries are often function words (the list's
        // た is a noun, not the past tense).
        .filter(|t| t.dictionary_form.chars().any(is_kanji))
        .filter(|t| Some(t.dictionary_form.as_str()) != except && Some(t.lemma.as_str()) != except)
        .filter_map(|t| LEVEL_OF.get(t.dictionary_form.as_str()).or_else(|| LEVEL_OF.get(t.lemma.as_str())).map(|l| (t.lemma.clone(), *l)))
        .filter(|(lemma, l)| *l + 2 <= level && seen.insert(lemma.clone()))
        .count()
}

/// Blanks as the exam writes them.
fn blank(s: &str) -> String {
    let mut s = s.to_string();
    for b in ["（ ）", "（）", "( )", "()", "(　)"] {
        s = s.replace(b, "（　）");
    }
    s
}

fn marked(prompt: &str, underline: Option<&str>) -> String {
    match underline {
        Some(u) if !u.is_empty() => prompt.replacen(u, &format!("【{u}】"), 1),
        _ => prompt.to_string(),
    }
}

/// The question as the checking call sees it.
fn check_text(m: Mondai, prompt: &str, underline: Option<&str>) -> String {
    match m {
        Mondai::Orthography => format!("【】のことばを漢字で書くとき、最もよいものは？ {}", marked(prompt, underline)),
        Mondai::Paraphrase => format!("【】に意味が最も近いものは？ {}", marked(prompt, underline)),
        Mondai::Usage => format!("「{prompt}」の使い方として最もよいものは？"),
        Mondai::SentenceOrder => format!("★ に入る最もよいものは？ {prompt}"),
        _ => format!("（　）に入れるのに最もよいものは？ {prompt}"),
    }
}

// --- generation ---

const ITEM_SYSTEM: &str = "You write questions for a JLPT-style practice test. Write your own questions; never copy \
official JLPT questions or prep-book questions. The Japanese must be natural, as a native speaker would write it. \
Vocabulary and grammar in sentences must be at the given level or easier, except the word being tested. Exactly one \
option is right; the other three are plausible for a learner but clearly wrong to someone who knows the point \
tested. Options have similar length and form, and the right one must not be guessable from the wording alone. Each \
item has a short explanation in English: why the right option is right and why the others aren't.";

const PASSAGE_SYSTEM: &str = "You write texts and questions for a JLPT-style practice test. Write your own texts; \
never copy official JLPT texts or prep books. Texts are natural Japanese at the given level (vocabulary, grammar \
and kanji at that level or easier), on everyday, social or cultural topics. Questions and options are in Japanese, \
four options each, exactly one right. Each question has a short explanation in English: where the text gives the \
answer and why the other options are wrong.";

fn spec(m: Mondai) -> &'static str {
    match m {
        Mondai::KanjiReading => "Type: 漢字読み. For each given word, write one sentence that contains the word exactly as \
given (same kanji, not conjugated). word = the given word, sentence = the sentence, underlined = the word; options = \
an empty list, answer = 0, explanation = empty.",
        Mondai::Orthography => "Type: 表記. For each given word, write a sentence in which the word is written in \
hiragana; underlined = that hiragana part exactly as it appears in the sentence. Options: four kanji spellings of \
that part: the right one (the given word's spelling) and three wrong ones that use kanji with the same reading or a \
similar shape. word = the given word.",
        Mondai::WordFormation => "Type: 語形成. Choose your own items at the level: sentences with one blank （　） where a \
prefix or suffix fits (such as 未〜, 再〜, 〜的, 〜化, 〜感, 〜性); options: four prefixes or suffixes. word = the \
derived word the right option makes.",
        Mondai::Context => "Type: 文脈規定. For each given word, a sentence with one blank （　） that only the given word \
fits (in the form the sentence needs). Options: four words of the same part of speech and level, the given word \
being the right one. word = the given word.",
        Mondai::Paraphrase => "Type: 言い換え類義. For each given word, a sentence that uses it; underlined = the word as it \
appears in the sentence. Options: four words or short phrases; only the right one means nearly the same in this \
sentence and could replace it. word = the given word.",
        Mondai::Usage => "Type: 用法. For each given word, sentence = the word itself, and options = four sentences that \
each use the word: one uses it naturally and correctly, the other three use it where its meaning or collocation is \
wrong (the error is in how the word is used, never in other grammar). word = the given word.",
        Mondai::GrammarForm => "Type: 文法形式の判断. Choose your own grammar points at the level, all different: \
sentences with one blank （　）; options: four grammar forms (particles, conjugations, expressions). word = the \
grammar point tested.",
        Mondai::SentenceOrder => "Type: 文の組み立て. Choose your own grammar points at the level, all different. For \
each item, write one natural sentence and split a middle stretch of it into four fragments that only fit together \
in one order. before = the sentence before the fragments, fragments = the four fragments in the right order, after = \
the rest of the sentence. The explanation says why that order is the only one.",
        Mondai::TextGrammar => "Type: 文章の文法. A coherent text (an essay or a letter) with numbered blanks written \
［1］, ［2］ … in order. One question per blank: question = the blank's mark, such as ［1］; options: four \
conjunctions, grammar forms or expressions, of which only one fits the flow of the text. text2 = empty.",
        Mondai::ShortText => "Type: 内容理解（短文）. Short texts (a note, an email, a notice, a short essay), one \
question each on the main point or a detail. text2 = empty.",
        Mondai::MidText => "Type: 内容理解（中文）. Texts such as essays, explanations or columns; questions on reasons, \
the writer's view and what a phrase refers to. text2 = empty.",
        Mondai::LongText => "Type: 内容理解（長文）. A long essay; questions on the overall argument and on details. \
text2 = empty.",
        Mondai::Integrated => "Type: 統合理解. Two texts, A (text) and B (text2), giving different views on one \
question; the questions ask what both say or how they differ.",
        Mondai::Argument => "Type: 主張理解. An editorial-style text with a clear argument; the questions ask about the \
writer's claim and reasoning. text2 = empty.",
        Mondai::InfoSearch => "Type: 情報検索. An information text (an advert, a notice, a timetable, a price list or a \
guide with conditions), laid out with line breaks; each question gives a situation with conditions, and the reader \
finds the information that matches. text2 = empty.",
        m => listening_spec(m),
    }
}

/// Length of each text, in Japanese characters.
fn length(m: Mondai, level: u8) -> &'static str {
    match (m, level) {
        (Mondai::ShortText, 5 | 4) => "80-150 characters",
        (Mondai::ShortText, _) => "150-220 characters",
        (Mondai::MidText, 5 | 4) => "250-350 characters",
        (Mondai::MidText, 3) => "300-400 characters",
        (Mondai::MidText, _) => "450-550 characters",
        (Mondai::LongText, 3) => "500-600 characters",
        (Mondai::LongText, _) => "900-1000 characters",
        (Mondai::Integrated, _) => "250-350 characters each",
        (Mondai::Argument, _) => "800-1000 characters",
        (Mondai::InfoSearch, 5 | 4) => "200-300 characters",
        (Mondai::InfoSearch, _) => "400-600 characters",
        (Mondai::TextGrammar, 5 | 4) => "200-300 characters",
        (Mondai::TextGrammar, 3) => "350-450 characters",
        (Mondai::TextGrammar, _) => "500-650 characters",
        _ => "",
    }
}

fn items_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "practice_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "word": {"type": "string"},
                        "sentence": {"type": "string"},
                        "underlined": {"type": "string"},
                        "options": {"type": "array", "items": {"type": "string"}},
                        "answer": {"type": "integer"},
                        "explanation": {"type": "string"}
                    },
                    "required": ["word", "sentence", "underlined", "options", "answer", "explanation"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["practice_items"],
        "additionalProperties": false
    })
}

fn order_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "order_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "before": {"type": "string"},
                        "fragments": {"type": "array", "items": {"type": "string"}},
                        "after": {"type": "string"},
                        "explanation": {"type": "string"}
                    },
                    "required": ["before", "fragments", "after", "explanation"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["order_items"],
        "additionalProperties": false
    })
}

fn passages_schema() -> Value {
    let mut q = mc_schema();
    q["properties"]["explanation"] = json!({"type": "string"});
    q["required"] = json!(["question", "options", "answer", "explanation"]);
    json!({
        "type": "object",
        "properties": {
            "practice_passages": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {"text": {"type": "string"}, "text2": {"type": "string"}, "questions": {"type": "array", "items": q}},
                    "required": ["text", "text2", "questions"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["practice_passages"],
        "additionalProperties": false
    })
}

fn parse<T: for<'de> Deserialize<'de>>(v: &Value, key: &str) -> Result<Vec<T>> {
    serde_json::from_value(v[key].clone()).map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))
}

/// What one question type needs: how many items, and (for word types) the words to build on.
struct Job {
    mondai: Mondai,
    level: u8,
    part: usize,
    /// Questions still to write (the rest came from the bank).
    need: usize,
    words: Vec<WordRef>,
    reused: Vec<Item>,
}

#[derive(Deserialize)]
struct RawItem {
    word: String,
    sentence: String,
    underlined: String,
    options: Vec<String>,
    answer: usize,
    explanation: String,
}

/// Word types (one sentence per item): asks for a third more than needed, keeps what the check
/// confirms, and asks once more for what's missing.
fn word_items(llm: &dyn Llm, tok: Option<&dyn Tokenizer>, job: &Job) -> Result<Vec<Item>> {
    let mut out: Vec<Item> = Vec::new();
    let mut words = job.words.iter();
    for _round in 0..2 {
        let missing = job.need.saturating_sub(out.len());
        if missing == 0 {
            break;
        }
        let ask = missing + missing.div_ceil(3) + 1;
        let mut prompt = format!("Level: N{}\n{}\nNumber of items: {ask}\n", job.level, spec(job.mondai));
        let given: Vec<&WordRef> = if job.words.is_empty() { vec![] } else { words.by_ref().take(ask).collect() };
        if !job.words.is_empty() {
            if given.is_empty() {
                break;
            }
            prompt.push_str("Words (one item each, in this order):\n");
            for (i, w) in given.iter().enumerate() {
                prompt.push_str(&format!("{}. {}（{}）\n", i + 1, w.expression, w.reading));
            }
        }
        let raw: Vec<RawItem> = parse(&llm.json(ITEM_SYSTEM, &prompt, &items_schema())?, "practice_items")?;
        let mut made: Vec<Item> = Vec::new();
        for r in raw {
            let word = given.iter().find(|w| w.expression == r.word.trim()).map(|w| (*w).clone());
            if !job.words.is_empty() && word.is_none() {
                continue;
            }
            let sentence = blank(r.sentence.trim());
            let underline = Some(r.underlined.trim().to_string()).filter(|u| !u.is_empty());
            let mut it = match job.mondai {
                Mondai::KanjiReading => {
                    let w = word.clone().expect("checked");
                    if !sentence.contains(&w.expression) {
                        continue;
                    }
                    // Options are made by rule; the dictionary check happens when the item is saved.
                    let mut options = vec![w.reading.clone()];
                    options.extend(distractors(&w.reading));
                    let mut it = new_item(job.mondai, job.level, sentence, options, 0);
                    it.underline = Some(w.expression.clone());
                    it.explanation = format!("{} is read {}.", w.expression, w.reading);
                    it
                }
                m => {
                    let mc = Mc { question: sentence.clone(), options: r.options.iter().map(|o| o.trim().to_string()).collect(), answer: r.answer };
                    if !valid(&mc) {
                        continue;
                    }
                    let ok = match m {
                        Mondai::Orthography | Mondai::Paraphrase => underline.as_ref().is_some_and(|u| sentence.contains(u.as_str())),
                        Mondai::Usage => word.is_some(),
                        _ => sentence.matches("（　）").count() == 1,
                    };
                    if !ok {
                        continue;
                    }
                    let prompt = if m == Mondai::Usage { word.as_ref().expect("checked").expression.clone() } else { sentence };
                    let mut it = new_item(m, job.level, prompt, mc.options, mc.answer);
                    it.underline = if matches!(m, Mondai::Orthography | Mondai::Paraphrase) { underline } else { None };
                    it.explanation = r.explanation.trim().to_string();
                    it
                }
            };
            if let Some(tok) = tok {
                let text = format!("{}\n{}", it.prompt, if job.mondai == Mondai::Usage { it.options.join("\n") } else { String::new() });
                if drift(tok, &text, job.level, word.as_ref().map(|w| w.expression.as_str())) > 0 {
                    continue;
                }
            }
            it.part = job.part;
            it.word = word;
            made.push(it);
        }
        // The check: answer every AI-written item cold; keep the ones it gets right.
        if job.mondai != Mondai::KanjiReading && !made.is_empty() {
            let mcs: Vec<Mc> = made
                .iter()
                .map(|i| Mc { question: check_text(i.mondai, &i.prompt, i.underline.as_deref()), options: i.options.clone(), answer: i.answer })
                .collect();
            let checked = solve(llm, &numbered(&mcs, None))?;
            made = made
                .into_iter()
                .enumerate()
                .filter(|(i, it)| checked.get(i).is_some_and(|(c, amb)| *c == it.answer && !amb))
                .map(|(_, it)| it)
                .collect();
        }
        out.extend(made);
    }
    let mut rng = Rng::new();
    for it in out.iter_mut().filter(|i| i.mondai != Mondai::KanjiReading) {
        let (options, answer) = shuffled(&Mc { question: String::new(), options: it.options.clone(), answer: it.answer }, &mut rng);
        it.options = options;
        it.answer = answer;
    }
    Ok(out)
}

#[derive(Deserialize)]
struct RawOrder {
    before: String,
    fragments: Vec<String>,
    after: String,
    explanation: String,
}

/// 文の組み立て: four fragments in order from the AI; the ★ is the third slot.
fn order_items(llm: &dyn Llm, tok: Option<&dyn Tokenizer>, job: &Job) -> Result<Vec<Item>> {
    let mut out: Vec<Item> = Vec::new();
    for _round in 0..2 {
        let missing = job.need.saturating_sub(out.len());
        if missing == 0 {
            break;
        }
        let ask = missing + missing.div_ceil(3) + 1;
        let prompt = format!("Level: N{}\n{}\nNumber of items: {ask}\n", job.level, spec(job.mondai));
        let raw: Vec<RawOrder> = parse(&llm.json(ITEM_SYSTEM, &prompt, &order_schema())?, "order_items")?;
        let mut rng = Rng::new();
        let mut made = Vec::new();
        for r in raw {
            let f: Vec<String> = r.fragments.iter().map(|x| x.trim().to_string()).collect();
            if f.len() != 4 || f.iter().any(String::is_empty) || f.iter().collect::<HashSet<_>>().len() != 4 {
                continue;
            }
            let (before, after) = (r.before.trim(), r.after.trim());
            let full = format!("{before}{}{after}", f.concat());
            if tok.is_some_and(|t| drift(t, &full, job.level, None) > 0) {
                continue;
            }
            let prompt = format!("{before}＿＿＿ ＿＿＿ ★ ＿＿＿{after}");
            let mut options = f.clone();
            rng.shuffle(&mut options);
            let answer = options.iter().position(|o| *o == f[2]).expect("in options");
            let mut it = new_item(job.mondai, job.level, prompt, options, answer);
            it.part = job.part;
            it.explanation = format!("{full}\n{}", r.explanation.trim());
            made.push(it);
        }
        if !made.is_empty() {
            let mcs: Vec<Mc> = made
                .iter()
                .map(|i| Mc { question: check_text(i.mondai, &i.prompt, None), options: i.options.clone(), answer: i.answer })
                .collect();
            let checked = solve(llm, &numbered(&mcs, None))?;
            out.extend(
                made.into_iter().enumerate().filter(|(i, it)| checked.get(i).is_some_and(|(c, amb)| *c == it.answer && !amb)).map(|(_, it)| it),
            );
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
struct RawQuestion {
    question: String,
    options: Vec<String>,
    answer: usize,
    explanation: String,
}

#[derive(Deserialize)]
struct RawPassage {
    text: String,
    text2: String,
    questions: Vec<RawQuestion>,
}

/// Passage types: a text with its questions. Reading questions must be right with the text and
/// not answerable without it; a 文章の文法 blank whose question fails gets its answer filled in.
fn passage_items(llm: &dyn Llm, tok: Option<&dyn Tokenizer>, job: &Job) -> Result<Vec<Item>> {
    let q = job.mondai.per_passage(job.level, job.need);
    let mut out: Vec<Item> = Vec::new();
    for _round in 0..2 {
        let missing = job.need.saturating_sub(out.len());
        if missing == 0 {
            break;
        }
        let texts = missing.div_ceil(q) + 1;
        let prompt = format!(
            "Level: N{}\n{}\nNumber of texts: {texts}\nQuestions per text: {q}\nLength of each text: {}\n",
            job.level,
            spec(job.mondai),
            length(job.mondai, job.level)
        );
        let raw: Vec<RawPassage> = parse(&llm.json(PASSAGE_SYSTEM, &prompt, &passages_schema())?, "practice_passages")?;
        let mut rng = Rng::new();
        // Every passage is checked, also the extras: they go into the bank (ADR 0016).
        for p in raw {
            let mut text = p.text.trim().to_string();
            if job.mondai == Mondai::Integrated {
                if p.text2.trim().is_empty() {
                    continue;
                }
                text = format!("A\n{text}\n\nB\n{}", p.text2.trim());
            }
            if job.mondai == Mondai::TextGrammar {
                for i in 1..=p.questions.len() {
                    text = text.replace(&format!("[{i}]"), &format!("［{i}］"));
                }
            }
            if text.is_empty() || tok.is_some_and(|t| drift(t, &text, job.level, None) > 2) {
                continue;
            }
            let qs: Vec<(Mc, String)> = p
                .questions
                .into_iter()
                .map(|r| (Mc { question: r.question.trim().to_string(), options: r.options.iter().map(|o| o.trim().to_string()).collect(), answer: r.answer }, r.explanation))
                .filter(|(m, _)| valid(m))
                .filter(|(m, _)| job.mondai != Mondai::TextGrammar || text.contains(m.question.as_str()))
                .take(q)
                .collect();
            if qs.is_empty() {
                continue;
            }
            let mcs: Vec<Mc> = qs.iter().map(|(m, _)| m.clone()).collect();
            let with = solve(llm, &numbered(&mcs, Some(&text)))?;
            let without = if job.mondai == Mondai::TextGrammar { HashMap::new() } else { solve(llm, &numbered(&mcs, None))? };
            let keep: Vec<bool> = mcs
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    with.get(&i).is_some_and(|(c, amb)| *c == m.answer && !amb) && !without.get(&i).is_some_and(|(c, _)| *c == m.answer)
                })
                .collect();
            if !keep.iter().any(|k| *k) {
                continue;
            }
            if job.mondai == Mondai::TextGrammar {
                for (m, k) in mcs.iter().zip(&keep) {
                    if !k {
                        text = text.replace(m.question.as_str(), &m.options[m.answer]);
                    }
                }
            }
            let group = Uuid::new_v4().to_string();
            for ((m, explanation), k) in qs.iter().zip(&keep) {
                if !k {
                    continue;
                }
                let (options, answer) = shuffled(m, &mut rng);
                let mut it = new_item(job.mondai, job.level, m.question.clone(), options, answer);
                it.part = job.part;
                it.passage = Some(text.clone());
                it.group = Some(group.clone());
                it.explanation = explanation.trim().to_string();
                out.push(it);
            }
        }
    }
    Ok(out)
}

/// Splits items into the ones a test takes (up to `need`) and the rest. Passages go whole; the
/// first passage may be cut when it alone has more questions than needed (not 文章の文法, whose
/// blanks would be left without questions).
fn take(items: Vec<Item>, need: usize) -> (Vec<Item>, Vec<Item>) {
    let (mut taken, mut rest) = (Vec::new(), Vec::new());
    let mut groups: Vec<Vec<Item>> = Vec::new();
    for it in items {
        match groups.last_mut() {
            Some(g) if it.group.is_some() && g[0].group == it.group => g.push(it),
            _ => groups.push(vec![it]),
        }
    }
    for mut g in groups {
        let room = need - taken.len();
        if g.len() <= room {
            taken.append(&mut g);
        } else if taken.is_empty() && room > 0 && g[0].mondai != Mondai::TextGrammar {
            rest.extend(g.split_off(room));
            taken.append(&mut g);
        } else {
            rest.append(&mut g);
        }
    }
    (taken, rest)
}

const LISTENING_SYSTEM: &str = "You write listening questions for a JLPT-style practice test; a speech \
engine will speak them. Write your own; never copy official JLPT questions or prep books. Spoken Japanese must be \
natural and at the given level (vocabulary, grammar), as people really talk, with speech-like fillers only where \
natural. Speakers are 'woman' and 'man'. Write numbers and times so they are read unambiguously. Exactly one option is \
right; the others are plausible for a learner who misheard or missed a detail. Each item has a short explanation in \
English: which line gives the answer and why the others are wrong.";

fn listening_spec(m: Mondai) -> &'static str {
    match m {
        Mondai::TaskListening => "Type: 課題理解. intro: the narrator's setting and question, such as \
「女の人と男の人が話しています。男の人はこのあと何をしますか。」. lines: a conversation of 6-12 lines about a task \
(what to buy, where to go, what to prepare), where plans change once so the listener must follow to the end. \
question: the narrator's question again. options: four short written answers (actions).",
        Mondai::PointListening => "Type: ポイント理解. intro: the narrator's setting and question, such as \
「男の人と女の人が話しています。女の人はどうして遅れましたか。」. lines: a conversation of 6-12 lines; the answer \
(a reason, a feeling, a detail) comes up after a few other candidates. question: the narrator's question again. \
options: four short written answers.",
        Mondai::GistListening => "Type: 概要理解. intro: the narrator's setting only, such as \
「ラジオで男の人が話しています。」 (no question). lines: one speaker's talk of 150-300 characters (a radio piece, \
a speech, a message). question: the narrator's question about the main point or the speaker's view. options: four \
answers, each one sentence (they are spoken, not shown).",
        Mondai::QuickResponse => "Type: 即時応答. intro and question: empty. lines: exactly one short line by \
'woman' or 'man' (a question, a request, a remark). options: three short replies by the other speaker; only one is a \
natural reply. They are spoken, not shown.",
        _ => "",
    }
}

fn listening_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "listening_items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "intro": {"type": "string"},
                        "lines": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {"speaker": {"type": "string", "enum": ["woman", "man"]}, "text": {"type": "string"}},
                                "required": ["speaker", "text"],
                                "additionalProperties": false
                            }
                        },
                        "question": {"type": "string"},
                        "options": {"type": "array", "items": {"type": "string"}},
                        "answer": {"type": "integer"},
                        "explanation": {"type": "string"}
                    },
                    "required": ["intro", "lines", "question", "options", "answer", "explanation"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["listening_items"],
        "additionalProperties": false
    })
}

#[derive(Deserialize)]
struct RawListening {
    intro: String,
    lines: Vec<Line>,
    question: String,
    options: Vec<String>,
    answer: usize,
    explanation: String,
}

/// 聴解 (ADR 0018): a script with its question. The check answers from the transcript; 課題理解
/// and ポイント理解 questions it can answer without the talk are dropped.
fn listening_items(llm: &dyn Llm, tok: Option<&dyn Tokenizer>, job: &Job) -> Result<Vec<Item>> {
    let m = job.mondai;
    let mut out: Vec<Item> = Vec::new();
    for _round in 0..2 {
        let missing = job.need.saturating_sub(out.len());
        if missing == 0 {
            break;
        }
        let ask = missing + missing.div_ceil(3) + 1;
        let prompt = format!("Level: N{}\n{}\nNumber of items: {ask}\n", job.level, listening_spec(m));
        let raw: Vec<RawListening> = parse(&llm.json(LISTENING_SYSTEM, &prompt, &listening_schema())?, "listening_items")?;
        let mut made: Vec<Item> = Vec::new();
        for r in raw {
            let lines: Vec<Line> = r
                .lines
                .into_iter()
                .map(|l| Line { speaker: if l.speaker == "man" { "man".into() } else { "woman".into() }, text: l.text.trim().to_string() })
                .filter(|l| !l.text.is_empty())
                .collect();
            let options: Vec<String> = r.options.iter().map(|o| o.trim().to_string()).collect();
            let question = r.question.trim().to_string();
            let shown = if m == Mondai::QuickResponse { "この発話に対する返事として最もよいものは？".to_string() } else { question.clone() };
            let mc = Mc { question: shown.clone(), options: options.clone(), answer: r.answer };
            let ok = valid_n(&mc, m.options())
                && !lines.is_empty()
                && (m == Mondai::QuickResponse || !question.is_empty())
                && (m != Mondai::QuickResponse || lines.len() == 1);
            if !ok {
                continue;
            }
            let script = Script { intro: r.intro.trim().to_string(), lines, question: if m == Mondai::QuickResponse { String::new() } else { question } };
            let spoken: String = script.lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
            if tok.is_some_and(|t| drift(t, &spoken, job.level, None) > 1) {
                continue;
            }
            let mut it = new_item(m, job.level, shown, options, r.answer);
            it.part = job.part;
            it.explanation = r.explanation.trim().to_string();
            it.script = Some(script);
            made.push(it);
        }
        if made.is_empty() {
            continue;
        }
        let with: Vec<Mc> = made
            .iter()
            .map(|i| Mc {
                question: format!("Transcript:\n{}Question: {}", i.script.as_ref().map(Script::transcript).unwrap_or_default(), i.prompt),
                options: i.options.clone(),
                answer: i.answer,
            })
            .collect();
        let right = solve(llm, &numbered(&with, None))?;
        let blind = if matches!(m, Mondai::TaskListening | Mondai::PointListening) {
            let without: Vec<Mc> = made.iter().map(|i| Mc { question: i.prompt.clone(), options: i.options.clone(), answer: i.answer }).collect();
            solve(llm, &numbered(&without, None))?
        } else {
            HashMap::new()
        };
        let mut rng = Rng::new();
        for (i, mut it) in made.into_iter().enumerate() {
            let checked = right.get(&i).is_some_and(|(c, amb)| *c == it.answer && !amb);
            let guessable = blind.get(&i).is_some_and(|(c, _)| *c == it.answer);
            if checked && !guessable {
                let (options, answer) = shuffled(&Mc { question: String::new(), options: it.options.clone(), answer: it.answer }, &mut rng);
                it.options = options;
                it.answer = answer;
                out.push(it);
            }
        }
    }
    Ok(out)
}

fn run_job(llm: &dyn Llm, tok: Option<&dyn Tokenizer>, job: &Job) -> Result<Vec<Item>> {
    if job.mondai.area() == Area::Listening {
        listening_items(llm, tok, job)
    } else if job.mondai == Mondai::SentenceOrder {
        order_items(llm, tok, job)
    } else if job.mondai.passage_based() {
        passage_items(llm, tok, job)
    } else {
        word_items(llm, tok, job)
    }
}

/// Words for the word types, at the test's level, not used elsewhere in the test.
fn pick_words(m: Mondai, level: u8, n: usize, used: &mut HashSet<String>, rng: &mut Rng) -> Vec<WordRef> {
    let fits = |w: &ListWord| match m {
        Mondai::KanjiReading => {
            w.expression.chars().count() >= 2 && w.expression.chars().all(is_kanji) && distractors(&w.reading).len() >= 3
        }
        Mondai::Orthography => w.expression.chars().any(is_kanji) && w.expression != w.reading,
        Mondai::Context | Mondai::Paraphrase | Mondai::Usage => w.expression.chars().count() >= 2,
        _ => false,
    };
    let mut pool: Vec<&ListWord> = VOCAB.iter().filter(|w| w.level == level && fits(w) && !used.contains(&w.expression)).collect();
    rng.shuffle(&mut pool);
    let mut out = Vec::new();
    for w in pool {
        if out.len() == n {
            break;
        }
        if used.insert(w.expression.clone()) {
            out.push(WordRef { expression: w.expression.clone(), reading: w.reading.clone() });
        }
    }
    out
}

/// Kanji reading options: drops rule-made readings the dictionary knows for the spelling, keeps
/// three, shuffles. False when too few are left.
fn finish_kanji(store: &Store, it: &mut Item, rng: &mut Rng) -> Result<bool> {
    let key = it.options[0].clone();
    let expression = it.word.as_ref().map(|w| w.expression.clone()).unwrap_or_default();
    let real: HashSet<String> = crate::dict::readings(store, &expression)?.into_iter().collect();
    let mut wrong: Vec<String> = it.options[1..].iter().filter(|o| !real.contains(*o)).cloned().collect();
    if wrong.len() < 3 {
        return Ok(false);
    }
    rng.shuffle(&mut wrong);
    let mut options: Vec<String> = wrong.into_iter().take(3).collect();
    options.push(key.clone());
    rng.shuffle(&mut options);
    it.answer = options.iter().position(|o| *o == key).expect("added");
    it.options = options;
    Ok(true)
}

// --- tests ---

/// Columns added to the level check's tables (schema v11).
pub(crate) const PRACTICE_SCHEMA: &str = r#"
ALTER TABLE tests ADD COLUMN config TEXT;               -- JSON (Config), practice tests
ALTER TABLE test_items ADD COLUMN reported INTEGER NOT NULL DEFAULT 0;
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub level: u8,
    pub half: bool,
    pub timed: bool,
    /// Question types written so far.
    #[serde(default)]
    pub prepared: Vec<Mondai>,
    /// Seconds used per part.
    #[serde(default)]
    pub used_secs: Vec<u64>,
    #[serde(default)]
    pub submitted: Vec<bool>,
    /// For a retry: the test it retries.
    #[serde(default)]
    pub parent: Option<String>,
    /// Share of questions to take from the bank, 0-100 (ADR 0016).
    #[serde(default)]
    pub reuse: u8,
    /// Whether bank questions used in earlier tests may come back.
    #[serde(default)]
    pub repeat_seen: bool,
    /// Questions taken from the bank so far.
    #[serde(default)]
    pub from_bank: usize,
    /// With the 聴解 part (ADR 0018).
    #[serde(default)]
    pub listening: bool,
}

impl Config {
    fn layout(&self) -> Vec<PartSpec> {
        layout(self.level, self.half, self.listening)
    }
}

fn config(store: &Store, test_id: &str) -> Result<(String, Config)> {
    let row: Option<(String, Option<String>)> = store
        .conn
        .query_row("SELECT kind, config FROM tests WHERE id = ?1", [test_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let (kind, c) = row.ok_or_else(|| Error::Llm("this test no longer exists".into()))?;
    let c = c.ok_or_else(|| Error::Llm("this isn't a practice test".into()))?;
    Ok((kind, serde_json::from_str(&c)?))
}

fn save_config(store: &Store, test_id: &str, c: &Config) -> Result<()> {
    store.conn.execute("UPDATE tests SET config = ?2 WHERE id = ?1", params![test_id, serde_json::to_string(c)?])?;
    Ok(())
}

fn save_items(store: &Store, test_id: &str, items: &[Item]) -> Result<()> {
    for it in items {
        store.conn.execute(
            "INSERT INTO test_items (test_id, item_id, section, level, item) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![test_id, it.id, serde_json::to_value(it.mondai)?.as_str().unwrap_or_default(), it.level, serde_json::to_string(it)?],
        )?;
    }
    Ok(())
}

struct Stored {
    item: Item,
    choice: Option<usize>,
    reported: bool,
}

fn stored(store: &Store, test_id: &str) -> Result<Vec<Stored>> {
    let mut st = store.conn.prepare("SELECT item, choice, reported FROM test_items WHERE test_id = ?1 ORDER BY rowid")?;
    let rows = st.query_map([test_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?, r.get::<_, i64>(2)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (j, c, rep) = row?;
        out.push(Stored { item: serde_json::from_str(&j)?, choice: c.map(|c| c as usize), reported: rep != 0 });
    }
    // Saved per question type as they're written; the sheet follows the layout (whose order
    // is the enum's).
    out.sort_by_key(|s| (s.item.part, s.item.mondai as u8));
    Ok(out)
}

/// A new practice test at a level (5 = N5 … 1 = N1); `prepare` writes its items.
pub fn create(store: &Store, level: u8, half: bool, timed: bool, reuse: u8, repeat_seen: bool, listening: bool) -> Result<String> {
    let level = level.clamp(1, 5);
    let id = Uuid::new_v4().to_string();
    let parts = layout(level, half, listening).len();
    let c = Config {
        level,
        half,
        timed,
        prepared: vec![],
        used_secs: vec![0; parts],
        submitted: vec![false; parts],
        parent: None,
        reuse: reuse.min(100),
        repeat_seen,
        from_bank: 0,
        listening,
    };
    store.conn.execute(
        "INSERT INTO tests (id, device_id, kind, start_level, started_at, config) VALUES (?1, ?2, 'practice', ?3, ?4, ?5)",
        params![id, store.device_id(), level, now_ms(), serde_json::to_string(&c)?],
    )?;
    Ok(id)
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    /// Questions taken from the bank.
    pub from_bank: usize,
}

fn kind(m: Mondai) -> String {
    serde_json::to_value(m).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

/// Bank questions for one question type: up to `want`, whole passages (ADR 0016).
fn from_bank(
    store: &Store,
    m: Mondai,
    level: u8,
    want: usize,
    repeat_seen: bool,
    used_words: &mut HashSet<String>,
    rng: &mut Rng,
) -> Result<Vec<Item>> {
    if want == 0 {
        return Ok(vec![]);
    }
    let mut rows: Vec<Item> = Vec::new();
    for r in crate::memory::bank_candidates(store, "practice", &kind(m), level, repeat_seen)? {
        if let Ok(mut it) = serde_json::from_str::<Item>(&r.item) {
            // One question per word in a test.
            if it.word.as_ref().is_some_and(|w| used_words.contains(&w.expression)) {
                continue;
            }
            it.origin = Some(r.id);
            it.group = r.group;
            rows.push(it);
        }
    }
    // Keep a passage's questions together, in the order they were written.
    let mut order: Vec<Option<String>> = Vec::new();
    for it in &rows {
        if it.group.is_none() || !order.contains(&it.group) {
            order.push(it.group.clone());
        }
    }
    let mut grouped: Vec<Item> = Vec::new();
    let mut singles = rows.iter().filter(|i| i.group.is_none());
    for g in order {
        match g {
            None => grouped.extend(singles.next().cloned()),
            Some(g) => grouped.extend(rows.iter().filter(|i| i.group.as_deref() == Some(g.as_str())).cloned()),
        }
    }
    let (mut taken, _) = take(grouped, want);
    // Passages that don't fit whole are skipped; a test needs whole passages.
    if m.passage_based() {
        let mut sizes: HashMap<String, usize> = HashMap::new();
        for it in &taken {
            *sizes.entry(it.group.clone().unwrap_or_default()).or_default() += 1;
        }
        let full: HashMap<String, usize> =
            rows.iter().fold(HashMap::new(), |mut m, i| {
                *m.entry(i.group.clone().unwrap_or_default()).or_default() += 1;
                m
            });
        taken.retain(|i| {
            let g = i.group.clone().unwrap_or_default();
            sizes.get(&g) == full.get(&g)
        });
    }
    for it in &mut taken {
        if let Some(w) = &it.word {
            used_words.insert(w.expression.clone());
        }
        it.id = Uuid::new_v4().to_string();
        let (options, answer) = shuffled(&Mc { question: String::new(), options: it.options.clone(), answer: it.answer }, rng);
        it.options = options;
        it.answer = answer;
    }
    Ok(taken)
}

/// What a new test would take from the bank and what it would ask the AI to write.
#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub from_bank: usize,
    pub new: usize,
    /// Question types the AI has to write (about two requests each).
    pub ai_types: usize,
}

pub fn plan(store: &Store, level: u8, half: bool, reuse: u8, repeat_seen: bool, listening: bool) -> Result<Plan> {
    let mut rng = Rng::new();
    let mut p = Plan { from_bank: 0, new: 0, ai_types: 0 };
    let mut words = HashSet::new();
    for part in layout(level, half, listening) {
        for (m, need) in part.mondai {
            let want = (need * usize::from(reuse.min(100))).div_ceil(100);
            let got = from_bank(store, m, level, want, repeat_seen, &mut words, &mut rng)?.len();
            p.from_bank += got;
            p.new += need - got;
            p.ai_types += usize::from(got < need);
        }
    }
    Ok(p)
}

/// How many AI requests run at once while preparing.
const PARALLEL: usize = 4;

/// Writes the question types not written yet, several at a time; resumable. `progress` gets
/// (types done, types in all); `stop` is checked between rounds.
pub fn prepare(
    store: &Store,
    llm: &dyn Llm,
    tok: Option<&dyn Tokenizer>,
    test_id: &str,
    mut progress: impl FnMut(usize, usize),
    stop: impl Fn() -> bool,
) -> Result<Progress> {
    let (_, mut c) = config(store, test_id)?;
    let parts = c.layout();
    let total: usize = parts.iter().map(|p| p.mondai.len()).sum();
    let mut rng = Rng::new();
    let mut used: HashSet<String> = stored(store, test_id)?.into_iter().filter_map(|s| s.item.word.map(|w| w.expression)).collect();
    let mut jobs: Vec<Job> = Vec::new();
    for (pi, p) in parts.iter().enumerate() {
        for &(m, need) in &p.mondai {
            if c.prepared.contains(&m) {
                continue;
            }
            // Bank questions first (as many as the chosen share), then the AI for the rest.
            let want = (need * usize::from(c.reuse.min(100))).div_ceil(100);
            let mut reused = from_bank(store, m, c.level, want, c.repeat_seen, &mut used, &mut rng)?;
            for it in &mut reused {
                it.part = pi;
            }
            let need = need - reused.len();
            let words = if need > 0 { pick_words(m, c.level, need * 2 + 4, &mut used, &mut rng) } else { vec![] };
            jobs.push(Job { mondai: m, level: c.level, part: pi, need, words, reused });
        }
    }
    // Types the bank fills alone need no AI.
    let (ready, jobs): (Vec<Job>, Vec<Job>) = jobs.into_iter().partition(|j| j.need == 0);
    for job in ready {
        finish_job(store, test_id, &mut c, job, vec![], &mut rng)?;
    }
    progress(c.prepared.len(), total);
    let mut first_error = None;
    for chunk in jobs.chunks(PARALLEL) {
        if stop() {
            break;
        }
        let results: Vec<Result<Vec<Item>>> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk.iter().map(|job| s.spawn(move || run_job(llm, tok, job))).collect();
            handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Err(Error::Llm("a question writer crashed".into())))).collect()
        });
        let mut jobs_done = Vec::new();
        for (job, r) in chunk.iter().zip(results) {
            match r {
                Ok(items) => jobs_done.push((job, items)),
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        for (job, items) in jobs_done {
            let job = Job { reused: job.reused.clone(), words: vec![], ..*job };
            finish_job(store, test_id, &mut c, job, items, &mut rng)?;
        }
        progress(c.prepared.len(), total);
        if first_error.is_some() {
            break;
        }
    }
    match first_error {
        Some(e) => Err(e),
        None => Ok(Progress { done: c.prepared.len(), total, from_bank: c.from_bank }),
    }
}

/// Saves one question type: the bank questions and the new ones the test needs; every new
/// checked question goes into the bank, used or not.
fn finish_job(store: &Store, test_id: &str, c: &mut Config, job: Job, mut new: Vec<Item>, rng: &mut Rng) -> Result<()> {
    if job.mondai == Mondai::KanjiReading {
        let mut kept = Vec::new();
        for mut it in new {
            if finish_kanji(store, &mut it, rng)? {
                kept.push(it);
            }
        }
        new = kept;
    }
    let rows: Vec<(String, Option<String>, String)> =
        new.iter().map(|i| Ok((i.id.clone(), i.group.clone(), serde_json::to_string(i)?))).collect::<Result<_>>()?;
    crate::memory::bank_add(store, "practice", &kind(job.mondai), job.level, &rows)?;
    let (taken, _rest) = take(new, job.need);
    let taken: Vec<Item> = taken.into_iter().map(|i| Item { origin: Some(i.id.clone()), id: Uuid::new_v4().to_string(), ..i }).collect();
    let mut items = job.reused;
    let reused = items.len();
    items.extend(taken);
    save_items(store, test_id, &items)?;
    crate::memory::bank_used(store, &items.iter().filter_map(|i| i.origin.clone()).collect::<Vec<_>>())?;
    crate::memory::reused(store, "practice_items", reused)?;
    c.from_bank += reused;
    c.prepared.push(job.mondai);
    save_config(store, test_id, c)
}

#[derive(Debug, Clone, Serialize)]
pub struct Group {
    pub mondai: Mondai,
    pub name_ja: &'static str,
    pub instruction: &'static str,
    pub hint: &'static str,
    pub items: Vec<ItemView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SheetPart {
    pub name: String,
    pub minutes: u32,
    pub used_secs: u64,
    pub submitted: bool,
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sheet {
    pub id: String,
    pub kind: String,
    pub level: u8,
    pub half: bool,
    pub timed: bool,
    pub ready: bool,
    pub prepared: usize,
    pub total: usize,
    pub finished: bool,
    pub parts: Vec<SheetPart>,
    pub answers: HashMap<String, usize>,
}

fn part_names(kind: &str, c: &Config) -> Vec<(String, u32)> {
    if kind == "retry" {
        vec![("Missed questions".into(), 0)]
    } else {
        c.layout().into_iter().map(|p| (p.name.to_string(), p.minutes)).collect()
    }
}

/// The test as the learner takes it: parts, question types with their instructions, numbered
/// items (no keys), and the answers so far.
pub fn sheet(store: &Store, test_id: &str) -> Result<Sheet> {
    let (kind, c) = config(store, test_id)?;
    let finished: bool =
        store.conn.query_row("SELECT finished_at IS NOT NULL FROM tests WHERE id = ?1", [test_id], |r| r.get(0))?;
    let all = stored(store, test_id)?;
    let total = if kind == "retry" { 0 } else { c.layout().iter().map(|p| p.mondai.len()).sum() };
    let mut parts: Vec<SheetPart> = part_names(&kind, &c)
        .into_iter()
        .enumerate()
        .map(|(i, (name, minutes))| SheetPart {
            name,
            minutes,
            used_secs: c.used_secs.get(i).copied().unwrap_or(0),
            submitted: c.submitted.get(i).copied().unwrap_or(false),
            groups: vec![],
        })
        .collect();
    let mut answers = HashMap::new();
    for (n, s) in all.iter().enumerate() {
        let it = &s.item;
        if let Some(ch) = s.choice {
            answers.insert(it.id.clone(), ch);
        }
        let Some(part) = parts.get_mut(it.part) else { continue };
        let view = ItemView {
            id: it.id.clone(),
            number: n + 1,
            prompt: it.prompt.clone(),
            underline: it.underline.clone(),
            options: it.options.clone(),
            passage: it.passage.clone(),
            group: it.group.clone(),
            script: it.script.clone(),
            spoken_options: it.mondai.spoken_options(),
        };
        match part.groups.last_mut() {
            Some(g) if g.mondai == it.mondai => g.items.push(view),
            _ => part.groups.push(Group {
                mondai: it.mondai,
                name_ja: it.mondai.name_ja(),
                instruction: it.mondai.instruction(),
                hint: it.mondai.hint(),
                items: vec![view],
            }),
        }
    }
    Ok(Sheet {
        id: test_id.into(),
        ready: kind == "retry" || c.prepared.len() >= total,
        prepared: c.prepared.len(),
        total,
        kind,
        level: c.level,
        half: c.half,
        timed: c.timed,
        finished,
        parts,
        answers,
    })
}

/// Records (or changes, or with `None` clears) an answer.
pub fn answer(store: &Store, test_id: &str, item_id: &str, choice: Option<usize>) -> Result<()> {
    let key: Option<String> = store
        .conn
        .query_row("SELECT item FROM test_items WHERE test_id = ?1 AND item_id = ?2", [test_id, item_id], |r| r.get(0))
        .optional()?;
    let Some(j) = key else { return Ok(()) };
    let it: Item = serde_json::from_str(&j)?;
    store.conn.execute(
        "UPDATE test_items SET choice = ?3, correct = ?4, answered_at = ?5 WHERE test_id = ?1 AND item_id = ?2",
        params![test_id, item_id, choice.map(|c| c as i64), choice.map(|c| (c == it.answer) as i64), choice.map(|_| now_ms())],
    )?;
    Ok(())
}

/// Stores the time used in a part, and whether the learner has handed it in.
pub fn time(store: &Store, test_id: &str, part: usize, used_secs: u64, submitted: bool) -> Result<()> {
    let (_, mut c) = config(store, test_id)?;
    if c.used_secs.len() <= part {
        c.used_secs.resize(part + 1, 0);
        c.submitted.resize(part + 1, false);
    }
    c.used_secs[part] = used_secs;
    c.submitted[part] = c.submitted[part] || submitted;
    save_config(store, test_id, &c)
}

// --- scoring ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    LikelyPass,
    Borderline,
    Unlikely,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSection {
    pub name: String,
    pub right: usize,
    pub total: usize,
    pub score: u32,
    pub max: u32,
    pub minimum: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grade {
    pub name: String,
    pub right: usize,
    pub total: usize,
    /// A (67% and more), B (34-67%), C (under 34%), as on the JLPT score report.
    pub grade: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScore {
    pub mondai: Mondai,
    pub name_ja: String,
    pub right: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeResult {
    pub kind: String,
    pub level: u8,
    pub half: bool,
    pub right: usize,
    pub total: usize,
    pub unanswered: usize,
    pub reported: usize,
    /// Score sections as on the JLPT report, without listening (empty for a retry).
    pub sections: Vec<ScoreSection>,
    pub total_score: u32,
    pub total_max: u32,
    /// The official pass mark scaled to the sections tested.
    pub pass_line: u32,
    pub verdict: Option<Verdict>,
    pub grades: Vec<Grade>,
    pub by_type: Vec<TypeScore>,
    /// Parts finished after their time ran out.
    pub overtime: Vec<String>,
}

/// Official pass mark out of 180 (listening included).
fn pass_mark(level: u8) -> f64 {
    match level {
        1 => 100.0,
        2 => 90.0,
        3 => 95.0,
        4 => 90.0,
        _ => 80.0,
    }
}

/// Margin around the scaled pass line for "borderline", as a share of the tested maximum.
const BORDERLINE: f64 = 0.06;

fn grade(right: usize, total: usize) -> char {
    let share = right as f64 / total.max(1) as f64;
    if share >= 0.67 {
        'A'
    } else if share >= 0.34 {
        'B'
    } else {
        'C'
    }
}

fn score(kind: &str, c: &Config, all: &[Stored]) -> PracticeResult {
    let counted: Vec<&Stored> = all.iter().filter(|s| !s.reported).collect();
    let right_of = |f: &dyn Fn(&Item) -> bool| {
        let sel: Vec<&&Stored> = counted.iter().filter(|s| f(&s.item)).collect();
        (sel.iter().filter(|s| s.choice == Some(s.item.answer)).count(), sel.len())
    };
    let (right, total) = right_of(&|_| true);
    let mut by_type: Vec<TypeScore> = Vec::new();
    for s in &counted {
        let ok = usize::from(s.choice == Some(s.item.answer));
        match by_type.iter_mut().find(|t| t.mondai == s.item.mondai) {
            Some(t) => {
                t.right += ok;
                t.total += 1;
            }
            None => by_type.push(TypeScore { mondai: s.item.mondai, name_ja: s.item.mondai.name_ja().into(), right: ok, total: 1 }),
        }
    }
    let mut r = PracticeResult {
        kind: kind.into(),
        level: c.level,
        half: c.half,
        right,
        total,
        unanswered: counted.iter().filter(|s| s.choice.is_none()).count(),
        reported: all.len() - counted.len(),
        sections: vec![],
        total_score: 0,
        total_max: 0,
        pass_line: 0,
        verdict: None,
        grades: vec![],
        by_type,
        overtime: vec![],
    };
    if kind != "practice" {
        return r;
    }
    let scaled = |(right, total): (usize, usize), max: u32| (max as f64 * right as f64 / total.max(1) as f64, right, total);
    let area = |i: &Item| i.mondai.area();
    let mut sections: Vec<(&str, (f64, usize, usize), u32, u32)> = if c.level >= 4 {
        vec![("言語知識（文字・語彙・文法）・読解", scaled(right_of(&|i| area(i) != Area::Listening), 120), 120, 38)]
    } else {
        vec![
            ("言語知識（文字・語彙・文法）", scaled(right_of(&|i| matches!(area(i), Area::Vocab | Area::Grammar)), 60), 60, 19),
            ("読解", scaled(right_of(&|i| area(i) == Area::Reading), 60), 60, 19),
        ]
    };
    if c.listening {
        sections.push(("聴解", scaled(right_of(&|i| area(i) == Area::Listening), 60), 60, 19));
    }
    let total_raw: f64 = sections.iter().map(|s| s.1.0).sum();
    let max: u32 = sections.iter().map(|s| s.2).sum();
    let line = pass_mark(c.level) * max as f64 / 180.0;
    let below_min = sections.iter().any(|s| s.1.0 < s.3 as f64);
    let margin = BORDERLINE * max as f64;
    r.verdict = Some(if below_min || total_raw < line - margin {
        Verdict::Unlikely
    } else if total_raw >= line + margin {
        Verdict::LikelyPass
    } else {
        Verdict::Borderline
    });
    r.sections = sections
        .iter()
        .map(|(name, (s, right, total), max, minimum)| ScoreSection {
            name: (*name).into(),
            right: *right,
            total: *total,
            score: s.round() as u32,
            max: *max,
            minimum: *minimum,
        })
        .collect();
    r.total_score = total_raw.round() as u32;
    r.total_max = max;
    r.pass_line = line.round() as u32;
    for (name, area) in [("文字・語彙", Area::Vocab), ("文法", Area::Grammar)] {
        let (right, total) = right_of(&|i| i.mondai.area() == area);
        if total > 0 {
            r.grades.push(Grade { name: name.into(), right, total, grade: grade(right, total) });
        }
    }
    if c.timed {
        r.overtime = part_names(kind, c)
            .into_iter()
            .enumerate()
            .filter(|(i, (_, minutes))| c.used_secs.get(*i).is_some_and(|u| *u > u64::from(*minutes) * 60))
            .map(|(_, (name, _))| name)
            .collect();
    }
    r
}

fn rescore(store: &Store, test_id: &str) -> Result<PracticeResult> {
    let (kind, c) = config(store, test_id)?;
    let r = score(&kind, &c, &stored(store, test_id)?);
    store.conn.execute(
        "UPDATE tests SET finished_at = coalesce(finished_at, ?2), result = ?3 WHERE id = ?1",
        params![test_id, now_ms(), serde_json::to_string(&r)?],
    )?;
    Ok(r)
}

/// Scores the test, stores the result and returns it.
pub fn finish(store: &Store, test_id: &str) -> Result<PracticeResult> {
    rescore(store, test_id)
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewItem {
    pub id: String,
    pub number: usize,
    pub mondai: Mondai,
    pub name_ja: &'static str,
    pub prompt: String,
    pub underline: Option<String>,
    pub options: Vec<String>,
    pub passage: Option<String>,
    pub group: Option<String>,
    pub answer: usize,
    pub choice: Option<usize>,
    pub explanation: String,
    pub reported: bool,
    pub word: Option<WordRef>,
    pub script: Option<Script>,
    pub spoken_options: bool,
}

/// Every question with the learner's answer, the key and the explanation.
pub fn review(store: &Store, test_id: &str) -> Result<Vec<ReviewItem>> {
    Ok(stored(store, test_id)?
        .into_iter()
        .enumerate()
        .map(|(n, s)| ReviewItem {
            id: s.item.id,
            number: n + 1,
            mondai: s.item.mondai,
            name_ja: s.item.mondai.name_ja(),
            prompt: s.item.prompt,
            underline: s.item.underline,
            options: s.item.options,
            passage: s.item.passage,
            group: s.item.group,
            answer: s.item.answer,
            choice: s.choice,
            explanation: s.item.explanation,
            reported: s.reported,
            spoken_options: s.item.mondai.spoken_options(),
            script: s.item.script,
            word: s.item.word,
        })
        .collect())
}

/// Marks a question as bad (or not); reported questions don't count. Returns the new result.
pub fn report(store: &Store, test_id: &str, item_id: &str, reported: bool) -> Result<PracticeResult> {
    if let Some(it) = stored(store, test_id)?.into_iter().find(|s| s.item.id == item_id) {
        crate::memory::bank_report(store, it.item.origin.as_deref().unwrap_or(item_id), reported)?;
    }
    store.conn.execute(
        "UPDATE test_items SET reported = ?3 WHERE test_id = ?1 AND item_id = ?2",
        params![test_id, item_id, reported as i64],
    )?;
    rescore(store, test_id)
}

/// A short untimed round with the questions missed (and not reported) in a test; no AI calls.
pub fn retry(store: &Store, test_id: &str) -> Result<String> {
    let (_, c) = config(store, test_id)?;
    let missed: Vec<Item> = stored(store, test_id)?
        .into_iter()
        .filter(|s| !s.reported && s.choice != Some(s.item.answer))
        .map(|s| Item { id: Uuid::new_v4().to_string(), part: 0, ..s.item })
        .collect();
    if missed.is_empty() {
        return Err(Error::Llm("there are no missed questions to retry".into()));
    }
    // Passages keep their own group, so a retried passage stays together.
    let id = Uuid::new_v4().to_string();
    let rc = Config { parent: Some(test_id.into()), used_secs: vec![0], submitted: vec![false], half: false, timed: false, prepared: vec![], ..c };
    store.conn.execute(
        "INSERT INTO tests (id, device_id, kind, start_level, started_at, config) VALUES (?1, ?2, 'retry', ?3, ?4, ?5)",
        params![id, store.device_id(), c.level, now_ms(), serde_json::to_string(&rc)?],
    )?;
    save_items(store, &id, &missed)?;
    Ok(id)
}

#[derive(Debug, Clone, Serialize)]
pub struct PastPractice {
    pub id: String,
    pub kind: String,
    pub level: u8,
    pub half: bool,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub ready: bool,
    pub answered: usize,
    pub items: usize,
    pub result: Option<PracticeResult>,
}

/// Practice tests and retries, newest first.
pub fn list(store: &Store) -> Result<Vec<PastPractice>> {
    let mut st = store.conn.prepare(
        "SELECT t.id, t.kind, t.started_at, t.finished_at, t.config, t.result,
                (SELECT count(*) FROM test_items i WHERE i.test_id = t.id),
                (SELECT count(choice) FROM test_items i WHERE i.test_id = t.id)
         FROM tests t WHERE t.kind IN ('practice', 'retry') ORDER BY t.started_at DESC LIMIT 30",
    )?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, Option<i64>>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, i64>(7)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, kind, started_at, finished_at, c, result, items, answered) = row?;
        let Some(c) = c.and_then(|c| serde_json::from_str::<Config>(&c).ok()) else { continue };
        let total: usize = c.layout().iter().map(|p| p.mondai.len()).sum();
        out.push(PastPractice {
            ready: kind == "retry" || c.prepared.len() >= total,
            id,
            kind,
            level: c.level,
            half: c.half,
            started_at,
            finished_at,
            answered: answered as usize,
            items: items as usize,
            result: result.and_then(|r| serde_json::from_str(&r).ok()),
        });
    }
    Ok(out)
}

pub fn delete(store: &Store, test_id: &str) -> Result<()> {
    store.conn.execute("DELETE FROM test_items WHERE test_id = ?1", [test_id])?;
    store.conn.execute("DELETE FROM tests WHERE id = ?1 AND kind IN ('practice', 'retry')", [test_id])?;
    Ok(())
}

/// The level to suggest: one above the last level check's result (the level being worked
/// towards), else the hardest level Anki knows half of.
pub fn suggested_level(store: &Store) -> Result<u8> {
    if let Some(last) = crate::proficiency::history(store)?.first() {
        // Ability 0 (below N5) … 5 (N1) → the next level up.
        return Ok(5u8.saturating_sub(last.result.level).clamp(1, 5));
    }
    Ok(crate::proficiency::suggested_start(&crate::proficiency::coverage(store)?))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn layout_counts() {
        let n3 = layout(3, false, false);
        assert_eq!(n3.len(), 2);
        assert_eq!(n3.iter().map(|p| p.minutes).sum::<u32>(), 100);
        let items: usize = n3.iter().flat_map(|p| &p.mondai).map(|(_, n)| n).sum();
        assert_eq!(items, 35 + 23 + 16);
        let half = layout(3, true, false);
        assert_eq!(half[0].minutes, 15);
        assert!(half.iter().flat_map(|p| &p.mondai).all(|(_, n)| *n >= 1));
        assert_eq!(layout(1, false, false)[0].minutes, 110);
        assert!(layout(1, false, false)[0].mondai.iter().all(|(m, _)| *m != Mondai::Orthography));
        assert_eq!(Mondai::MidText.per_passage(3, 6), 3);
        assert_eq!(Mondai::MidText.per_passage(5, 2), 2);
        assert_eq!(Mondai::LongText.per_passage(1, 4), 4);
    }

    /// Answers like the AI would: generators by schema; the checker answers 0 (the generator's
    /// key) when it sees what it needs, and 1 for reading questions without the passage.
    struct Fake {
        calls: Mutex<Vec<String>>,
    }

    impl Fake {
        fn new() -> Self {
            Fake { calls: Mutex::new(vec![]) }
        }
    }

    impl Llm for Fake {
        fn json(&self, _system: &str, user: &str, schema: &Value) -> Result<Value> {
            self.calls.lock().unwrap().push(user.to_string());
            let props: Vec<&String> = schema["properties"].as_object().unwrap().keys().collect();
            let n = |key: &str| user.lines().find_map(|l| l.strip_prefix(key)).and_then(|v| v.trim().parse::<usize>().ok()).unwrap_or(3);
            let words: Vec<String> =
                user.lines().filter_map(|l| l.split_once(". ").filter(|(i, _)| i.parse::<usize>().is_ok()).map(|(_, w)| w.split('（').next().unwrap().to_string())).collect();
            Ok(match props[0].as_str() {
                "practice_items" => {
                    let k = n("Number of items:");
                    let items: Vec<Value> = (0..k)
                        .map(|i| {
                            let w = words.get(i).cloned().unwrap_or_else(|| format!("語{i}"));
                            if user.contains("Type: 漢字読み") {
                                json!({"word": w, "sentence": format!("昨日{w}がありました。"), "underlined": w, "options": [], "answer": 0, "explanation": ""})
                            } else if user.contains("Type: 表記") || user.contains("Type: 言い換え類義") {
                                json!({"word": w, "sentence": format!("これは{w}です。"), "underlined": w, "options": [w, "甲", "乙", "丙"], "answer": 0, "explanation": "Because."})
                            } else if user.contains("Type: 用法") {
                                json!({"word": w, "sentence": w, "underlined": "", "options": ["正しい文。", "変な文一。", "変な文二。", "変な文三。"], "answer": 0, "explanation": "Because."})
                            } else {
                                json!({"word": w, "sentence": format!("毎日（ ）行きます。{i}"), "underlined": "", "options": ["駅へ", "駅を", "駅が", "駅で"], "answer": 0, "explanation": "Because."})
                            }
                        })
                        .collect();
                    json!({"practice_items": items})
                }
                "order_items" => {
                    let k = n("Number of items:");
                    json!({"order_items": (0..k).map(|i| json!({"before": format!("私は{i}"), "fragments": ["駅", "の", "前", "で"], "after": "待ちます。", "explanation": "Order."})).collect::<Vec<_>>()})
                }
                "practice_passages" => {
                    let k = n("Number of texts:");
                    let q = n("Questions per text:");
                    let grammar = user.contains("Type: 文章の文法");
                    json!({"practice_passages": (0..k).map(|t| json!({
                        "text": if grammar { (1..=q).map(|i| format!("文［{i}］。")).collect::<String>() } else { format!("田中さんは毎朝七時に起きます。{t}") },
                        "text2": "Bの文章。",
                        "questions": (1..=q).map(|i| json!({
                            "question": if grammar { format!("［{i}］") } else { format!("質問{i}") },
                            "options": ["七時", "八時", "六時", "九時"], "answer": 0, "explanation": "The text says so."
                        })).collect::<Vec<_>>()
                    })).collect::<Vec<_>>()})
                }
                "listening_items" => {
                    let k = n("Number of items:");
                    let quick = user.contains("Type: 即時応答");
                    json!({"listening_items": (0..k).map(|i| if quick {
                        json!({"intro": "", "lines": [{"speaker": "woman", "text": format!("明日、一緒に行きませんか。{i}")}], "question": "",
                               "options": ["はい、ぜひ。", "いいえ、行きました。", "はい、そうでした。"], "answer": 0, "explanation": "An invitation."})
                    } else {
                        json!({"intro": "女の人と男の人が話しています。男の人はこのあと何をしますか。",
                               "lines": [{"speaker": "woman", "text": "パンを買ってきて。"}, {"speaker": "man", "text": format!("わかった。牛乳も？{i}")}],
                               "question": "男の人はこのあと何をしますか。質問",
                               "options": ["パンを買う", "牛乳を飲む", "家に帰る", "寝る"], "answer": 0, "explanation": "She asks for bread."})
                    }).collect::<Vec<_>>()})
                }
                _ => {
                    // Per question: its options; the ★ items' key is the fragment 前.
                    let mut questions: Vec<Vec<&str>> = Vec::new();
                    for l in user.lines() {
                        if l.split_once(". ").is_some_and(|(i, _)| i.parse::<usize>().is_ok()) {
                            questions.push(vec![]);
                        } else if let (Some(q), Some((_, o))) = (questions.last_mut(), l.trim().split_once(") ")) {
                            q.push(o);
                        }
                    }
                    let sees = user.starts_with("Passage:") || user.contains("Transcript:") || !user.contains("質問");
                    json!({"answers": questions.iter().enumerate().map(|(i, opts)| {
                        let choice = opts.iter().position(|o| *o == "前").unwrap_or(if sees { 0 } else { 1 });
                        json!({"index": i, "choice": choice, "ambiguous": false})
                    }).collect::<Vec<_>>()})
                }
            })
        }
    }

    #[test]
    fn prepares_a_whole_test_and_scores_it() {
        let s = Store::open_in_memory().unwrap();
        let id = create(&s, 3, true, true, 0, false, false).unwrap();
        let llm = Fake::new();
        let mut seen = Vec::new();
        let p = prepare(&s, &llm, None, &id, |d, t| seen.push((d, t)), || false).unwrap();
        assert_eq!(p.done, p.total);
        assert_eq!(seen.last(), Some(&(p.total, p.total)));
        let sheet = sheet(&s, &id).unwrap();
        assert!(sheet.ready && !sheet.finished);
        assert_eq!(sheet.parts.len(), 2);
        // Every question type of the layout is there, in the layout's order, with its count.
        for (part, spec) in sheet.parts.iter().zip(layout(3, true, false)) {
            let got: Vec<(Mondai, usize)> = part.groups.iter().map(|g| (g.mondai, g.items.len())).collect();
            assert_eq!(got, spec.mondai, "{}", part.name);
        }
        let all = stored(&s, &id).unwrap();
        for st in &all {
            let it = &st.item;
            assert_eq!(it.options.len(), 4, "{it:?}");
            assert!(it.answer < 4);
            if it.mondai == Mondai::KanjiReading {
                let w = it.word.as_ref().unwrap();
                assert_eq!(it.options[it.answer], w.reading);
                assert!(it.prompt.contains(&w.expression));
            }
            if it.mondai == Mondai::SentenceOrder {
                assert_eq!(it.options[it.answer], "前", "★ is the third fragment");
                assert!(it.prompt.contains('★'));
            }
            if it.mondai.area() == Area::Reading {
                assert!(it.passage.is_some() && it.group.is_some());
            }
        }
        // Numbers run through the test.
        let numbers: Vec<usize> = sheet.parts.iter().flat_map(|p| &p.groups).flat_map(|g| &g.items).map(|i| i.number).collect();
        assert_eq!(numbers, (1..=all.len()).collect::<Vec<_>>());

        // Everything right: a likely pass.
        for st in &all {
            answer(&s, &id, &st.item.id, Some(st.item.answer)).unwrap();
        }
        time(&s, &id, 0, 20 * 60, true).unwrap();
        let r = finish(&s, &id).unwrap();
        assert_eq!(r.verdict, Some(Verdict::LikelyPass));
        assert_eq!((r.right, r.total), (all.len(), all.len()));
        assert_eq!(r.sections.len(), 2);
        assert_eq!(r.total_max, 120);
        assert_eq!(r.pass_line, 63);
        assert_eq!(r.overtime, vec!["言語知識（文字・語彙）".to_string()], "20 minutes in a 15-minute part");
        assert!(r.grades.iter().all(|g| g.grade == 'A'));
        assert_eq!(list(&s).unwrap()[0].result.as_ref().unwrap().right, all.len());

        // Reporting a question takes it out of the score.
        let r2 = report(&s, &id, &all[0].item.id, true).unwrap();
        assert_eq!((r2.total, r2.reported), (all.len() - 1, 1));
    }

    #[test]
    fn reading_questions_answerable_without_the_text_are_dropped() {
        let s = Store::open_in_memory().unwrap();
        let _ = s;
        let llm = Fake::new();
        let job = Job { mondai: Mondai::ShortText, level: 4, part: 0, need: 2, words: vec![], reused: vec![] };
        let all = passage_items(&llm, None, &job).unwrap();
        assert_eq!(all.len(), 3, "every checked question is returned, extras for the bank");
        let (items, rest) = take(all, job.need);
        assert_eq!((items.len(), rest.len()), (2, 1));
        assert!(items.iter().all(|i| i.passage.as_deref().is_some_and(|p| p.contains("七時"))));
        // A checker that also answers right without the text: nothing survives.
        struct Knows;
        impl Llm for Knows {
            fn json(&self, s: &str, u: &str, schema: &Value) -> Result<Value> {
                let mut v = Fake::new().json(s, u, schema)?;
                if let Some(a) = v["answers"].as_array_mut() {
                    for x in a {
                        x["choice"] = json!(0);
                    }
                }
                Ok(v)
            }
        }
        assert!(passage_items(&Knows, None, &job).unwrap().is_empty());
    }

    #[test]
    fn text_grammar_fills_blanks_whose_question_failed() {
        struct Half;
        impl Llm for Half {
            fn json(&self, s: &str, u: &str, schema: &Value) -> Result<Value> {
                let mut v = Fake::new().json(s, u, schema)?;
                if let Some(a) = v["answers"].as_array_mut() {
                    a[1]["ambiguous"] = json!(true);
                }
                Ok(v)
            }
        }
        let job = Job { mondai: Mondai::TextGrammar, level: 5, part: 1, need: 3, words: vec![], reused: vec![] };
        let items = passage_items(&Half, None, &job).unwrap();
        assert!(!items.is_empty());
        let text = items[0].passage.clone().unwrap();
        assert!(!text.contains("［2］") && text.contains("文七時"), "{text}");
        assert!(items.iter().all(|i| i.prompt != "［2］"));
    }

    #[test]
    fn level_drift_counts_hard_list_words() {
        let Some(t) = crate::tokenize::test_support::sudachi() else {
            eprintln!("ANKURA_SUDACHI_DIC not set; skipping");
            return;
        };
        assert_eq!(drift(t, "毎日駅まで歩いて行きます。", 5, None), 0);
        // 概念 and 妥協 are N1 words: too hard for N4, fine for N2.
        assert_eq!(drift(t, "その概念について妥協しました。", 4, None), 2);
        assert_eq!(drift(t, "その概念について妥協しました。", 4, Some("概念")), 1);
        assert_eq!(drift(t, "その概念について妥協しました。", 2, None), 0);
    }

    #[test]
    fn scoring_applies_pass_line_and_minimums() {
        let c = Config {
            level: 2,
            half: false,
            timed: false,
            prepared: vec![],
            used_secs: vec![],
            submitted: vec![],
            parent: None,
            reuse: 0,
            repeat_seen: false,
            from_bank: 0,
            listening: false,
        };
        let mk = |m: Mondai, right: bool| Stored {
            item: new_item(m, 2, "q".into(), vec!["a".into(); 4], 0),
            choice: Some(if right { 0 } else { 1 }),
            reported: false,
        };
        // All knowledge right, reading 3 of 10: reading is 18/60, under the minimum of 19.
        let mut all: Vec<Stored> = (0..10).map(|_| mk(Mondai::Context, true)).collect();
        all.extend((0..10).map(|i| mk(Mondai::MidText, i < 3)));
        let r = score("practice", &c, &all);
        assert_eq!(r.sections[1].score, 18);
        assert_eq!(r.verdict, Some(Verdict::Unlikely));
        // 6 of 10 in both: 72/120 against a line of 60 ± 7.2 → likely pass.
        let all: Vec<Stored> = (0..20).map(|i| mk(if i < 10 { Mondai::Context } else { Mondai::MidText }, i % 10 < 6)).collect();
        assert_eq!(score("practice", &c, &all).verdict, Some(Verdict::LikelyPass));
        // 5 of 10: 60/120, right on the line → borderline.
        let all: Vec<Stored> = (0..20).map(|i| mk(if i < 10 { Mondai::Context } else { Mondai::MidText }, i % 10 < 5)).collect();
        let r = score("practice", &c, &all);
        assert_eq!((r.verdict, r.total_score, r.pass_line), (Some(Verdict::Borderline), 60, 60));
        assert_eq!(r.grades[0].grade, 'B');
    }

    #[test]
    fn retry_has_only_missed_questions() {
        let s = Store::open_in_memory().unwrap();
        let id = create(&s, 5, true, false, 0, false, false).unwrap();
        prepare(&s, &Fake::new(), None, &id, |_, _| {}, || false).unwrap();
        let all = stored(&s, &id).unwrap();
        for (i, st) in all.iter().enumerate() {
            answer(&s, &id, &st.item.id, Some(if i % 2 == 0 { st.item.answer } else { (st.item.answer + 1) % 4 })).unwrap();
        }
        finish(&s, &id).unwrap();
        let r = retry(&s, &id).unwrap();
        let sh = sheet(&s, &r).unwrap();
        assert!(sh.ready && sh.kind == "retry" && sh.parts.len() == 1);
        let n: usize = sh.parts[0].groups.iter().map(|g| g.items.len()).sum();
        assert_eq!(n, all.len() / 2);
        let res = finish(&s, &r).unwrap();
        assert!(res.verdict.is_none() && res.sections.is_empty());
        assert_eq!(list(&s).unwrap().len(), 2);
        delete(&s, &r).unwrap();
        assert_eq!(list(&s).unwrap().len(), 1);
    }

    #[test]
    fn listening_part_is_written_and_scored() {
        assert_eq!(layout(3, false, true).last().map(|p| (p.name, p.minutes)), Some(("聴解", 40)));
        let s = Store::open_in_memory().unwrap();
        let id = create(&s, 3, true, false, 0, false, true).unwrap();
        let p = prepare(&s, &Fake::new(), None, &id, |_, _| {}, || false).unwrap();
        assert_eq!(p.done, p.total);
        let sh = sheet(&s, &id).unwrap();
        assert_eq!(sh.parts.len(), 3);
        let listening = &sh.parts[2];
        assert_eq!(
            listening.groups.iter().map(|g| (g.mondai, g.items.len())).collect::<Vec<_>>(),
            layout(3, true, true)[2].mondai
        );
        for g in &listening.groups {
            for it in &g.items {
                let sc = it.script.as_ref().expect("a script");
                assert!(!sc.lines.is_empty());
                assert_eq!(it.spoken_options, matches!(g.mondai, Mondai::GistListening | Mondai::QuickResponse));
                assert_eq!(it.options.len(), if g.mondai == Mondai::QuickResponse { 3 } else { 4 });
            }
        }
        // All right: three sections out of 180 with the official pass mark (N3: 95).
        for st in stored(&s, &id).unwrap() {
            answer(&s, &id, &st.item.id, Some(st.item.answer)).unwrap();
        }
        let r = finish(&s, &id).unwrap();
        assert_eq!(r.sections.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["言語知識（文字・語彙・文法）", "読解", "聴解"]);
        assert_eq!((r.total_max, r.pass_line, r.verdict), (180, 95, Some(Verdict::LikelyPass)));
    }

    #[test]
    fn later_tests_reuse_the_bank() {
        let s = Store::open_in_memory().unwrap();
        let llm = Fake::new();
        let first = create(&s, 4, true, false, 0, false, false).unwrap();
        prepare(&s, &llm, None, &first, |_, _| {}, || false).unwrap();
        let first_calls = llm.calls.lock().unwrap().len();
        let n = stored(&s, &first).unwrap().len();
        // Every checked question is banked, the extras unused.
        let counts = crate::memory::bank_counts(&s, "practice").unwrap();
        assert!(counts[&4].total > n && counts[&4].unseen == counts[&4].total - n, "{counts:?}");

        // Bank first, unseen only: the extras come back, the AI writes less.
        let p = plan(&s, 4, true, 100, false, false).unwrap();
        assert!(p.from_bank > 0 && p.from_bank + p.new == n, "{p:?}");
        let second = create(&s, 4, true, false, 100, false, false).unwrap();
        let r = prepare(&s, &llm, None, &second, |_, _| {}, || false).unwrap();
        // The plan is an estimate: new questions take words that can rule out a bank question.
        assert!(r.from_bank.abs_diff(p.from_bank) <= 2, "{r:?} {p:?}");
        assert!(llm.calls.lock().unwrap().len() - first_calls < first_calls);
        let firsts: HashSet<String> = stored(&s, &first).unwrap().into_iter().filter_map(|x| x.item.origin).collect();
        let seconds = stored(&s, &second).unwrap();
        assert_eq!(seconds.len(), n);
        assert!(seconds.iter().all(|x| x.item.origin.as_ref().is_some_and(|o| !firsts.contains(o))), "nothing seen is repeated");
        // Reused passages stay whole and in the sheet's layout.
        let sh = sheet(&s, &second).unwrap();
        for (part, spec) in sh.parts.iter().zip(layout(4, true, false)) {
            assert_eq!(part.groups.iter().map(|g| (g.mondai, g.items.len())).collect::<Vec<_>>(), spec.mondai);
        }

        // Repeating seen questions allowed: no AI at all.
        let before = llm.calls.lock().unwrap().len();
        let third = create(&s, 4, true, false, 100, true, false).unwrap();
        let r = prepare(&s, &llm, None, &third, |_, _| {}, || false).unwrap();
        assert_eq!(r.done, r.total);
        // Nearly all from the bank; a type whose bank questions share words with another type's
        // may need one written (the test bank is tiny).
        assert!(r.from_bank + 2 >= n, "{r:?}");
        if r.from_bank == n {
            assert_eq!(llm.calls.lock().unwrap().len(), before);
        }
        assert_eq!(stored(&s, &third).unwrap().len(), n);
        assert!(crate::memory::stats(&s).unwrap().reused["practice_items"] >= n - 2);

        // A reported question never comes back.
        let bad = seconds[0].item.clone();
        report(&s, &second, &bad.id, true).unwrap();
        let rows = crate::memory::bank_candidates(&s, "practice", &kind(bad.mondai), 4, true).unwrap();
        assert!(rows.iter().all(|r| Some(&r.id) != bad.origin.as_ref()));
    }

    #[test]
    fn stopping_and_continuing() {
        let s = Store::open_in_memory().unwrap();
        let id = create(&s, 4, true, true, 0, false, false).unwrap();
        let llm = Fake::new();
        // Stops after the first round of PARALLEL types.
        let rounds = std::cell::Cell::new(0);
        let p = prepare(&s, &llm, None, &id, |_, _| {}, || {
            rounds.set(rounds.get() + 1);
            rounds.get() > 1
        })
        .unwrap();
        assert_eq!(p.done, PARALLEL);
        assert!(!sheet(&s, &id).unwrap().ready);
        let p = prepare(&s, &llm, None, &id, |_, _| {}, || false).unwrap();
        assert_eq!(p.done, p.total);
        assert!(sheet(&s, &id).unwrap().ready);
    }
}
