//! A text as the reader shows it: paragraphs of tokens with furigana, word state and coverage.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::furigana::{Segment, segments};
use crate::kana::has_kanji;
use crate::knowledge::WordState;
use crate::store::Store;
use crate::tokenize::{Token, TokenKind, Tokenizer};
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
    /// Where the state came from: "mark", "anki", "basic" (a very common word), "parts" (all parts
    /// of a compound known) or "none".
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

/// The form a phrase is stored and matched under: its words up to the last content word, with
/// that word in its dictionary form (気になります → 気になる), and the matching reading. `None`
/// unless at least two tokens lead up to the last content word.
pub(crate) fn phrase_form(toks: &[Token]) -> Option<(String, String)> {
    let last = toks.iter().rposition(|t| matches!(t.kind, TokenKind::Word | TokenKind::Name))?;
    let reading_of = |t: &Token| if t.reading.is_empty() { t.surface.clone() } else { t.reading.clone() };
    let mut form = String::new();
    let mut reading = String::new();
    for t in &toks[..last] {
        form.push_str(&t.surface);
        reading.push_str(&reading_of(t));
    }
    form.push_str(&toks[last].dictionary_form);
    reading.push_str(&toks[last].lemma_reading);
    (last >= 1).then_some((form, reading))
}

/// The key Ankura already has for a phrase (from Anki or a mark), whatever its reading.
fn known_phrase(store: &Store, lang: &str, form: &str) -> Result<Option<(WordKey, WordState, &'static str)>> {
    for key in store.keys_for_lemma(lang, form)? {
        let (state, source) = state_of(store, &key)?;
        if state != WordState::Unknown || source == "mark" {
            return Ok(Some((key, state, source)));
        }
    }
    Ok(None)
}

/// A phrase the user selected (気になります), as one token: its dictionary form (気になる), reading
/// and state. Prefers a form Ankura already knows, then one the dictionary knows (that, or the
/// selection as written). `None` for a selection without words.
pub fn phrase(store: &Store, tok: &dyn Tokenizer, selection: &str) -> Result<Option<ReaderToken>> {
    let selection = selection.trim();
    let toks: Vec<_> =
        tok.tokenize(selection)?.into_iter().filter(|t| !matches!(t.kind, TokenKind::Space | TokenKind::Symbol)).collect();
    let Some(last) = toks.iter().rposition(|t| matches!(t.kind, TokenKind::Word | TokenKind::Name)) else {
        return Ok(None);
    };
    let reading_of = |t: &Token| if t.reading.is_empty() { t.surface.clone() } else { t.reading.clone() };
    let (base, base_reading) = phrase_form(&toks).unwrap_or_else(|| (toks[last].dictionary_form.clone(), toks[last].lemma_reading.clone()));
    let written: String = toks.iter().map(|t| t.surface.as_str()).collect();
    let written_reading: String = toks.iter().map(reading_of).collect();
    let lang = tok.lang();

    let mut found = None;
    for form in [&base, &written] {
        if let Some(k) = known_phrase(store, lang, form)? {
            found = Some(k);
            break;
        }
    }
    let (key, state, source) = match found {
        Some(k) => k,
        None => {
            let mut chosen = (base.clone(), base_reading.clone());
            for (form, reading) in [(base, base_reading), (written, written_reading)] {
                if let Some(e) = crate::dict::lookup(store, &form, &reading)?.into_iter().find(|e| e.expression == form) {
                    chosen = (form, e.reading);
                    break;
                }
            }
            let key = WordKey { lang: lang.to_string(), lemma: chosen.0, reading: chosen.1 };
            let (state, source) = state_of(store, &key)?;
            (key, state, source)
        }
    };
    Ok(Some(ReaderToken {
        surface: selection.to_string(),
        dictionary_form: key.lemma.clone(),
        segments: vec![Segment { text: selection.to_string(), ruby: None }],
        kind: TokenKind::Word,
        key: Some(key),
        state,
        source: source.into(),
        has_kanji: has_kanji(selection),
    }))
}

/// Longest phrase Ankura knows starting at token `i` (2-6 tokens, no punctuation; may end in
/// auxiliaries or て/で after its last content word, as in 気になります or 気になって).
fn phrase_at(
    store: &Store,
    lang: &str,
    lemmas: &HashSet<String>,
    toks: &[Token],
    i: usize,
) -> Result<Option<(usize, WordKey, WordState, &'static str)>> {
    if !matches!(toks[i].kind, TokenKind::Word | TokenKind::Name | TokenKind::Grammar) {
        return Ok(None);
    }
    let trailing_ok = |t: &Token| {
        t.pos.first().is_some_and(|p| p == "助動詞") || (t.pos.get(1).is_some_and(|p| p == "接続助詞") && matches!(t.surface.as_str(), "て" | "で"))
    };
    for len in (2..=6).rev() {
        let end = i + len;
        if end > toks.len() {
            continue;
        }
        let span = &toks[i..end];
        if span.iter().any(|t| matches!(t.kind, TokenKind::Space | TokenKind::Symbol)) {
            continue;
        }
        let Some(last) = span.iter().rposition(|t| matches!(t.kind, TokenKind::Word | TokenKind::Name)) else { continue };
        if !span[last + 1..].iter().all(trailing_ok) {
            continue;
        }
        let Some((form, _)) = phrase_form(span) else { continue };
        let written: String = span.iter().map(|t| t.surface.as_str()).collect();
        for f in [&form, &written] {
            if lemmas.contains(f.as_str()) {
                if let Some((key, state, source)) = known_phrase(store, lang, f)? {
                    return Ok(Some((end, key, state, source)));
                }
            }
        }
    }
    Ok(None)
}

/// Very common words every learner knows but few have cards for, as Sudachi normalizes them
/// (する → 為る, こと → 事). They count as known unless the user marks them otherwise.
const BASIC_WORDS: &[&str] = &[
    "為る", "有る", "居る", "成る", "無い", "良い", "言う", "思う", "来る", "行く", "見る", "出来る",
    "事", "物", "時", "所", "人", "今", "何", "此の", "其の", "彼の", "此れ", "其れ", "彼れ", "此処", "其処",
    "私", "僕", "貴方", "彼", "彼女", "皆", "未だ", "もう", "又", "とても", "一寸", "如何", "然う", "斯う",
    "そう", "こう", "どう", "ちょっと", "まだ", "また",
];

fn state_of(store: &Store, key: &WordKey) -> Result<(WordState, &'static str)> {
    if let Some(s) = store.word_override(key)? {
        return Ok((s, "mark"));
    }
    match store.knowledge(key)? {
        Some(k) if k.state != WordState::Unknown => Ok((k.state, "anki")),
        _ if BASIC_WORDS.contains(&key.lemma.as_str()) => Ok((WordState::Known, "basic")),
        _ => Ok((WordState::Unknown, "none")),
    }
}

pub fn analyze(store: &Store, tok: &dyn Tokenizer, text: &str) -> Result<ReaderDoc> {
    analyze_text(store, tok, text, None)
}

/// `analyze`, with the reading fixes for this text (and the ones for everywhere).
pub fn analyze_text(store: &Store, tok: &dyn Tokenizer, text: &str, text_id: Option<&str>) -> Result<ReaderDoc> {
    let lang = tok.lang();
    let fixes = store.reading_fixes(lang, text_id)?;
    // Everything Ankura has a state for: a span is only looked up when its form is one of these.
    let phrase_lemmas: HashSet<String> = store.all_lemmas(lang)?;
    let mut paragraphs = Vec::new();
    let mut cov = Coverage::default();
    for line in text.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let mut out = Vec::new();
        let mut toks = tok.tokenize(line)?;
        // Readings the user corrected. For a word in its dictionary form the fix also says which
        // word it is (人気 にんき → ひとけ), so its key, meaning and state follow.
        for t in toks.iter_mut() {
            if let Some(r) = fixes.get(&t.surface) {
                if t.surface == t.dictionary_form {
                    t.lemma_reading = r.clone();
                }
                t.reading = r.clone();
            }
        }
        let mut i = 0;
        while i < toks.len() {
            // A phrase from Anki or the user's marks shows as one word (気になります).
            if !phrase_lemmas.is_empty() {
                if let Some((end, key, state, source)) = phrase_at(store, lang, &phrase_lemmas, &toks, i)? {
                    let span = &toks[i..end];
                    cov.words += 1;
                    match state {
                        s if s.is_known() => cov.known += 1,
                        WordState::Learning => cov.learning += 1,
                        WordState::New => cov.new += 1,
                        _ => cov.unknown += 1,
                    }
                    out.push(ReaderToken {
                        segments: span.iter().flat_map(|t| segments(&t.surface, &t.reading)).collect(),
                        has_kanji: span.iter().any(|t| has_kanji(&t.surface)),
                        dictionary_form: key.lemma.clone(),
                        surface: span.iter().map(|t| t.surface.as_str()).collect(),
                        kind: TokenKind::Word,
                        key: Some(key),
                        state,
                        source: source.to_string(),
                    });
                    i = end;
                    continue;
                }
            }
            let t = toks[i].clone();
            i += 1;
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

/// Words of the N5-N3 lists: a voice reads these right from their kanji.
static EASY: std::sync::LazyLock<HashSet<&'static str>> = std::sync::LazyLock::new(|| {
    crate::proficiency::VOCAB.iter().filter(|w| w.level >= 3).map(|w| w.expression.as_str()).collect()
});

/// The text as a voice should say it (ADR 0017). Kanji stay where a voice reads them right and
/// where they show it how the sentence splits (と山崎は言った: written in kana, は+いった is heard
/// as 入った). Replaced by the reading the reader shows, in katakana so it doesn't run into the
/// kana around it: names, words not on the N5-N3 lists (口籠もった → クチゴモッた), and words
/// whose reading the learner corrected.
pub fn spoken(store: &Store, tok: &dyn Tokenizer, text: &str, text_id: Option<&str>) -> Result<String> {
    let fixes = store.reading_fixes(tok.lang(), text_id)?;
    let doc = analyze_text(store, tok, text, text_id)?;
    Ok(doc
        .paragraphs
        .iter()
        .map(|p| {
            p.iter()
                .map(|t| {
                    let common = EASY.contains(t.dictionary_form.as_str()) || t.key.as_ref().is_some_and(|k| EASY.contains(k.lemma.as_str()));
                    let replace = t.has_kanji && (t.kind == TokenKind::Name || !common || fixes.contains_key(&t.surface));
                    if replace {
                        crate::kana::to_katakana(&t.segments.iter().map(|g| g.ruby.as_deref().unwrap_or(&g.text)).collect::<String>())
                    } else {
                        t.surface.clone()
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{CardFacts, KnowledgeSettings};
    use crate::tokenize::test_support::sudachi;

    #[test]
    fn reading_fixes_apply_everywhere_or_in_one_text() {
        let Some(t) = sudachi() else { return };
        let s = Store::open_in_memory().unwrap();
        let reading = |doc: &ReaderDoc, w: &str| {
            let x = doc.paragraphs[0].iter().find(|x| x.surface == w).unwrap();
            (x.segments.iter().filter_map(|g| g.ruby.clone()).collect::<String>(), x.key.as_ref().unwrap().reading.clone())
        };
        let text = "人気のない道。";
        assert_eq!(reading(&analyze_text(&s, t, text, Some("a")).unwrap(), "人気"), ("にんき".into(), "にんき".into()));
        s.set_reading_fix("ja", "人気", Some("ヒトケ"), Some("a")).unwrap();
        assert_eq!(reading(&analyze_text(&s, t, text, Some("a")).unwrap(), "人気"), ("ひとけ".into(), "ひとけ".into()));
        assert_eq!(reading(&analyze_text(&s, t, text, Some("b")).unwrap(), "人気").0, "にんき", "only in text a");
        s.set_reading_fix("ja", "人気", Some("にんき"), None).unwrap();
        assert_eq!(reading(&analyze_text(&s, t, text, Some("a")).unwrap(), "人気").0, "ひとけ", "the text's own fix wins");
        s.set_reading_fix("ja", "人気", None, Some("a")).unwrap();
        assert_eq!(reading(&analyze_text(&s, t, text, Some("a")).unwrap(), "人気").0, "にんき");
    }

    #[test]
    fn known_phrases_show_as_one_word() {
        let Some(t) = sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        // A phrase card synced from Anki, in the form sync stores it (any reading).
        let card = CardFacts { card_id: 1, note_id: 1, card_type: 2, queue: 2, interval: 60, lapses: 0, stability: None, retrievability: None };
        let ki_key = WordKey { lang: "ja".into(), lemma: "気になる".into(), reading: "きになります".into() };
        s.replace_anki_snapshot(&[(1, card)], &[(1, ki_key.clone())], &std::collections::HashMap::new(), &KnowledgeSettings::default())
            .unwrap();
        let doc = analyze(&s, t, "歴史がすごく気になります。気にしない。").unwrap();
        let toks = &doc.paragraphs[0];
        let ki = toks.iter().find(|x| x.surface == "気になります").expect("merged");
        assert_eq!(ki.key.as_ref(), Some(&ki_key));
        assert_eq!((ki.state, ki.dictionary_form.as_str()), (WordState::Known, "気になる"));
        assert!(ki.segments.iter().any(|g| g.ruby.as_deref() == Some("き")), "furigana kept: {:?}", ki.segments);
        // 気にする isn't the phrase: its words stay separate.
        assert!(toks.iter().any(|x| x.surface == "気"), "{:?}", toks.iter().map(|x| &x.surface).collect::<Vec<_>>());

        // A phrase the user marked (from a selection) matches as well, and the selection finds it.
        let mark = WordKey { lang: "ja".into(), lemma: "途方に暮れる".into(), reading: "とほうにくれる".into() };
        s.set_word_override(&mark, Some(WordState::MarkedKnown)).unwrap();
        let doc = analyze(&s, t, "彼は途方に暮れていた。").unwrap();
        let x = doc.paragraphs[0].iter().find(|x| x.dictionary_form == "途方に暮れる").expect("merged");
        assert_eq!((x.surface.as_str(), x.source.as_str()), ("途方に暮れて", "mark"));
        let sel = phrase(&s, t, "気になります").unwrap().unwrap();
        assert_eq!(sel.key, Some(ki_key), "the selection uses the synced key");
        assert_eq!(sel.state, WordState::Known);
    }

    #[test]
    fn selected_phrases() {
        let Some(t) = sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        // Without a dictionary: the last content word goes to its dictionary form.
        let p = phrase(&s, t, " 気になります。").unwrap().unwrap();
        assert_eq!((p.dictionary_form.as_str(), p.surface.as_str()), ("気になる", "気になります。"));
        assert_eq!(p.key.as_ref().unwrap().reading, "きになる");
        assert_eq!(p.state, WordState::Unknown);
        assert!(phrase(&s, t, "、。").unwrap().is_none());

        // With one: the form the dictionary has wins (here the polite set phrase as written).
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("d.zip");
        crate::dict::write_zip(&zip, &[("term_bank_1.json", serde_json::json!([
            ["お願いします", "おねがいします", "exp", "", 100, ["please"], 1, ""]
        ]))]);
        crate::dict::import_yomitan_zip(&mut s, &zip).unwrap();
        let p = phrase(&s, t, "お願いします").unwrap().unwrap();
        assert_eq!(p.dictionary_form, "お願いします");
        assert_eq!(p.key.unwrap().reading, "おねがいします");
    }

    #[test]
    fn colours_and_coverage() {
        let Some(t) = sudachi() else { return };
        let mut s = Store::open_in_memory().unwrap();
        let key = |l: &str, r: &str| WordKey { lang: "ja".into(), lemma: l.into(), reading: r.into() };
        let card = |id| CardFacts { card_id: id, note_id: id, card_type: 2, queue: 2, interval: 60, lapses: 0, stability: None, retrievability: None };
        s.replace_anki_snapshot(
            &[(1, card(1)), (1, card(2)), (1, card(3))],
            &[(1, key("猫", "ねこ")), (2, key("海外", "かいがい")), (3, key("旅行", "りょこう"))],
            &std::collections::HashMap::new(),
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
        assert_eq!(行き.state, WordState::Known);
        assert_eq!(行き.segments[0].ruby.as_deref(), Some("い"));
        // 猫, 好き, 海外旅行 known; 行き is a basic word (行く).
        assert_eq!(行き.source, "basic");
        assert_eq!((doc.coverage.words, doc.coverage.known), (4, 4));
        let doc = analyze(&s, t, "宿題をした。").unwrap();
        let し = doc.paragraphs[0].iter().find(|x| x.surface == "し").unwrap();
        assert_eq!((し.state, し.source.as_str()), (WordState::Known, "basic"));
    }

    #[test]
    fn spoken_form_uses_the_shown_readings() {
        let Some(t) = sudachi() else { return };
        let s = Store::open_in_memory().unwrap();
        // Rare words and names in katakana; common words keep their kanji, so は stays a particle.
        assert_eq!(spoken(&s, t, "彼は口籠もった。", None).unwrap(), "彼はクチゴモッた。");
        assert_eq!(spoken(&s, t, "と山崎は言った。", None).unwrap(), "とヤマザキは言った。");
        // A corrected reading is what the voice says, too.
        s.set_reading_fix("ja", "人気", Some("ひとけ"), None).unwrap();
        assert_eq!(spoken(&s, t, "人気のない道", None).unwrap(), "ヒトケのない道");
    }
}
