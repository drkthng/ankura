//! Tokenizer for Italian and Spanish (ADR 0001, ADR 0023): words split at spaces, punctuation
//! and elisions (l'amico), each mapped to its dictionary form with the form table of the
//! language's Wiktionary dictionary (`word_forms`, filled by `dict::import_yomitan_zip`), plus
//! rules for verbs with attached pronouns (dammelo, dímelo).
//!
//! The tokenizer reads the tables through its own database connection, so it needs no memory for
//! the million forms and is ready as soon as the dictionary is imported.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, params};

use crate::tokenize::{Token, TokenKind, Tokenizer};
use crate::{Error, Result};

/// Dictionary tags of words that are grammar, not vocabulary: not counted in coverage.
const FUNCTION_TAGS: &[&str] = &["prep", "conj", "det", "article", "art", "pron", "contr", "contraction", "particle"];

/// Verbs so common that a form of theirs is that verb, whatever else it could be (era, sale, fue).
const COMMON_VERBS_IT: &[&str] = &[
    "essere", "avere", "fare", "andare", "stare", "dare", "dire", "potere", "volere", "dovere", "sapere", "venire", "vedere",
];
const COMMON_VERBS_ES: &[&str] = &[
    "ser", "estar", "haber", "tener", "hacer", "ir", "poder", "decir", "dar", "ver", "saber", "querer", "venir",
];

/// Italian words written with a final apostrophe.
const IT_TRUNCATED: &[&str] = &["po", "da", "di", "fa", "sta", "va", "be", "mo", "to", "ve"];
/// Italian short imperatives that double the pronoun's consonant (dammi, dimmi, fallo, vacci).
const IT_SHORT_IMPERATIVES: &[(&str, &str)] = &[("da", "dare"), ("di", "dire"), ("fa", "fare"), ("sta", "stare"), ("va", "andare")];

const IT_CLITICS: &[&str] = &[
    "glielo", "gliela", "glieli", "gliele", "gliene", "melo", "mela", "meli", "mele", "mene", "telo", "tela", "teli", "tele", "tene",
    "celo", "cela", "celi", "cele", "cene", "velo", "vela", "veli", "vele", "vene", "selo", "sela", "seli", "sele", "sene", "gli",
    "mi", "ti", "ci", "vi", "si", "lo", "la", "li", "le", "ne",
];
const ES_CLITICS: &[&str] = &[
    "melos", "melas", "telos", "telas", "selos", "selas", "noslos", "noslas", "oslos", "oslas", "melo", "mela", "mele", "telo",
    "tela", "tele", "selo", "sela", "sele", "noslo", "nosla", "nosle", "oslo", "osla", "los", "las", "les", "nos", "lo", "la",
    "le", "me", "te", "se", "os",
];

pub struct RomanceTokenizer {
    lang: &'static str,
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Word,
    Number,
    Space,
    Symbol,
}

fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

/// Splits a text into words, numbers, runs of spaces and single symbols. An elided word keeps
/// its apostrophe and ends there (l'amico → l' + amico); so do po', da', fa'.
fn segment(text: &str) -> Vec<(String, Class)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(String, Class)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() {
            let mut word = String::new();
            while i < chars.len() {
                let c = chars[i];
                let next_is_letter = chars.get(i + 1).is_some_and(|n| n.is_alphabetic());
                if c.is_alphabetic() {
                    word.push(c);
                    i += 1;
                } else if is_apostrophe(c) && (next_is_letter || IT_TRUNCATED.contains(&word.to_lowercase().as_str())) {
                    word.push('\'');
                    i += 1;
                    break;
                } else if c == '-' && next_is_letter {
                    word.push(c);
                    i += 1;
                } else {
                    break;
                }
            }
            out.push((word, Class::Word));
        } else if c.is_numeric() {
            let mut n = String::new();
            while i < chars.len() && (chars[i].is_numeric() || (matches!(chars[i], '.' | ',') && chars.get(i + 1).is_some_and(|d| d.is_numeric()))) {
                n.push(chars[i]);
                i += 1;
            }
            out.push((n, Class::Number));
        } else if c.is_whitespace() {
            let mut s = String::new();
            while i < chars.len() && chars[i].is_whitespace() {
                s.push(chars[i]);
                i += 1;
            }
            out.push((s, Class::Space));
        } else {
            out.push((c.to_string(), Class::Symbol));
            i += 1;
        }
    }
    out
}

/// Without the written accent (dí → di, comprándo → comprando).
fn unaccent(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' => 'u',
            c => c,
        })
        .collect()
}

/// A form row's target as a lemma: "decir combined with indirect object me…" → "decir".
pub(crate) fn clean_lemma(target: &str) -> String {
    let t = target.split(" combined with").next().unwrap_or(target).trim();
    t.to_string()
}

struct Resolved {
    lemma: String,
    tags: Vec<String>,
    /// Neither the dictionary nor the rules know the word.
    unknown: bool,
}

impl RomanceTokenizer {
    /// Opens the language's database (the one its dictionary was imported into).
    pub fn open(db: &Path, lang: &str) -> Result<Self> {
        let lang = match lang {
            "it" => "it",
            "es" => "es",
            other => return Err(Error::Tokenizer(format!("no tokenizer for the language {other}"))),
        };
        let conn = Connection::open(db)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        Ok(RomanceTokenizer { lang, conn: Mutex::new(conn) })
    }

    /// Whether a dictionary with a form table is there.
    pub fn ready(&self) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT EXISTS (SELECT 1 FROM word_forms)", [], |r| r.get::<_, bool>(0)).unwrap_or(false)
    }

    /// The dictionary's tags for a headword, or `None` if it isn't one.
    fn headword(conn: &Connection, word: &str) -> Result<Option<Vec<String>>> {
        let mut st = conn.prepare_cached("SELECT tags FROM dict_terms WHERE expression = ?1 LIMIT 8")?;
        let rows = st.query_map([word], |r| r.get::<_, String>(0))?;
        let mut tags: Vec<String> = Vec::new();
        let mut found = false;
        for row in rows {
            found = true;
            for t in row?.split_whitespace() {
                if !tags.iter().any(|x| x == t) {
                    tags.push(t.to_string());
                }
            }
        }
        Ok(found.then_some(tags))
    }

    /// The words this form is a form of.
    fn lemmas(conn: &Connection, form: &str) -> Result<Vec<String>> {
        let mut st = conn.prepare_cached("SELECT DISTINCT lemma FROM word_forms WHERE form = ?1 LIMIT 8")?;
        let rows = st.query_map([form], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn known(&self, conn: &Connection, lemma: &str) -> bool {
        conn.prepare_cached("SELECT EXISTS (SELECT 1 FROM knowledge WHERE lang = ?1 AND lemma = ?2)")
            .and_then(|mut st| st.query_row(params![self.lang, lemma], |r| r.get::<_, bool>(0)))
            .unwrap_or(false)
    }

    /// A headword as it is, else the first word the form belongs to.
    fn simple(conn: &Connection, form: &str) -> Result<Option<String>> {
        if Self::headword(conn, form)?.is_some() {
            return Ok(Some(form.to_string()));
        }
        Ok(Self::lemmas(conn, form)?.into_iter().next())
    }

    /// The verb of a form with attached pronouns (parlarne, dammelo, dímelo, vámonos).
    fn clitic_verb(&self, conn: &Connection, form: &str) -> Result<Option<String>> {
        let clitics = if self.lang == "it" { IT_CLITICS } else { ES_CLITICS };
        for cl in clitics {
            let Some(stem) = form.strip_suffix(cl) else { continue };
            if stem.chars().count() < 2 {
                continue;
            }
            let mut tries: Vec<String> = Vec::new();
            if self.lang == "it" {
                // dammelo, dimmi, fallo: a short imperative doubles the pronoun's first consonant.
                if let (Some(last), Some(first)) = (stem.chars().last(), cl.chars().next()) {
                    if last == first {
                        let short = &stem[..stem.len() - last.len_utf8()];
                        if let Some((_, verb)) = IT_SHORT_IMPERATIVES.iter().find(|(s, _)| *s == short) {
                            return Ok(Some(verb.to_string()));
                        }
                    }
                }
                tries.extend([format!("{stem}e"), format!("{stem}re"), stem.to_string()]);
            } else {
                let plain = unaccent(stem);
                tries.extend([plain.clone(), format!("{plain}s"), format!("{plain}d")]);
            }
            for t in tries {
                if let Some(lemma) = Self::simple(conn, &t)? {
                    return Ok(Some(lemma));
                }
            }
        }
        Ok(None)
    }

    fn resolve(&self, conn: &Connection, form: &str) -> Result<Resolved> {
        let own = Self::headword(conn, form)?;
        // Grammar words are themselves (del, che, para), whatever else they could be a form of.
        if let Some(tags) = &own {
            if tags.iter().any(|t| FUNCTION_TAGS.contains(&t.as_str())) {
                return Ok(Resolved { lemma: form.to_string(), tags: tags.clone(), unknown: false });
            }
        }
        let lemmas: Vec<String> = Self::lemmas(conn, form)?.iter().map(|l| clean_lemma(l)).filter(|l| !l.is_empty()).collect();
        let common = if self.lang == "it" { COMMON_VERBS_IT } else { COMMON_VERBS_ES };
        let mut candidates: Vec<&str> = Vec::new();
        if own.is_some() {
            candidates.push(form);
        }
        candidates.extend(lemmas.iter().map(String::as_str));
        // In this order: a word the learner has in Anki, a very common verb, the word itself if
        // the dictionary has it, the first word it is a form of.
        let in_anki = candidates.iter().copied().find(|c| self.known(conn, c));
        let common_verb = lemmas.iter().map(String::as_str).find(|l| common.contains(l));
        let chosen: Option<String> = in_anki.or(common_verb).or(candidates.first().copied()).map(str::to_string);
        let (lemma, unknown) = match chosen {
            Some(l) => (l, false),
            None => match self.clitic_verb(conn, form)? {
                Some(l) => (l, false),
                None => (form.to_string(), true),
            },
        };
        let tags = if lemma == form { own.unwrap_or_default() } else { Self::headword(conn, &lemma)?.unwrap_or_default() };
        Ok(Resolved { lemma, tags, unknown })
    }
}

impl Tokenizer for RomanceTokenizer {
    fn lang(&self) -> &'static str {
        self.lang
    }

    fn tokenize(&self, text: &str) -> Result<Vec<Token>> {
        let conn = self.conn.lock().unwrap();
        let mut out: Vec<Token> = Vec::new();
        // The first word of a sentence is capitalised whatever it is.
        let mut sentence_start = true;
        for (surface, class) in segment(text) {
            let (lemma, pos, kind) = match class {
                Class::Space => (surface.clone(), Vec::new(), TokenKind::Space),
                Class::Symbol => {
                    if matches!(surface.as_str(), "." | "!" | "?" | "…" | "¡" | "¿" | "«" | "\"" | "“" | "—" | "–") {
                        sentence_start = true;
                    }
                    (surface.clone(), Vec::new(), TokenKind::Symbol)
                }
                Class::Number => {
                    sentence_start = false;
                    (surface.clone(), Vec::new(), TokenKind::Number)
                }
                Class::Word => {
                    let lower = surface.to_lowercase();
                    let capitalised = surface != lower;
                    let mut r = self.resolve(&conn, &lower)?;
                    let mut kind = if r.tags.iter().any(|t| FUNCTION_TAGS.contains(&t.as_str())) { TokenKind::Grammar } else { TokenKind::Word };
                    if r.unknown && capitalised {
                        // Roma, María: a headword only with its capital, or not in the dictionary.
                        if let Some(tags) = Self::headword(&conn, &surface)? {
                            r = Resolved { lemma: surface.clone(), tags, unknown: false };
                            kind = TokenKind::Name;
                        } else if !sentence_start {
                            r.lemma = surface.clone();
                            kind = TokenKind::Name;
                        }
                    }
                    sentence_start = false;
                    (r.lemma, r.tags, kind)
                }
            };
            out.push(Token {
                surface,
                dictionary_form: lemma.clone(),
                lemma,
                lemma_reading: String::new(),
                reading: String::new(),
                pos,
                kind,
                parts: Vec::new(),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    /// A database with a few dictionary entries (headword, tags) and forms (form, lemma).
    fn tokenizer(lang: &str, words: &[(&str, &str)], forms: &[(&str, &str)]) -> (tempfile::TempDir, Store, RomanceTokenizer) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let store = Store::open(&path).unwrap();
        store.conn.execute("INSERT INTO dictionaries (id, title, revision, imported_at) VALUES (1, 'test', '', 0)", []).unwrap();
        for (w, tags) in words {
            store
                .conn
                .execute(
                    "INSERT INTO dict_terms (dict_id, expression, reading, tags, score, glossary, sequence, term_tags)
                     VALUES (1, ?1, ?1, ?2, 0, '[\"x\"]', 0, '')",
                    params![w, tags],
                )
                .unwrap();
        }
        for (f, l) in forms {
            store.conn.execute("INSERT INTO word_forms (dict_id, form, lemma, info) VALUES (1, ?1, ?2, '')", params![f, l]).unwrap();
        }
        let tok = RomanceTokenizer::open(&path, lang).unwrap();
        (dir, store, tok)
    }

    fn lemmas(tok: &RomanceTokenizer, text: &str) -> Vec<(String, String, TokenKind)> {
        tok.tokenize(text)
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != TokenKind::Space)
            .map(|t| (t.surface, t.lemma, t.kind))
            .collect()
    }

    fn row(s: &str, l: &str, k: TokenKind) -> (String, String, TokenKind) {
        (s.to_string(), l.to_string(), k)
    }

    #[test]
    fn splits_words_elisions_numbers_and_symbols() {
        let parts: Vec<String> = segment("L'amico ha un po' di caffè: 3,5 euro!").into_iter().map(|(s, _)| s).collect();
        assert_eq!(parts, ["L'", "amico", " ", "ha", " ", "un", " ", "po'", " ", "di", " ", "caffè", ":", " ", "3,5", " ", "euro", "!"]);
        // A typographic apostrophe is the same; a quote after a word is not part of it.
        let parts: Vec<String> = segment("dell\u{2019}acqua 'ciao'").into_iter().map(|(s, _)| s).collect();
        assert_eq!(parts, ["dell'", "acqua", " ", "'", "ciao", "'"]);
        assert_eq!(unaccent("dímelo"), "dimelo");
        assert_eq!(clean_lemma("decir combined with indirect object me and direct object lo"), "decir");
    }

    #[test]
    fn italian_words_get_their_dictionary_form() {
        let (_dir, store, tok) = tokenizer(
            "it",
            &[("casa", "n fem"), ("parlare", "v"), ("essere", "v"), ("era", "n fem"), ("il", "article"), ("l'", "article"), ("amico", "n"), ("dare", "v"), ("Roma", "name"), ("sale", "n")],
            &[("case", "casa"), ("parlavo", "parlare"), ("era", "essere"), ("amici", "amico"), ("sale", "salire")],
        );
        assert!(tok.ready());
        assert_eq!(
            lemmas(&tok, "Parlavo con l'amico: le case di Roma."),
            [
                row("Parlavo", "parlare", TokenKind::Word),
                row("con", "con", TokenKind::Word),
                row("l'", "l'", TokenKind::Grammar),
                row("amico", "amico", TokenKind::Word),
                row(":", ":", TokenKind::Symbol),
                row("le", "le", TokenKind::Word),
                row("case", "casa", TokenKind::Word),
                row("di", "di", TokenKind::Word),
                row("Roma", "Roma", TokenKind::Name),
                row(".", ".", TokenKind::Symbol),
            ]
        );
        // A form of a very common verb is that verb (era), another word that is also a form
        // stays itself (sale), and attached pronouns come off the verb.
        let l = |text: &str| lemmas(&tok, text).into_iter().map(|t| t.1).collect::<Vec<_>>();
        assert_eq!(l("era sale parlarne dammelo"), ["essere", "sale", "parlare", "dare"]);
        // A name the dictionary doesn't have, in the middle of a sentence.
        assert_eq!(lemmas(&tok, "amico Gianni")[1], row("Gianni", "Gianni", TokenKind::Name));
        // What the learner has in Anki wins: with a card for salire, sale is salire.
        store.conn.execute("INSERT INTO knowledge (lang, lemma, reading, state, strength, cards) VALUES ('it', 'salire', '', 'known', 30, 1)", []).unwrap();
        assert_eq!(l("sale"), ["salire"]);
        // Readings stay empty: the word key is language and lemma.
        assert!(tok.tokenize("case").unwrap().iter().all(|t| t.reading.is_empty() && t.lemma_reading.is_empty()));
    }

    #[test]
    fn spanish_forms_and_attached_pronouns() {
        let (_dir, _store, tok) = tokenizer(
            "es",
            &[("hablar", "v"), ("casa", "n"), ("decir", "v"), ("ir", "v"), ("para", "prep"), ("sentar", "v")],
            &[("hablaba", "hablar"), ("casas", "casa"), ("casas", "casar"), ("di", "decir"), ("vamos", "ir"), ("sentad", "sentar"), ("dámelo", "dar combined with indirect object me and direct object lo")],
        );
        let l = |text: &str| lemmas(&tok, text).into_iter().map(|t| t.1).collect::<Vec<_>>();
        assert_eq!(l("¿Hablaba de las casas?"), ["¿", "hablar", "de", "las", "casa", "?"]);
        assert_eq!(l("dímelo vámonos sentaos dámelo hablarle"), ["decir", "ir", "sentar", "dar", "hablar"]);
        assert_eq!(lemmas(&tok, "para")[0].2, TokenKind::Grammar);
        assert!(RomanceTokenizer::open(std::path::Path::new("x.db"), "fr").is_err());
    }
}
