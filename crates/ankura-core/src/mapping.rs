//! Anki note fields → word keys, the mapping preview, and the Anki sync that builds Ankura's
//! knowledge table.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::anki::{AnkiSource, NoteInfo};
use crate::kana::{is_all_kana, to_hiragana};
use crate::knowledge::{CardFacts, KnowledgeSettings};
use crate::normalize::clean_field;
use crate::store::{NoteTypeMapping, Store};
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
        let content: Vec<_> =
            tokens.iter().filter(|t| matches!(t.kind, TokenKind::Word | TokenKind::Name | TokenKind::Number)).collect();
        match content.as_slice() {
            [] => {}
            [t] => {
                // A reading given for the field only applies when the field is the word's own
                // dictionary form (not e.g. a conjugated example).
                let reading = match &hint {
                    Some(h) if t.surface == cand.text && t.surface == t.dictionary_form => h.clone(),
                    _ => t.lemma_reading.clone(),
                };
                keys.push(WordKey { lang: tok.lang().into(), lemma: t.lemma.clone(), reading });
                if outcome == FieldOutcome::Empty {
                    outcome = FieldOutcome::Word;
                }
            }
            many => {
                let reading = hint.unwrap_or_else(|| many.iter().map(|t| t.lemma_reading.as_str()).collect());
                keys.push(WordKey { lang: tok.lang().into(), lemma: cand.text.clone(), reading });
                if outcome == FieldOutcome::Empty {
                    outcome = FieldOutcome::Phrase;
                }
            }
        }
    }
    let mut seen = HashSet::new();
    keys.retain(|k| seen.insert(k.clone()));
    Ok(FieldKeys { outcome, keys })
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
    sample: usize,
) -> Result<MappingPreview> {
    let ids = anki.find_notes(&format!("mid:{}", mapping.model_id))?;
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
        let field = n.fields.get(&mapping.target_field).map(String::as_str).unwrap_or("");
        let reading = mapping.reading_field.as_ref().and_then(|f| n.fields.get(f)).map(String::as_str);
        let fk = keys_for_field(tok, field, reading)?;
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncReport {
    pub notes: usize,
    pub cards: usize,
    pub words: usize,
    pub phrases: usize,
    pub empty_fields: usize,
    pub fsrs: bool,
}

const STABILITY_STEPS: [f64; 6] = [1.0, 7.0, 21.0, 60.0, 180.0, 365.0];
const RETRIEVABILITY_STEPS: [f64; 4] = [0.5, 0.7, 0.8, 0.9];

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
    if mappings.is_empty() {
        store.replace_anki_snapshot(&cards, &card_keys, settings)?;
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
    let in_scope: HashSet<i64> = anki.find_cards(&base)?.into_iter().collect();

    for m in &mappings {
        let note_ids = anki.find_notes(&format!("mid:{}", m.model_id))?;
        let notes: Vec<NoteInfo> = anki.notes_info(&note_ids)?;
        let note_cards: Vec<i64> =
            notes.iter().flat_map(|n| n.cards.iter().copied()).filter(|c| in_scope.contains(c)).collect();
        let intervals: HashMap<i64, i64> = note_cards.iter().copied().zip(anki.intervals(&note_cards)?).collect();
        for n in &notes {
            let field = n.fields.get(&m.target_field).map(String::as_str).unwrap_or("");
            let reading = m.reading_field.as_ref().and_then(|f| n.fields.get(f)).map(String::as_str);
            let fk = keys_for_field(tok, field, reading)?;
            match fk.outcome {
                FieldOutcome::Word => report.words += 1,
                FieldOutcome::Phrase => report.phrases += 1,
                FieldOutcome::Empty => report.empty_fields += 1,
            }
            report.notes += 1;
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
                cards.push((
                    m.model_id,
                    CardFacts {
                        card_id,
                        note_id: n.note_id,
                        card_type,
                        queue,
                        interval,
                        lapses: 0,
                        stability: stab.get(&card_id).copied().or(report.fsrs.then_some(0.0)).filter(|_| card_type != 0),
                        retrievability: retr.get(&card_id).copied().or(report.fsrs.then_some(0.0)).filter(|_| card_type != 0),
                    },
                ));
                for k in &fk.keys {
                    card_keys.push((card_id, k.clone()));
                }
            }
        }
    }
    report.cards = cards.len();
    store.replace_anki_snapshot(&cards, &card_keys, settings)?;
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
        assert_eq!(k, FieldKeys { outcome: FieldOutcome::Word, keys: vec![key("食べる", "たべる")] });
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
    fn sync_builds_knowledge() {
        let Some(t) = sudachi() else { return };
        let card = |id, interval, flags: &'static [&'static str], s: Option<f64>, r: Option<f64>| FakeCard {
            id,
            interval,
            flags,
            stability: s,
            retrievability: r,
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
        ]);
        let anki = AnkiConnect::new(&url);
        let mut store = Store::open_in_memory().unwrap();
        store
            .save_mapping(&NoteTypeMapping { model_id: 10, model_name: "Vocab".into(), target_field: "Word".into(), reading_field: Some("Reading".into()) })
            .unwrap();
        let report = sync_anki(&anki, t, &mut store, &KnowledgeSettings::default(), "").unwrap();
        assert!(report.fsrs);
        assert_eq!((report.notes, report.cards, report.words), (5, 5, 5));

        let state = |l: &str, r: &str| store.knowledge(&key(l, r)).unwrap().map(|k| k.state);
        assert_eq!(state("猫", "ねこ"), Some(WordState::Known));
        assert_eq!(state("犬", "いぬ"), Some(WordState::Learning));
        assert_eq!(state("鳥", "とり"), Some(WordState::New));
        assert_eq!(state("馬", "うま"), Some(WordState::Fading));
        assert_eq!(state("牛", "うし"), Some(WordState::Unknown));
        // Not mapped.
        assert_eq!(state("魚", "さかな"), None);

        let preview = preview_mapping(&anki, t, &store.mappings().unwrap()[0], 50).unwrap();
        assert_eq!((preview.notes_total, preview.words), (5, 5));
    }
}
