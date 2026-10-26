//! Anki note fields → word keys, the mapping preview, and the Anki sync that builds Ankura's
//! knowledge table.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::anki::{AnkiSource, NoteInfo};
use crate::kana::{has_kanji, is_all_kana, to_hiragana};
use crate::knowledge::{CardFacts, KnowledgeSettings};
use crate::normalize::{clean_field, marked_text};
use crate::store::{NoteTypeMapping, Store, TargetMode};
use crate::tokenize::{TokenKind, Tokenizer};
use crate::{Result, WordKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldOutcome {
    /// One word.
    Word,
    /// Several words; stored as a phrase key (matched in the reader later, ROADMAP M3).
    Phrase,
    /// Nothing usable in the field.
    Empty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldKeys {
    pub outcome: FieldOutcome,
    pub keys: Vec<WordKey>,
    /// How each key is written on the card (降りる), in the same order as `keys`; the key's lemma
    /// is the normalized spelling (下りる) used for matching.
    #[serde(default)]
    pub displays: Vec<String>,
}

/// Word keys for one note: the target field, plus an optional reading field.
pub fn keys_for_field(tok: &dyn Tokenizer, target: &str, reading_field: Option<&str>) -> Result<FieldKeys> {
    let reading_override = reading_field
        .and_then(|r| clean_field(r).into_iter().next())
        .map(|c| c.reading_hint.unwrap_or(c.text))
        .filter(|r| is_all_kana(r))
        .map(|r| to_hiragana(&r));

    let mut keys = Vec::new();
    let mut outcome = FieldOutcome::Empty;
    for (i, cand) in clean_field(target).into_iter().enumerate() {
        let hint = reading_override.clone().filter(|_| i == 0).or(cand.reading_hint.clone());
        let tokens = tok.tokenize(&cand.text)?;
        let mut content: Vec<_> =
            tokens.iter().filter(|t| matches!(t.kind, TokenKind::Word | TokenKind::Name | TokenKind::Number)).collect();
        // むしゃくしゃして, あらわにする, 勉強できる: the light verb isn't part of the word.
        if content.len() > 1 {
            content.retain(|t| !LIGHT_VERBS.contains(&t.lemma.as_str()));
        }
        match content.as_slice() {
            [] => {}
            [t] => {
                // A reading given for the field only applies when the field is the word's own
                // dictionary form (not e.g. a conjugated example).
                let reading = match &hint {
                    Some(h) if t.surface == cand.text && t.surface == t.dictionary_form && plausible_reading(h, &cand.text) => {
                        h.clone()
                    }
                    _ => t.lemma_reading.clone(),
                };
                let display = if t.surface == cand.text { cand.text.clone() } else { t.dictionary_form.clone() };
                keys.push((WordKey { lang: tok.lang().into(), lemma: t.lemma.clone(), reading }, display));
                if outcome == FieldOutcome::Empty {
                    outcome = FieldOutcome::Word;
                }
            }
            // A sentence with nothing marked whose reading field names one of its words
            // (日本にはほとんど石油がありません + せきゆ): that word is the card's word.
            many if hint.as_ref().is_some_and(|h| many.iter().filter(|t| t.lemma_reading == *h || t.reading == *h).count() == 1) => {
                let h = hint.as_ref().unwrap();
                let t = many.iter().find(|t| t.lemma_reading == *h || t.reading == *h).unwrap();
                keys.push((
                    WordKey { lang: tok.lang().into(), lemma: t.lemma.clone(), reading: t.lemma_reading.clone() },
                    t.dictionary_form.clone(),
                ));
                if outcome == FieldOutcome::Empty {
                    outcome = FieldOutcome::Word;
                }
            }
            many => {
                let reading = hint.unwrap_or_else(|| many.iter().map(|t| t.lemma_reading.as_str()).collect());
                keys.push((WordKey { lang: tok.lang().into(), lemma: cand.text.clone(), reading }, cand.text.clone()));
                // The reader doesn't match phrases yet (ROADMAP), so the phrase's words count too:
                // a card for 脱水症状 or 途方に暮れる also tells us about 脱水, 症状, 途方, 暮れる.
                for t in many {
                    keys.push((
                        WordKey { lang: tok.lang().into(), lemma: t.lemma.clone(), reading: t.lemma_reading.clone() },
                        t.dictionary_form.clone(),
                    ));
                }
                if outcome == FieldOutcome::Empty {
                    outcome = FieldOutcome::Phrase;
                }
            }
        }
    }
    let mut seen = HashSet::new();
    keys.retain(|(k, _)| seen.insert(k.clone()));
    let (keys, displays) = keys.into_iter().unzip();
    Ok(FieldKeys { outcome, keys, displays })
}

/// Which notes count: those of the user's chosen part of the collection, and only cards they
/// study (suspended cards are left out unless the user counts them).
#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub extra_query: String,
    pub include_suspended: bool,
}

impl Scope {
    fn notes_query(&self, model_id: i64) -> String {
        let mut q = format!("mid:{model_id}");
        if !self.include_suspended {
            q.push_str(" -is:suspended");
        }
        if !self.extra_query.trim().is_empty() {
            q.push(' ');
            q.push_str(self.extra_query.trim());
        }
        q
    }
}

/// Verbs that turn the word before them into a verb or an expression (normalized lemmas).
const LIGHT_VERBS: &[&str] = &["為る", "する", "成る", "なる", "出来る", "できる", "有る", "ある", "居る", "いる"];

/// A reading for a word can't be much longer than the word (a whole-sentence reading isn't one),
/// and has to end with the word's own trailing kana (食べる → …べる).
fn plausible_reading(reading: &str, word: &str) -> bool {
    let tail: String = to_hiragana(&word.chars().rev().take_while(|c| !crate::kana::is_kanji(*c)).collect::<String>())
        .chars()
        .rev()
        .collect();
    reading.chars().count() <= word.chars().count() * 4 + 2 && reading.ends_with(&tail)
}

/// The text a mapping takes the word from: the whole field, or its marked parts (one per line).
/// A sentence with nothing marked (すみません！, どうして？) is taken whole: the sentence is what the
/// card teaches.
fn target_text(mapping: &NoteTypeMapping, fields: &BTreeMap<String, String>) -> String {
    let field = fields.get(&mapping.target_field).map(String::as_str).unwrap_or("");
    match mapping.target_mode {
        TargetMode::Field => field.to_string(),
        TargetMode::Marked => {
            let marked = marked_text(field);
            if marked.is_empty() { field.to_string() } else { marked.join("\n") }
        }
    }
}

/// Word keys for one note under a mapping.
pub fn keys_for_note(tok: &dyn Tokenizer, mapping: &NoteTypeMapping, fields: &BTreeMap<String, String>) -> Result<FieldKeys> {
    let reading = mapping.reading_field.as_ref().and_then(|f| fields.get(f)).map(String::as_str);
    keys_for_field(tok, &target_text(mapping, fields), reading)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewRow {
    pub note_id: i64,
    pub field: String,
    pub outcome: FieldOutcome,
    pub keys: Vec<WordKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingPreview {
    pub notes_total: usize,
    pub sampled: usize,
    pub words: usize,
    pub phrases: usize,
    pub empty: usize,
    /// Up to 8 rows of each outcome.
    pub examples: Vec<PreviewRow>,
}

/// Applies a mapping to a sample of the note type's notes.
pub fn preview_mapping(
    anki: &dyn AnkiSource,
    tok: &dyn Tokenizer,
    mapping: &NoteTypeMapping,
    scope: &Scope,
    sample: usize,
) -> Result<MappingPreview> {
    let ids = anki.find_notes(&scope.notes_query(mapping.model_id))?;
    let sample_ids: Vec<i64> = if ids.len() <= sample {
        ids.clone()
    } else {
        let step = ids.len() as f64 / sample as f64;
        (0..sample).map(|i| ids[(i as f64 * step) as usize]).collect()
    };
    let notes = anki.notes_info(&sample_ids)?;
    let mut p = MappingPreview { notes_total: ids.len(), sampled: notes.len(), words: 0, phrases: 0, empty: 0, examples: vec![] };
    let mut shown: HashMap<FieldOutcome, usize> = HashMap::new();
    for n in &notes {
        let fk = keys_for_note(tok, mapping, &n.fields)?;
        let field = n.fields.get(&mapping.target_field).cloned().unwrap_or_default();
        match fk.outcome {
            FieldOutcome::Word => p.words += 1,
            FieldOutcome::Phrase => p.phrases += 1,
            FieldOutcome::Empty => p.empty += 1,
        }
        let count = shown.entry(fk.outcome).or_default();
        if *count < 8 {
            *count += 1;
            p.examples.push(PreviewRow { note_id: n.note_id, field: field.to_string(), outcome: fk.outcome, keys: fk.keys });
        }
    }
    Ok(p)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub mapping: NoteTypeMapping,
    /// Why these fields were picked, in plain words for the setup screen.
    pub reason: String,
}

/// Looks at a sample of the note type's notes and proposes a mapping from what the fields
/// contain, not from their names: a field where most notes have a bold/italic part is a
/// sentence with a marked word; otherwise the field that most often holds a single word with
/// kanji. The reading field is the one whose first line is kana in most notes.
pub fn suggest_mapping(
    anki: &dyn AnkiSource,
    tok: &dyn Tokenizer,
    model_id: i64,
    model_name: &str,
    fields: &[String],
    scope: &Scope,
) -> Result<Option<Suggestion>> {
    let ids = anki.find_notes(&scope.notes_query(model_id))?;
    let step = (ids.len() / 60).max(1);
    let sample: Vec<i64> = ids.iter().step_by(step).take(60).copied().collect();
    let notes = anki.notes_info(&sample)?;
    if notes.is_empty() {
        return Ok(None);
    }
    let n = notes.len() as f64;
    let ratio = |f: &dyn Fn(&str) -> bool, field: &str| -> f64 {
        notes.iter().filter(|note| f(note.fields.get(field).map(String::as_str).unwrap_or(""))).count() as f64 / n
    };
    let first = |v: &str| clean_field(v).into_iter().next().map(|c| c.text).unwrap_or_default();

    let marked = fields
        .iter()
        .map(|f| (f, ratio(&|v| !marked_text(v).is_empty(), f)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, r)| *r >= 0.5);
    let (target, mode, reason) = if let Some((f, r)) = marked {
        let reason = format!("In {:.0}% of notes, “{f}” has a bold, italic or underlined part, so that part is taken as the word.", r * 100.0);
        (f.clone(), TargetMode::Marked, reason)
    } else {
        let mut best: Option<(&String, f64)> = None;
        for f in fields {
            let mut words = 0.0;
            for note in &notes {
                let v = note.fields.get(f).map(String::as_str).unwrap_or("");
                if has_kanji(&first(v)) && keys_for_field(tok, v, None)?.outcome == FieldOutcome::Word {
                    words += 1.0;
                }
            }
            if best.is_none_or(|(_, b)| words / n > b) {
                best = Some((f, words / n));
            }
        }
        match best.filter(|(_, r)| *r >= 0.3) {
            Some((f, r)) => (
                f.clone(),
                TargetMode::Field,
                format!("“{f}” holds a single word with kanji in {:.0}% of notes.", r * 100.0),
            ),
            None => return Ok(None),
        }
    };
    let reading = fields
        .iter()
        .filter(|f| **f != target)
        .map(|f| (f, ratio(&|v| is_all_kana(&first(v)), f)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, r)| *r >= 0.6)
        .map(|(f, _)| f.clone());
    let reason = match &reading {
        Some(r) => format!("{reason} “{r}” starts with kana, so it's used as the reading."),
        None => reason,
    };
    Ok(Some(Suggestion {
        mapping: NoteTypeMapping {
            model_id,
            model_name: model_name.to_string(),
            target_field: target,
            reading_field: reading,
            target_mode: mode,
        },
        reason,
    }))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncReport {
    pub notes: usize,
    pub cards: usize,
    pub words: usize,
    pub phrases: usize,
    pub empty_fields: usize,
    pub fsrs: bool,
    /// Cards suspended in Anki (they don't count unless the setting says so).
    pub suspended: usize,
    /// Review cards overdue by 30 days or more.
    pub overdue: usize,
    pub note_types: Vec<NoteTypeReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NoteTypeReport {
    pub model_name: String,
    pub notes: usize,
    pub words: usize,
    pub phrases: usize,
    pub empty_fields: usize,
}

const STABILITY_STEPS: [f64; 6] = [1.0, 7.0, 21.0, 60.0, 180.0, 365.0];
const RETRIEVABILITY_STEPS: [f64; 4] = [0.5, 0.7, 0.8, 0.9];
/// Days overdue, for estimating recall when FSRS is off.
const OVERDUE_STEPS: [i64; 6] = [7, 30, 90, 180, 365, 730];

/// Recall estimate for an SM-2 card: its interval taken as stability, on FSRS-4's forgetting
/// curve R = 1 / (1 + t / 9S), where t is the time since the last review (interval + overdue).
pub fn sm2_retrievability(interval: i64, overdue: i64) -> f64 {
    let s = interval.max(1) as f64;
    1.0 / (1.0 + (s + overdue as f64) / (9.0 * s))
}

/// Highest threshold whose search contains the card, per card.
fn buckets(anki: &dyn AnkiSource, base: &str, prop: &str, steps: &[f64]) -> Result<HashMap<i64, f64>> {
    let mut out = HashMap::new();
    for &t in steps {
        for id in anki.find_cards(&format!("({base}) prop:{prop}>={t}"))? {
            out.insert(id, t);
        }
    }
    Ok(out)
}

/// Reads every mapped note type from Anki and rebuilds the knowledge table.
/// `extra_query` narrows the cards (for example `deck:Japanese*`).
pub fn sync_anki(
    anki: &dyn AnkiSource,
    tok: &dyn Tokenizer,
    store: &mut Store,
    settings: &KnowledgeSettings,
    extra_query: &str,
) -> Result<SyncReport> {
    let mappings = store.mappings()?;
    let mut report = SyncReport::default();
    let mut cards: Vec<(i64, CardFacts)> = Vec::new();
    let mut card_keys: Vec<(i64, WordKey)> = Vec::new();
    let mut displays: HashMap<WordKey, String> = HashMap::new();
    if mappings.is_empty() {
        store.replace_anki_snapshot(&cards, &card_keys, &displays, settings)?;
        return Ok(report);
    }

    let mids = mappings.iter().map(|m| format!("mid:{}", m.model_id)).collect::<Vec<_>>().join(" OR ");
    let base = if extra_query.trim().is_empty() { format!("({mids})") } else { format!("({mids}) {}", extra_query.trim()) };
    let set = |q: &str| -> Result<HashSet<i64>> { Ok(anki.find_cards(&format!("{base} {q}"))?.into_iter().collect()) };
    let new = set("is:new")?;
    let learn = set("is:learn")?;
    let suspended = set("is:suspended")?;
    let buried = set("is:buried")?;
    report.fsrs = !anki.find_cards(&format!("{base} prop:s>=0"))?.is_empty();
    let (stab, retr) = if report.fsrs {
        (buckets(anki, &base, "s", &STABILITY_STEPS)?, buckets(anki, &base, "r", &RETRIEVABILITY_STEPS)?)
    } else {
        (HashMap::new(), HashMap::new())
    };
    // Without FSRS, how far overdue each review card is (prop:due<=-N: overdue by N days or more).
    let mut overdue: HashMap<i64, i64> = HashMap::new();
    if !report.fsrs {
        for t in OVERDUE_STEPS {
            for id in anki.find_cards(&format!("{base} is:review prop:due<=-{t}"))? {
                overdue.insert(id, t);
            }
        }
    }
    report.suspended = suspended.len();
    let in_scope: HashSet<i64> = anki.find_cards(&base)?.into_iter().collect();

    for m in &mappings {
        let mut per_type = NoteTypeReport { model_name: m.model_name.clone(), ..Default::default() };
        let note_ids = anki.find_notes(&format!("mid:{}", m.model_id))?;
        let notes: Vec<NoteInfo> = anki.notes_info(&note_ids)?;
        let note_cards: Vec<i64> =
            notes.iter().flat_map(|n| n.cards.iter().copied()).filter(|c| in_scope.contains(c)).collect();
        let intervals: HashMap<i64, i64> = note_cards.iter().copied().zip(anki.intervals(&note_cards)?).collect();
        for n in &notes {
            let fk = keys_for_note(tok, m, &n.fields)?;
            // The report only counts notes the user studies.
            let studied = n.cards.iter().any(|c| in_scope.contains(c) && (settings.count_suspended || !suspended.contains(c)));
            if studied {
                match fk.outcome {
                    FieldOutcome::Word => per_type.words += 1,
                    FieldOutcome::Phrase => per_type.phrases += 1,
                    FieldOutcome::Empty => per_type.empty_fields += 1,
                }
                per_type.notes += 1;
            }
            for &card_id in n.cards.iter().filter(|c| in_scope.contains(c)) {
                let card_type = if new.contains(&card_id) {
                    0
                } else if learn.contains(&card_id) {
                    1
                } else {
                    2
                };
                let queue = if suspended.contains(&card_id) {
                    -1
                } else if buried.contains(&card_id) {
                    -2
                } else {
                    card_type
                };
                let interval = intervals.get(&card_id).copied().unwrap_or(0).max(0);
                let late = overdue.get(&card_id).copied().unwrap_or(0);
                if late >= 30 {
                    report.overdue += 1;
                }
                let (stability, retrievability) = if card_type == 0 {
                    (None, None)
                } else if report.fsrs {
                    (Some(stab.get(&card_id).copied().unwrap_or(0.0)), Some(retr.get(&card_id).copied().unwrap_or(0.0)))
                } else if card_type == 2 {
                    (None, Some(sm2_retrievability(interval, late)))
                } else {
                    (None, None)
                };
                cards.push((
                    m.model_id,
                    CardFacts {
                        card_id,
                        note_id: n.note_id,
                        card_type,
                        queue,
                        interval,
                        lapses: 0,
                        stability,
                        retrievability,
                    },
                ));
                for (k, d) in fk.keys.iter().zip(&fk.displays) {
                    card_keys.push((card_id, k.clone()));
                    displays.entry(k.clone()).or_insert_with(|| d.clone());
                }
            }
        }
        report.notes += per_type.notes;
        report.words += per_type.words;
        report.phrases += per_type.phrases;
        report.empty_fields += per_type.empty_fields;
        report.note_types.push(per_type);
    }
    report.cards = cards.len();
    store.replace_anki_snapshot(&cards, &card_keys, &displays, settings)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anki::AnkiConnect;
    use crate::anki::fake::{FakeCard, FakeNote, serve};
    use crate::knowledge::WordState;
    use crate::tokenize::test_support::sudachi;

    fn key(lemma: &str, reading: &str) -> WordKey {
        WordKey { lang: "ja".into(), lemma: lemma.into(), reading: reading.into() }
    }

    #[test]
    fn field_keys() {
        let Some(t) = sudachi() else { return };
        let k = keys_for_field(t, "<b>食べる</b>", None).unwrap();
        assert_eq!(
            k,
            FieldKeys { outcome: FieldOutcome::Word, keys: vec![key("食べる", "たべる")], displays: vec!["食べる".into()] }
        );
        // The card's spelling is kept for display, the normalized one for matching.
        let k = keys_for_field(t, "降りる", None).unwrap();
        assert_eq!((k.keys[0].lemma.as_str(), k.displays[0].as_str()), ("下りる", "降りる"));
        // Variant spellings meet at the normalized lemma.
        assert_eq!(keys_for_field(t, "わかる", None).unwrap().keys, vec![key("分かる", "わかる")]);
        // A reading field picks the homograph reading.
        assert_eq!(keys_for_field(t, "人気", Some("ひとけ")).unwrap().keys, vec![key("人気", "ひとけ")]);
        assert_eq!(keys_for_field(t, "人気[ひとけ]", None).unwrap().keys, vec![key("人気", "ひとけ")]);
        // Alternatives in one field.
        assert_eq!(keys_for_field(t, "分かる、解る", None).unwrap().keys, vec![key("分かる", "わかる")]);
        assert_eq!(keys_for_field(t, "気をつける", None).unwrap().outcome, FieldOutcome::Phrase);
        assert_eq!(keys_for_field(t, "[sound:a.mp3]", None).unwrap().outcome, FieldOutcome::Empty);
    }

    #[test]
    fn sm2_recall_estimate() {
        // On time: R = 0.9, as for FSRS at t = S.
        assert!((sm2_retrievability(100, 0) - 0.9).abs() < 1e-9);
        // A 100-day card 180 days late is below 0.8; a 300-day card 180 days late isn't.
        assert!(sm2_retrievability(100, 180) < 0.8);
        assert!(sm2_retrievability(300, 180) > 0.8);
    }

    #[test]
    fn overdue_cards_fade_without_fsrs() {
        let Some(t) = sudachi() else { return };
        let card = |id, interval, overdue| FakeCard { id, interval, flags: &["review"], stability: None, retrievability: None, overdue };
        let url = serve(vec![
            FakeNote { id: 1, model: ("Vocab", 10), fields: vec![("Word", "猫")], cards: vec![card(11, 100, 0)] },
            FakeNote { id: 2, model: ("Vocab", 10), fields: vec![("Word", "犬")], cards: vec![card(21, 100, 200)] },
            FakeNote { id: 3, model: ("Vocab", 10), fields: vec![("Word", "鳥")], cards: vec![card(31, 400, 200)] },
        ]);
        let anki = AnkiConnect::new(&url);
        let mut store = Store::open_in_memory().unwrap();
        store
            .save_mapping(&NoteTypeMapping {
                model_id: 10,
                model_name: "Vocab".into(),
                target_field: "Word".into(),
                reading_field: None,
                target_mode: TargetMode::Field,
            })
            .unwrap();
        let report = sync_anki(&anki, t, &mut store, &KnowledgeSettings::default(), "").unwrap();
        assert!(!report.fsrs);
        assert_eq!(report.overdue, 2);
        assert_eq!(report.note_types.len(), 1);
        assert_eq!(report.note_types[0].words, 3);
        let state = |l: &str, r: &str| store.knowledge(&key(l, r)).unwrap().map(|k| k.state);
        assert_eq!(state("猫", "ねこ"), Some(WordState::Known));
        assert_eq!(state("犬", "いぬ"), Some(WordState::Fading));
        assert_eq!(state("鳥", "とり"), Some(WordState::Known));
    }

    #[test]
    fn sentence_cards_with_a_marked_word() {
        let Some(t) = sudachi() else { return };
        let fields = |sentence: &str, reading: &str| -> BTreeMap<String, String> {
            [("Sentence", sentence), ("Reading", reading), ("Meaning", "…")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };
        let m = NoteTypeMapping {
            model_id: 1,
            model_name: "Sentences".into(),
            target_field: "Sentence".into(),
            reading_field: Some("Reading".into()),
            target_mode: TargetMode::Marked,
        };
        // The owner's layout: word reading on the first line, the sentence's reading below.
        let k = keys_for_note(t, &m, &fields("<b>長篠</b>の戦いは日本の歴史で有名です。", "ながしの<br>ながしのがたたかいは…")).unwrap();
        assert_eq!(k.keys, vec![key("長篠", "ながしの")]);
        // A conjugated marked word gives the dictionary form.
        let k = keys_for_note(t, &m, &fields("彼は駅へ<i>向かった</i>。", "むかった")).unwrap();
        assert_eq!(k.keys, vec![key("向かう", "むかう")]);
        // A whole-sentence reading isn't taken as the word's reading.
        let k = keys_for_note(t, &m, &fields("<b>人気</b>のない道を通った。", "ひとけのないみちをとおった")).unwrap();
        assert_eq!(k.keys, vec![key("人気", "にんき")]);
        // Nothing marked: the whole sentence.
        assert_eq!(keys_for_note(t, &m, &fields("すみません！", "")).unwrap().outcome, FieldOutcome::Word);
        let k = keys_for_note(t, &m, &fields("どうして？", "")).unwrap();
        assert_eq!(k.outcome, FieldOutcome::Word);
        // A word plus a light verb is the word.
        let k = keys_for_note(t, &m, &fields("<b>むしゃくしゃして</b>いた。", "")).unwrap();
        assert_eq!(k.outcome, FieldOutcome::Word, "{k:?}");
        let k = keys_for_note(t, &m, &fields("怒りを<b>あらわにする</b>。", "")).unwrap();
        assert_eq!(k.outcome, FieldOutcome::Word, "{k:?}");
        // Nothing marked, but the reading field names the word.
        let k = keys_for_note(t, &m, &fields("日本にはほとんど石油がありません。", "せきゆ")).unwrap();
        assert_eq!(k.keys, vec![key("石油", "せきゆ")]);
        // A real phrase keeps its own key and adds its words.
        let k = keys_for_note(t, &m, &fields("ただの<b>脱水症状</b>だった。", "")).unwrap();
        assert_eq!(k.outcome, FieldOutcome::Phrase, "{k:?}");
        assert!(k.keys.iter().any(|x| x.lemma == "脱水") && k.keys.iter().any(|x| x.lemma == "症状"), "{k:?}");
    }

    #[test]
    fn sync_builds_knowledge() {
        let Some(t) = sudachi() else { return };
        let card = |id, interval, flags: &'static [&'static str], s: Option<f64>, r: Option<f64>| FakeCard {
            id,
            interval,
            flags,
            stability: s,
            retrievability: r,
            overdue: 0,
        };
        let url = serve(vec![
            FakeNote { id: 1, model: ("Vocab", 10), fields: vec![("Word", "猫"), ("Reading", "ねこ")],
                cards: vec![card(11, 120, &["review"], Some(200.0), Some(0.95))] },
            FakeNote { id: 2, model: ("Vocab", 10), fields: vec![("Word", "犬"), ("Reading", "いぬ")],
                cards: vec![card(21, 3, &["review"], Some(4.0), Some(0.9))] },
            FakeNote { id: 3, model: ("Vocab", 10), fields: vec![("Word", "鳥"), ("Reading", "とり")],
                cards: vec![card(31, 0, &["new"], None, None)] },
            FakeNote { id: 4, model: ("Vocab", 10), fields: vec![("Word", "馬"), ("Reading", "うま")],
                cards: vec![card(41, 90, &["review"], Some(100.0), Some(0.6))] },
            FakeNote { id: 5, model: ("Vocab", 10), fields: vec![("Word", "牛"), ("Reading", "うし")],
                cards: vec![card(51, 90, &["review", "suspended"], Some(100.0), Some(0.95))] },
            FakeNote { id: 6, model: ("Other", 20), fields: vec![("Front", "魚")],
                cards: vec![card(61, 90, &["review"], Some(100.0), Some(0.95))] },
            FakeNote { id: 7, model: ("Sentences", 30), fields: vec![("Sentence", "<b>長篠</b>の戦いは有名です。"), ("Reading", "ながしの<br>ながしのがたたかいは"), ("Meaning", "Nagashino")],
                cards: vec![card(71, 30, &["review"], Some(40.0), Some(0.9))] },
            FakeNote { id: 8, model: ("Sentences", 30), fields: vec![("Sentence", "駅へ<i>急いだ</i>。"), ("Reading", "いそいだ"), ("Meaning", "hurried")],
                cards: vec![card(81, 30, &["review"], Some(40.0), Some(0.9))] },
        ]);
        let anki = AnkiConnect::new(&url);
        let mut store = Store::open_in_memory().unwrap();
        store
            .save_mapping(&NoteTypeMapping {
                model_id: 10,
                model_name: "Vocab".into(),
                target_field: "Word".into(),
                reading_field: Some("Reading".into()),
                target_mode: TargetMode::Field,
            })
            .unwrap();
        let report = sync_anki(&anki, t, &mut store, &KnowledgeSettings::default(), "").unwrap();
        assert!(report.fsrs);
        // 5 cards stored; the suspended note isn't counted as studied.
        assert_eq!((report.notes, report.cards, report.words), (4, 5, 4));

        let state = |l: &str, r: &str| store.knowledge(&key(l, r)).unwrap().map(|k| k.state);
        assert_eq!(state("猫", "ねこ"), Some(WordState::Known));
        assert_eq!(state("犬", "いぬ"), Some(WordState::Learning));
        assert_eq!(state("鳥", "とり"), Some(WordState::New));
        assert_eq!(state("馬", "うま"), Some(WordState::Fading));
        // Only suspended cards: treated like no card at all.
        assert_eq!(state("牛", "うし"), None);
        // Not mapped.
        assert_eq!(state("魚", "さかな"), None);

        let preview = preview_mapping(&anki, t, &store.mappings().unwrap()[0], &Scope::default(), 50).unwrap();
        // The suspended 牛 note is left out.
        assert_eq!((preview.notes_total, preview.words), (4, 4));
        assert_eq!(report.note_types[0].notes, 4);

        let fields: Vec<String> = ["Sentence", "Reading", "Meaning"].map(String::from).to_vec();
        let sug = suggest_mapping(&anki, t, 30, "Sentences", &fields, &Scope::default()).unwrap().unwrap();
        assert_eq!(sug.mapping.target_field, "Sentence");
        assert_eq!(sug.mapping.target_mode, TargetMode::Marked);
        assert_eq!(sug.mapping.reading_field.as_deref(), Some("Reading"));
        let fields: Vec<String> = ["Word", "Reading"].map(String::from).to_vec();
        let sug = suggest_mapping(&anki, t, 10, "Vocab", &fields, &Scope::default()).unwrap().unwrap();
        assert_eq!((sug.mapping.target_field.as_str(), sug.mapping.target_mode), ("Word", TargetMode::Field));
        assert_eq!(sug.mapping.reading_field.as_deref(), Some("Reading"));
    }
}
