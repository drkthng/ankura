//! The level check (ADR 0014): a 15-20 minute JLPT-style estimate of the learner's level.
//! Vocabulary (yes/no, with made-up words) and kanji readings come from bundled JLPT lists
//! (Jonathan Waller, tanos.co.uk, CC BY); grammar and reading items are written by the AI and
//! checked by a second AI call. Scoring is a latent-level model: each possible level gives each
//! answer a probability, and the product says how likely each level is.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::kana::is_kanji;
use crate::llm::Llm;
use crate::store::{Store, now_ms};
use crate::{Error, Result};

// --- bundled lists: level 5 = N5 … 1 = N1 ---

pub(crate) struct ListWord {
    pub expression: String,
    pub reading: String,
    pub level: u8,
}

pub(crate) static VOCAB: LazyLock<Vec<ListWord>> = LazyLock::new(|| {
    include_str!("../data/jlpt_vocab.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some(ListWord { expression: f.next()?.into(), reading: f.next()?.into(), level: f.next()?.parse().ok()? })
        })
        .collect()
});

static KANJI: LazyLock<Vec<(char, u8)>> = LazyLock::new(|| {
    include_str!("../data/jlpt_kanji.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?.chars().next()?, f.next()?.parse().ok()?))
        })
        .collect()
});

pub const CREDIT: &str = "Word and kanji levels: JLPT lists by Jonathan Waller (tanos.co.uk), CC BY.";

/// "N3" for level 3; 0 is "below N5".
pub fn level_name(level: u8) -> String {
    if level == 0 { "below N5".into() } else { format!("N{level}") }
}

/// Ability on a 0-5 scale (0 below N5, 1 N5 … 5 N1) for a JLPT level (5 … 1).
fn rank(level: u8) -> u8 {
    6 - level
}

/// A small random-number generator (xorshift), seeded per test.
pub(crate) struct Rng(pub(crate) u64);
impl Rng {
    pub(crate) fn new() -> Self {
        Rng((Uuid::new_v4().as_u128() as u64) | 1)
    }
    #[cfg(test)]
    pub(crate) fn seeded(s: u64) -> Self {
        Rng(s | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub(crate) fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }
}

// --- items ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    Vocab,
    Kanji,
    Grammar1,
    Grammar2,
    Reading,
}

impl Section {
    fn as_str(self) -> &'static str {
        match self {
            Section::Vocab => "vocab",
            Section::Kanji => "kanji",
            Section::Grammar1 => "grammar1",
            Section::Grammar2 => "grammar2",
            Section::Reading => "reading",
        }
    }
}

/// An item with its key; the UI gets `ItemView` (no key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub section: Section,
    /// JLPT level of the item (5 … 1).
    pub level: u8,
    pub prompt: String,
    pub options: Vec<String>,
    pub answer: usize,
    /// A made-up word (vocabulary section).
    #[serde(default)]
    pub pseudo: bool,
    #[serde(default)]
    pub passage: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemView {
    pub id: String,
    pub section: Section,
    pub prompt: String,
    pub options: Vec<String>,
    pub passage: Option<String>,
}

impl From<&Item> for ItemView {
    fn from(i: &Item) -> Self {
        ItemView { id: i.id.clone(), section: i.section, prompt: i.prompt.clone(), options: i.options.clone(), passage: i.passage.clone() }
    }
}

fn item(section: Section, level: u8, prompt: String, options: Vec<String>, answer: usize) -> Item {
    Item { id: Uuid::new_v4().to_string(), section, level, prompt, options, answer, pseudo: false, passage: None }
}

/// What the learner knows according to Anki and their marks (spellings), to skip in the test.
pub(crate) fn known_spellings(store: &Store) -> Result<HashSet<String>> {
    let mut st = store.conn.prepare(
        "SELECT lemma FROM knowledge WHERE lang = 'ja' AND state IN ('known', 'fading')
         UNION SELECT display FROM knowledge WHERE lang = 'ja' AND state IN ('known', 'fading') AND display IS NOT NULL
         UNION SELECT lemma FROM word_overrides WHERE lang = 'ja' AND state = 'marked_known'",
    )?;
    let rows = st.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone, Serialize)]
pub struct LevelCoverage {
    pub level: u8,
    pub words: usize,
    pub known_in_anki: usize,
}

/// Per level, how many of the list's words Anki (or the learner's marks) says they know.
pub fn coverage(store: &Store) -> Result<Vec<LevelCoverage>> {
    let known = known_spellings(store)?;
    Ok((1..=5)
        .rev()
        .map(|level| {
            let words: Vec<&ListWord> = VOCAB.iter().filter(|w| w.level == level).collect();
            LevelCoverage {
                level,
                words: words.len(),
                known_in_anki: words.iter().filter(|w| known.contains(&w.expression)).count(),
            }
        })
        .collect())
}

/// The level to start at: the hardest level of which Anki knows at least half, else N5.
pub fn suggested_start(cov: &[LevelCoverage]) -> u8 {
    cov.iter()
        .filter(|c| c.words > 0 && c.known_in_anki * 2 >= c.words)
        .map(|c| c.level)
        .min()
        .unwrap_or(5)
}

const VOCAB_PER_LEVEL: usize = 8;
const PSEUDOWORDS: usize = 16;
const KANJI_PER_LEVEL: usize = 3;

fn vocab_items(store: &Store, rng: &mut Rng) -> Result<Vec<Item>> {
    let known = known_spellings(store)?;
    let mut out = Vec::new();
    for level in (1..=5).rev() {
        let mut pool: Vec<&ListWord> =
            VOCAB.iter().filter(|w| w.level == level && !known.contains(&w.expression)).collect();
        rng.shuffle(&mut pool);
        let mut seen = HashSet::new();
        for w in pool.into_iter().filter(|w| seen.insert(w.expression.clone())).take(VOCAB_PER_LEVEL) {
            out.push(item(Section::Vocab, level, w.expression.clone(), vec![], 0));
        }
    }
    let real: HashSet<&str> = VOCAB.iter().map(|w| w.expression.as_str()).collect();
    let common: Vec<char> = KANJI.iter().filter(|(_, l)| *l >= 3).map(|(c, _)| *c).collect();
    let mut tries = 0;
    let mut made = HashSet::new();
    while made.len() < PSEUDOWORDS && tries < 2000 {
        tries += 1;
        let w: String = [common[rng.below(common.len())], common[rng.below(common.len())]].iter().collect();
        if w.chars().next() == w.chars().nth(1) || real.contains(w.as_str()) || known.contains(&w) || made.contains(&w) {
            continue;
        }
        // No dictionary word, either (when a dictionary is installed).
        if !crate::dict::lookup(store, &w, "")?.iter().all(|e| e.expression != w) {
            continue;
        }
        made.insert(w.clone());
        let mut it = item(Section::Vocab, 5, w, vec![], 1);
        it.pseudo = true;
        out.push(it);
    }
    rng.shuffle(&mut out);
    Ok(out)
}

/// Plausible wrong readings: voicing, long vowels, small っ.
pub(crate) fn distractors(reading: &str) -> Vec<String> {
    const VOICE: &[(char, char)] = &[
        ('か', 'が'), ('き', 'ぎ'), ('く', 'ぐ'), ('け', 'げ'), ('こ', 'ご'), ('さ', 'ざ'), ('し', 'じ'), ('す', 'ず'), ('せ', 'ぜ'),
        ('そ', 'ぞ'), ('た', 'だ'), ('ち', 'ぢ'), ('つ', 'づ'), ('て', 'で'), ('と', 'ど'), ('は', 'ば'), ('ひ', 'び'), ('ふ', 'ぶ'),
        ('へ', 'べ'), ('ほ', 'ぼ'),
    ];
    let chars: Vec<char> = reading.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut push = |s: String| {
        if s != reading && !out.contains(&s) && !s.is_empty() {
            out.push(s);
        }
    };
    // Voice or unvoice one mora.
    for (i, c) in chars.iter().enumerate() {
        if let Some((a, b)) = VOICE.iter().find(|(a, b)| a == c || b == c) {
            let mut v = chars.clone();
            v[i] = if c == a { *b } else { *a };
            push(v.into_iter().collect());
        }
    }
    // Lengthen or shorten a vowel (きょう ↔ きょ, しゅう ↔ しゅ, こう ↔ こ).
    for i in 0..chars.len() {
        if chars[i] == 'う' && i > 0 {
            let mut v = chars.clone();
            v.remove(i);
            push(v.into_iter().collect());
        } else if "こそとのほもよろごぞどぼょゅくすつぬふむゆるぐずぶ".contains(chars[i]) && chars.get(i + 1) != Some(&'う') {
            let mut v = chars.clone();
            v.insert(i + 1, 'う');
            push(v.into_iter().collect());
        }
    }
    // Add or drop a small っ.
    for i in 1..chars.len() {
        let mut v = chars.clone();
        if chars[i] == 'っ' {
            v.remove(i);
        } else if chars[i - 1] != 'っ' && "かきくけこさしすせそたちつてとぱぴぷぺぽ".contains(chars[i]) {
            v.insert(i, 'っ');
        } else {
            continue;
        }
        push(v.into_iter().collect());
    }
    out
}

fn kanji_items(rng: &mut Rng) -> Vec<Item> {
    let mut out = Vec::new();
    for level in (1..=5).rev() {
        let mut pool: Vec<&ListWord> = VOCAB
            .iter()
            .filter(|w| w.level == level && w.expression.chars().all(is_kanji) && w.expression.chars().count() >= 2)
            .collect();
        rng.shuffle(&mut pool);
        let mut n = 0;
        for w in pool {
            let mut d = distractors(&w.reading);
            if d.len() < 3 {
                continue;
            }
            rng.shuffle(&mut d);
            let mut options: Vec<String> = d.into_iter().take(3).collect();
            options.push(w.reading.clone());
            rng.shuffle(&mut options);
            let answer = options.iter().position(|o| *o == w.reading).expect("added");
            out.push(item(Section::Kanji, level, w.expression.clone(), options, answer));
            n += 1;
            if n == KANJI_PER_LEVEL {
                break;
            }
        }
    }
    rng.shuffle(&mut out);
    out
}

// --- AI-written items, checked by a second call ---

const GRAMMAR_SYSTEM: &str = "You write grammar questions for a Japanese level check, in the style of the JLPT's \
文法形式の判断 (choose the form that fits the blank). Write your own questions; never copy official JLPT questions. \
Each item is one natural Japanese sentence with exactly one blank written （　）, and four options. Exactly one \
option fits both grammatically and in meaning; the other three are plausible forms of the same level that clearly \
don't fit. The grammar point and the vocabulary must be at the given level or easier. The answer must not be \
guessable from the sentence without knowing the grammar. Vary the grammar points.";

const READING_SYSTEM: &str = "You write reading passages for a Japanese level check, in the style of the JLPT's \
読解 section. Write your own passages; never copy official ones. Each passage is a short, natural text (a note, an \
email, a notice, a short essay) at the given level, with two multiple-choice questions in Japanese, four options \
each. Every question must need the passage: someone who hasn't read it must not be able to answer from general \
knowledge. Exactly one option is right.";

const SOLVE_SYSTEM: &str = "You check Japanese test questions. Answer each question. If more than one option could be \
right, or none is, mark it ambiguous.";

pub(crate) fn mc_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"question": {"type": "string"}, "options": {"type": "array", "items": {"type": "string"}}, "answer": {"type": "integer"}},
        "required": ["question", "options", "answer"],
        "additionalProperties": false
    })
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Mc {
    pub(crate) question: String,
    pub(crate) options: Vec<String>,
    pub(crate) answer: usize,
}

fn solve_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "answers": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {"index": {"type": "integer"}, "choice": {"type": "integer"}, "ambiguous": {"type": "boolean"}},
                    "required": ["index", "choice", "ambiguous"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["answers"],
        "additionalProperties": false
    })
}

/// The second call's answers (index → choice, ambiguous) for numbered questions.
pub(crate) fn solve(llm: &dyn Llm, questions: &str) -> Result<HashMap<usize, (usize, bool)>> {
    let v = llm.json(SOLVE_SYSTEM, questions, &solve_schema())?;
    Ok(v["answers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| Some((a["index"].as_u64()? as usize, (a["choice"].as_u64()? as usize, a["ambiguous"].as_bool().unwrap_or(false)))))
        .collect())
}

pub(crate) fn numbered(items: &[Mc], passage: Option<&str>) -> String {
    let mut s = String::new();
    if let Some(p) = passage {
        s.push_str(&format!("Passage:\n{p}\n\n"));
    }
    for (i, m) in items.iter().enumerate() {
        s.push_str(&format!("{i}. {}\n", m.question));
        for (j, o) in m.options.iter().enumerate() {
            s.push_str(&format!("   {j}) {o}\n"));
        }
    }
    s
}

pub(crate) fn valid(m: &Mc) -> bool {
    m.options.len() == 4 && m.answer < 4 && !m.question.trim().is_empty() && m.options.iter().collect::<HashSet<_>>().len() == 4
}

/// Shuffles the options so the key isn't always in the same place.
pub(crate) fn shuffled(m: &Mc, rng: &mut Rng) -> (Vec<String>, usize) {
    let mut idx: Vec<usize> = (0..4).collect();
    rng.shuffle(&mut idx);
    let options = idx.iter().map(|&i| m.options[i].clone()).collect();
    let answer = idx.iter().position(|&i| i == m.answer).expect("permutation");
    (options, answer)
}

pub const GRAMMAR_STAGE1: usize = 8;
pub const GRAMMAR_STAGE2: usize = 12;

/// Grammar items at a level, written by the AI; items the checking call answers differently or
/// finds ambiguous are dropped.
fn grammar_items(llm: &dyn Llm, level: u8, n: usize, section: Section, rng: &mut Rng) -> Result<Vec<Item>> {
    let schema = json!({
        "type": "object",
        "properties": {"items": {"type": "array", "items": mc_schema()}},
        "required": ["items"],
        "additionalProperties": false
    });
    let prompt = format!("Level: N{level}\nNumber of items: {}\n", n + 4);
    let v = llm.json(GRAMMAR_SYSTEM, &prompt, &schema)?;
    let items: Vec<Mc> = serde_json::from_value::<Vec<Mc>>(v["items"].clone())
        .map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?
        .into_iter()
        .filter(|m| valid(m) && m.question.contains('（'))
        .collect();
    let checked = solve(llm, &numbered(&items, None))?;
    Ok(items
        .iter()
        .enumerate()
        .filter(|(i, m)| checked.get(i).is_some_and(|(c, amb)| *c == m.answer && !amb))
        .take(n)
        .map(|(_, m)| {
            let (options, answer) = shuffled(m, rng);
            item(section, level, m.question.clone(), options, answer)
        })
        .collect())
}

const PASSAGES: usize = 2;

fn reading_items(llm: &dyn Llm, level: u8, rng: &mut Rng) -> Result<Vec<Item>> {
    let schema = json!({
        "type": "object",
        "properties": {
            "passages": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {"text": {"type": "string"}, "questions": {"type": "array", "items": mc_schema()}},
                    "required": ["text", "questions"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["passages"],
        "additionalProperties": false
    });
    let length = match level {
        5 | 4 => "about 150-250 characters",
        3 => "about 300-400 characters",
        _ => "about 400-550 characters",
    };
    let v = llm.json(READING_SYSTEM, &format!("Level: N{level}\nPassages: {}\nLength: {length}\n", PASSAGES + 1), &schema)?;
    #[derive(Deserialize)]
    struct Passage {
        text: String,
        questions: Vec<Mc>,
    }
    let passages: Vec<Passage> = serde_json::from_value(v["passages"].clone())
        .map_err(|e| Error::Llm(format!("the answer didn't match the expected shape: {e}")))?;
    let mut out = Vec::new();
    for p in passages {
        if out.len() >= PASSAGES * 2 {
            break;
        }
        let qs: Vec<Mc> = p.questions.into_iter().filter(valid).take(2).collect();
        if qs.is_empty() {
            continue;
        }
        // Must be right with the passage, and not answerable without it.
        let with = solve(llm, &numbered(&qs, Some(&p.text)))?;
        let without = solve(llm, &numbered(&qs, None))?;
        for (i, m) in qs.iter().enumerate() {
            let right_with = with.get(&i).is_some_and(|(c, amb)| *c == m.answer && !amb);
            let right_without = without.get(&i).is_some_and(|(c, _)| *c == m.answer);
            if right_with && !right_without {
                let (options, answer) = shuffled(m, rng);
                let mut it = item(Section::Reading, level, m.question.clone(), options, answer);
                it.passage = Some(p.text.clone());
                out.push(it);
            }
        }
    }
    Ok(out)
}

// --- tests: stored with every answer (ADR 0014 point 6) ---

pub(crate) const TESTS_SCHEMA: &str = r#"
CREATE TABLE tests (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    kind TEXT NOT NULL,            -- 'level_check'
    start_level INTEGER NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    result TEXT                    -- JSON (Estimate)
);
CREATE TABLE test_items (
    test_id TEXT NOT NULL,
    item_id TEXT NOT NULL,
    section TEXT NOT NULL,
    level INTEGER NOT NULL,
    item TEXT NOT NULL,            -- JSON (Item, with the key)
    choice INTEGER,
    correct INTEGER,
    answered_at INTEGER,
    PRIMARY KEY (test_id, item_id)
);
"#;

fn save_items(store: &Store, test_id: &str, items: &[Item]) -> Result<()> {
    for it in items {
        store.conn.execute(
            "INSERT INTO test_items (test_id, item_id, section, level, item) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![test_id, it.id, it.section.as_str(), it.level, serde_json::to_string(it)?],
        )?;
    }
    Ok(())
}

fn items(store: &Store, test_id: &str) -> Result<Vec<(Item, Option<usize>)>> {
    let mut st = store.conn.prepare("SELECT item, choice FROM test_items WHERE test_id = ?1 ORDER BY rowid")?;
    let rows = st.query_map([test_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (j, c) = row?;
        out.push((serde_json::from_str(&j)?, c.map(|c| c as usize)));
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct Started {
    pub id: String,
    pub start_level: u8,
    pub vocab: Vec<ItemView>,
    pub kanji: Vec<ItemView>,
}

/// Starts a level check at a level (5 … 1): the vocabulary and kanji sections.
pub fn start(store: &Store, start_level: u8) -> Result<Started> {
    let start_level = start_level.clamp(1, 5);
    let id = Uuid::new_v4().to_string();
    store.conn.execute(
        "INSERT INTO tests (id, device_id, kind, start_level, started_at) VALUES (?1, ?2, 'level_check', ?3, ?4)",
        params![id, store.device_id(), start_level, now_ms()],
    )?;
    let mut rng = Rng::new();
    let vocab = vocab_items(store, &mut rng)?;
    let kanji = kanji_items(&mut rng);
    save_items(store, &id, &vocab)?;
    save_items(store, &id, &kanji)?;
    Ok(Started { id, start_level, vocab: vocab.iter().map(ItemView::from).collect(), kanji: kanji.iter().map(ItemView::from).collect() })
}

#[derive(Debug, Clone, Deserialize)]
pub struct Answer {
    pub item_id: String,
    pub choice: usize,
}

pub fn answer(store: &Store, test_id: &str, answers: &[Answer]) -> Result<()> {
    let all: HashMap<String, Item> = items(store, test_id)?.into_iter().map(|(i, _)| (i.id.clone(), i)).collect();
    for a in answers {
        let Some(it) = all.get(&a.item_id) else { continue };
        store.conn.execute(
            "UPDATE test_items SET choice = ?3, correct = ?4, answered_at = ?5 WHERE test_id = ?1 AND item_id = ?2",
            params![test_id, a.item_id, a.choice as i64, (a.choice == it.answer) as i64, now_ms()],
        )?;
    }
    Ok(())
}

fn start_level(store: &Store, test_id: &str) -> Result<u8> {
    store
        .conn
        .query_row("SELECT start_level FROM tests WHERE id = ?1", [test_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| Error::Llm("this test no longer exists".into()))
}

/// The next grammar stage. Stage 1 is at the starting level; stage 2 one level harder, the
/// same, or one easier, depending on stage 1.
pub fn grammar(store: &Store, llm: &dyn Llm, test_id: &str, stage: u8) -> Result<Vec<ItemView>> {
    let start = start_level(store, test_id)?;
    let (level, n, section) = if stage == 1 {
        (start, GRAMMAR_STAGE1, Section::Grammar1)
    } else {
        let done: Vec<bool> = items(store, test_id)?
            .into_iter()
            .filter(|(i, c)| i.section == Section::Grammar1 && c.is_some())
            .map(|(i, c)| c == Some(i.answer))
            .collect();
        let share = done.iter().filter(|c| **c).count() as f64 / done.len().max(1) as f64;
        let level = if share >= 0.75 { start.saturating_sub(1).max(1) } else if share <= 0.4 { (start + 1).min(5) } else { start };
        (level, GRAMMAR_STAGE2, Section::Grammar2)
    };
    let made = grammar_items(llm, level, n, section, &mut Rng::new())?;
    if made.is_empty() {
        return Err(Error::Llm("the AI's grammar questions didn't pass the check; try again".into()));
    }
    save_items(store, test_id, &made)?;
    Ok(made.iter().map(ItemView::from).collect())
}

/// The reading section, at the most likely level so far.
pub fn reading(store: &Store, llm: &dyn Llm, test_id: &str) -> Result<Vec<ItemView>> {
    let so_far = estimate_from(&items(store, test_id)?);
    let level = if so_far.level == 0 { 5 } else { 6 - so_far.level };
    let made = reading_items(llm, level, &mut Rng::new())?;
    save_items(store, test_id, &made)?;
    Ok(made.iter().map(ItemView::from).collect())
}

// --- scoring ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionScore {
    pub section: String,
    /// JLPT level of the items (5 … 1).
    pub level: u8,
    pub right: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estimate {
    /// Probability of each ability, 0 (below N5), 1 (N5) … 5 (N1).
    pub probs: Vec<f64>,
    /// The most likely ability (same scale).
    pub level: u8,
    pub sections: Vec<SectionScore>,
    /// Share of made-up words the learner said they know.
    pub false_alarms: f64,
    /// Estimated words of the lists known, with a rough range.
    pub vocab_known: usize,
    pub vocab_range: (usize, usize),
    pub vocab_total: usize,
    pub cefr: String,
}

/// The Japan Foundation's CEFR reference per JLPT level (reading and listening only).
fn cefr(ability: u8) -> &'static str {
    match ability {
        0 => "below A1",
        1 => "A1",
        2 => "A2",
        3 => "A2-B1",
        4 => "B1-B2",
        _ => "B2-C1",
    }
}

/// Probability of a right answer to an item `gap` levels above the learner (≤ 0: at or below):
/// they know it, or they guess right (`guess`: 1 in 4 for multiple choice, the rate of yes to
/// made-up words for the yes/no words, so saying yes to everything tells nothing).
fn p_right(gap: i32, guess: f64) -> f64 {
    let knows = match gap {
        g if g <= 0 => 0.85,
        1 => 0.35,
        _ => 0.05,
    };
    (guess + (1.0 - guess) * knows).clamp(0.02, 0.98)
}

fn estimate_from(all: &[(Item, Option<usize>)]) -> Estimate {
    let pseudo: Vec<bool> = all.iter().filter(|(i, c)| i.pseudo && c.is_some()).map(|(_, c)| *c == Some(0)).collect();
    let fa = pseudo.iter().filter(|y| **y).count() as f64 / pseudo.len().max(1) as f64;
    let mut log = [0f64; 6];
    for (it, choice) in all {
        let (Some(c), false) = (choice, it.pseudo) else { continue };
        let right = *c == it.answer;
        // A "no" to a real word is wrong either way; guessing only makes a "yes" likelier.
        let guess = if it.section == Section::Vocab { fa } else { 0.25 };
        for (ability, l) in log.iter_mut().enumerate() {
            let p = p_right(i32::from(rank(it.level)) - ability as i32, guess);
            *l += if right { p.ln() } else { (1.0 - p).ln() };
        }
    }
    let max = log.iter().cloned().fold(f64::MIN, f64::max);
    let w: Vec<f64> = log.iter().map(|l| (l - max).exp()).collect();
    let sum: f64 = w.iter().sum();
    let probs: Vec<f64> = w.iter().map(|x| x / sum).collect();
    let level = probs.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i as u8).unwrap_or(0);

    let mut sections: Vec<SectionScore> = Vec::new();
    for (it, choice) in all.iter().filter(|(i, c)| c.is_some() && !i.pseudo) {
        let name = match it.section {
            Section::Grammar1 | Section::Grammar2 => "grammar",
            s => s.as_str(),
        };
        let right = *choice == Some(it.answer);
        match sections.iter_mut().find(|s| s.section == name && s.level == it.level) {
            Some(s) => {
                s.total += 1;
                s.right += usize::from(right);
            }
            None => sections.push(SectionScore { section: name.into(), level: it.level, right: usize::from(right), total: 1 }),
        }
    }
    sections.sort_by(|a, b| a.section.cmp(&b.section).then(b.level.cmp(&a.level)));
    Estimate { probs, level, sections, false_alarms: fa, vocab_known: 0, vocab_range: (0, 0), vocab_total: 0, cefr: cefr(level).into() }
}

/// Words of the lists the learner knows: Anki's, plus the tested share (corrected for saying yes
/// to made-up words) of the rest, per level.
fn vocab_estimate(store: &Store, e: &mut Estimate) -> Result<()> {
    let cov = coverage(store)?;
    let (mut known, mut var, mut total) = (0f64, 0f64, 0usize);
    for c in &cov {
        total += c.words;
        let s = e.sections.iter().find(|s| s.section == "vocab" && s.level == c.level);
        let rest = (c.words - c.known_in_anki) as f64;
        let (h, n) = s.map(|s| (s.right as f64 / s.total.max(1) as f64, s.total as f64)).unwrap_or((0.0, 0.0));
        let corrected = if e.false_alarms < 1.0 { ((h - e.false_alarms) / (1.0 - e.false_alarms)).clamp(0.0, 1.0) } else { 0.0 };
        known += c.known_in_anki as f64 + corrected * rest;
        if n > 0.0 {
            var += (corrected * (1.0 - corrected) / n) * rest * rest;
        }
    }
    let sd = var.sqrt() * 1.96;
    e.vocab_known = known.round() as usize;
    e.vocab_range = ((known - sd).max(0.0).round() as usize, ((known + sd).round() as usize).min(total));
    e.vocab_total = total;
    Ok(())
}

/// Scores the test, stores the result and returns it.
pub fn finish(store: &Store, test_id: &str) -> Result<Estimate> {
    let mut e = estimate_from(&items(store, test_id)?);
    vocab_estimate(store, &mut e)?;
    store.conn.execute(
        "UPDATE tests SET finished_at = ?2, result = ?3 WHERE id = ?1",
        params![test_id, now_ms(), serde_json::to_string(&e)?],
    )?;
    Ok(e)
}

#[derive(Debug, Clone, Serialize)]
pub struct PastTest {
    pub id: String,
    pub finished_at: i64,
    pub result: Estimate,
}

pub fn history(store: &Store) -> Result<Vec<PastTest>> {
    let mut st = store.conn.prepare(
        "SELECT id, finished_at, result FROM tests WHERE result IS NOT NULL AND kind = 'level_check' ORDER BY finished_at DESC LIMIT 20",
    )?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))?;
    let mut out = Vec::new();
    for row in rows {
        let (id, finished_at, j) = row?;
        if let Ok(result) = serde_json::from_str(&j) {
            out.push(PastTest { id, finished_at, result });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn lists_load() {
        assert!(VOCAB.len() > 8000 && KANJI.len() > 2000);
        assert!(VOCAB.iter().any(|w| w.expression == "会う" && w.reading == "あう" && w.level == 5));
        assert_eq!(level_name(3), "N3");
        assert_eq!(level_name(0), "below N5");
    }

    #[test]
    fn distractor_rules() {
        let d = distractors("がっこう");
        assert!(d.contains(&"かっこう".to_string()) && d.contains(&"がっこ".to_string()) && d.contains(&"がこう".to_string()), "{d:?}");
        assert!(!d.contains(&"がっこう".to_string()));
        assert!(distractors("きょう").contains(&"きょ".to_string()));
    }

    #[test]
    fn builds_vocab_and_kanji_sections() {
        let s = Store::open_in_memory().unwrap();
        let mut rng = Rng::seeded(7);
        let v = vocab_items(&s, &mut rng).unwrap();
        assert_eq!(v.iter().filter(|i| !i.pseudo).count(), 40);
        assert_eq!(v.iter().filter(|i| i.pseudo).count(), PSEUDOWORDS);
        let real: HashSet<&str> = VOCAB.iter().map(|w| w.expression.as_str()).collect();
        assert!(v.iter().filter(|i| i.pseudo).all(|i| !real.contains(i.prompt.as_str()) && i.answer == 1));
        let k = kanji_items(&mut rng);
        assert_eq!(k.len(), 15);
        for it in &k {
            assert_eq!(it.options.len(), 4);
            let right = VOCAB.iter().find(|w| w.expression == it.prompt && w.reading == it.options[it.answer]);
            assert!(right.is_some(), "{it:?}");
        }
    }

    #[test]
    fn known_words_are_skipped_and_start_suggested() {
        let mut s = Store::open_in_memory().unwrap();
        let n5: Vec<&ListWord> = VOCAB.iter().filter(|w| w.level == 5).collect();
        let cards: Vec<(i64, crate::knowledge::CardFacts)> = (0..n5.len() as i64)
            .map(|i| (1, crate::knowledge::CardFacts { card_id: i, note_id: i, card_type: 2, queue: 2, interval: 100, lapses: 0, stability: None, retrievability: None }))
            .collect();
        let keys: Vec<(i64, crate::WordKey)> = n5
            .iter()
            .enumerate()
            .map(|(i, w)| (i as i64, crate::WordKey { lang: "ja".into(), lemma: w.expression.clone(), reading: w.reading.clone() }))
            .collect();
        s.replace_anki_snapshot(&cards, &keys, &HashMap::new(), &crate::knowledge::KnowledgeSettings::default()).unwrap();
        let cov = coverage(&s).unwrap();
        assert_eq!(cov[0].level, 5);
        assert!(cov[0].known_in_anki * 10 >= cov[0].words * 9, "{cov:?}");
        assert_eq!(suggested_start(&cov), 5);
        let v = vocab_items(&s, &mut Rng::seeded(3)).unwrap();
        // All of N5 is known, so no N5 word is asked (made-up words carry level 5 too).
        assert!(v.iter().filter(|i| !i.pseudo && i.level == 5).count() <= 1, "{v:?}");
    }

    struct Scripted(Mutex<Vec<Value>>);
    impl Llm for Scripted {
        fn json(&self, _s: &str, _u: &str, _schema: &Value) -> Result<Value> {
            Ok(self.0.lock().unwrap().remove(0))
        }
    }

    #[test]
    fn grammar_items_keep_only_checked_ones() {
        let q = |n: usize| json!({"question": format!("文{n}（　）です。"), "options": ["あ", "い", "う", "え"], "answer": 1});
        let llm = Scripted(Mutex::new(vec![
            json!({"items": [q(0), q(1), q(2), {"question": "no blank", "options": ["a","b","c","d"], "answer": 0}]}),
            // The check agrees on 0, disagrees on 1, finds 2 ambiguous.
            json!({"answers": [{"index": 0, "choice": 1, "ambiguous": false}, {"index": 1, "choice": 2, "ambiguous": false}, {"index": 2, "choice": 1, "ambiguous": true}]}),
        ]));
        let items = grammar_items(&llm, 3, 8, Section::Grammar1, &mut Rng::seeded(1)).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].prompt, "文0（　）です。");
        assert_eq!(items[0].options[items[0].answer], "い", "the key follows the shuffle");
    }

    #[test]
    fn scoring_finds_the_level() {
        // Right on everything up to N3, wrong above: most likely N3.
        let mut all = Vec::new();
        for level in (1..=5u8).rev() {
            for _ in 0..6 {
                let it = item(Section::Grammar1, level, "q".into(), vec!["a".into(); 4], 0);
                all.push((it, Some(if level >= 3 { 0 } else { 1 })));
            }
        }
        let mut p = item(Section::Vocab, 5, "偽語".into(), vec![], 1);
        p.pseudo = true;
        all.push((p, Some(1)));
        let e = estimate_from(&all);
        assert_eq!(level_name(6 - e.level.max(1)), "N3", "{:?}", e.probs);
        assert!(e.probs[3] > 0.7, "{:?}", e.probs);
        assert_eq!(e.cefr, "A2-B1");
        assert_eq!(e.false_alarms, 0.0);
        let g3 = e.sections.iter().find(|s| s.section == "grammar" && s.level == 3).unwrap();
        assert_eq!((g3.right, g3.total), (6, 6));
    }

    #[test]
    fn yes_to_everything_says_nothing() {
        let mut all = Vec::new();
        for level in (1..=5u8).rev() {
            for _ in 0..8 {
                all.push((item(Section::Vocab, level, "w".into(), vec![], 0), Some(0)));
            }
        }
        for _ in 0..16 {
            let mut p = item(Section::Vocab, 5, "偽".into(), vec![], 1);
            p.pseudo = true;
            all.push((p, Some(0)));
        }
        let e = estimate_from(&all);
        assert_eq!(e.false_alarms, 1.0);
        assert!(e.probs.iter().all(|p| p.is_finite() && (*p - 1.0 / 6.0).abs() < 1e-9), "{:?}", e.probs);
    }

    #[test]
    fn a_whole_test_is_stored() {
        let s = Store::open_in_memory().unwrap();
        let t = start(&s, 4).unwrap();
        let all = items(&s, &t.id).unwrap();
        let answers: Vec<Answer> = all.iter().map(|(i, _)| Answer { item_id: i.id.clone(), choice: i.answer }).collect();
        answer(&s, &t.id, &answers).unwrap();
        let e = finish(&s, &t.id).unwrap();
        assert_eq!(e.level, 5, "all right: N1 most likely ({:?})", e.probs);
        assert!(e.vocab_known > 0 && e.vocab_range.0 <= e.vocab_known && e.vocab_known <= e.vocab_range.1);
        assert_eq!(history(&s).unwrap().len(), 1);
    }
}
