//! Cleans Anki field values into plain candidate words (architecture-options §3).

use std::sync::LazyLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::kana::{is_all_kana, to_hiragana};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    /// A reading found in the field itself (furigana markup, ruby, or a kana gloss in brackets).
    pub reading_hint: Option<String>,
}

static RE_SOUND: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[sound:[^\]]*\]").unwrap());
static RE_CLOZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{c\d+::(.*?)(?:::[^}]*)?\}\}").unwrap());
static RE_RT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<rt[^>]*>(.*?)</rt>").unwrap());
static RE_RP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<rp[^>]*>.*?</rp>").unwrap());
static RE_BREAK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<br\s*/?>|</(?:div|p|li)>").unwrap());
static RE_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]*>").unwrap());
static RE_FURIGANA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" ?([^ \[\]]+)\[([^\]]*)\]").unwrap());
static RE_PAREN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[(（]([^)）]*)[)）]").unwrap());
static RE_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\u{3000}]+").unwrap());

fn decode_entities(s: &str) -> String {
    static RE_ENT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);").unwrap());
    RE_ENT
        .replace_all(s, |c: &regex::Captures| {
            let e = &c[1];
            let decoded = if let Some(hex) = e.strip_prefix("#x").or_else(|| e.strip_prefix("#X")) {
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
            } else if let Some(dec) = e.strip_prefix('#') {
                dec.parse().ok().and_then(char::from_u32)
            } else {
                match e {
                    "nbsp" => Some(' '),
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ => None,
                }
            };
            decoded.map(String::from).unwrap_or_else(|| c[0].to_string())
        })
        .into_owned()
}

/// Splits Anki's `漢字[かんじ]` markup into the base text and, if the result is all kana, a
/// full reading.
fn strip_furigana(s: &str) -> (String, Option<String>) {
    if !RE_FURIGANA.is_match(s) {
        return (s.to_string(), None);
    }
    let base = RE_FURIGANA.replace_all(s, "$1").into_owned();
    let reading = RE_FURIGANA.replace_all(s, "$2").replace(' ', "");
    let reading = is_all_kana(&reading).then(|| to_hiragana(&reading));
    (base, reading)
}

static RE_MARKED: LazyLock<[Regex; 6]> = LazyLock::new(|| {
    let tag = |t: &str| Regex::new(&format!(r"(?is)<{t}(?:\s[^>]*)?>(.*?)</{t}>")).unwrap();
    [
        tag("b"),
        tag("strong"),
        tag("i"),
        tag("em"),
        tag("u"),
        Regex::new(r#"(?is)<span[^>]*style\s*=\s*"[^"]*font-weight:\s*(?:bold|[6-9]00)[^"]*"[^>]*>(.*?)</span>"#).unwrap(),
    ]
});

/// The bold, italic or underlined parts of a field, in field order (raw HTML inside each part).
/// Sentence cards often mark the word they teach this way: `<b>長篠</b>の戦いは…`.
/// A field as plain text: line breaks kept, tags, audio and entities removed.
pub fn plain_text(raw: &str) -> String {
    let s = RE_SOUND.replace_all(raw, "");
    let s = RE_BREAK.replace_all(&s, "\n");
    let s = RE_TAG.replace_all(&s, "");
    let s = decode_entities(&s);
    s.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

pub fn marked_text(raw: &str) -> Vec<String> {
    let mut found: Vec<(usize, usize, String)> = Vec::new();
    for re in RE_MARKED.iter() {
        for c in re.captures_iter(raw) {
            let whole = c.get(0).unwrap();
            found.push((whole.start(), whole.end(), c[1].to_string()));
        }
    }
    found.sort_by_key(|(start, end, _)| (*start, std::cmp::Reverse(*end)));
    let mut out = Vec::new();
    let mut covered = 0;
    for (start, end, inner) in found {
        // Skip a mark nested inside one already taken (<b><i>x</i></b>).
        if start < covered {
            continue;
        }
        covered = end;
        if !clean_field(&inner).is_empty() {
            out.push(inner);
        }
    }
    out
}

/// Cleans one field value. Returns the candidates in field order (several when the field lists
/// alternatives such as `分かる、解る`).
pub fn clean_field(raw: &str) -> Vec<Candidate> {
    let mut s = RE_SOUND.replace_all(raw, "").into_owned();
    s = RE_CLOZE.replace_all(&s, "$1").into_owned();

    // <ruby>漢字<rt>かんじ</rt></ruby> → the base, with the ruby text turned into Anki markup so
    // one code path handles both.
    s = RE_RP.replace_all(&s, "").into_owned();
    s = RE_RT.replace_all(&s, "[$1]").into_owned();
    s = RE_BREAK.replace_all(&s, "\n").into_owned();
    s = RE_TAG.replace_all(&s, "").into_owned();
    s = decode_entities(&s);
    s = s.nfkc().collect();

    let mut out = Vec::new();
    for part in s.split(['\n', '、', ',', ';', '/', '，', '；', '／']) {
        let mut reading_hint = None;
        // A bracketed kana gloss is a reading; anything else in brackets is a note.
        let mut text = RE_PAREN
            .replace_all(part, |c: &regex::Captures| {
                let inner = c[1].trim();
                if is_all_kana(inner) {
                    reading_hint = Some(to_hiragana(inner));
                }
                String::new()
            })
            .into_owned();
        let (base, furigana_reading) = strip_furigana(&text);
        text = base;
        if furigana_reading.is_some() {
            reading_hint = furigana_reading;
        }
        let text = RE_SPACE.replace_all(text.trim(), " ").into_owned();
        let text = text.trim_matches(|c: char| "。.!?！？「」『』\"'~〜".contains(c)).to_string();
        if !text.is_empty() {
            out.push(Candidate { text, reading_hint });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(raw: &str) -> Vec<(String, Option<String>)> {
        clean_field(raw).into_iter().map(|c| (c.text, c.reading_hint)).collect()
    }

    #[test]
    fn plain_word() {
        assert_eq!(texts("食べる"), vec![("食べる".into(), None)]);
    }

    #[test]
    fn html_and_entities() {
        assert_eq!(texts("<div><b>勉強</b>&nbsp;</div>"), vec![("勉強".into(), None)]);
        assert_eq!(texts("<span style=\"color:red\">先生</span>"), vec![("先生".into(), None)]);
    }

    #[test]
    fn anki_furigana_markup() {
        assert_eq!(texts("日本語[にほんご]"), vec![("日本語".into(), Some("にほんご".into()))]);
        assert_eq!(texts(" 食[た]べる"), vec![("食べる".into(), Some("たべる".into()))]);
        assert_eq!(texts("人気[ひとけ]"), vec![("人気".into(), Some("ひとけ".into()))]);
    }

    #[test]
    fn ruby_tags() {
        assert_eq!(
            texts("<ruby>漢字<rp>(</rp><rt>かんじ</rt><rp>)</rp></ruby>"),
            vec![("漢字".into(), Some("かんじ".into()))]
        );
    }

    #[test]
    fn cloze_and_sound() {
        assert_eq!(texts("{{c1::猫::animal}}[sound:neko.mp3]"), vec![("猫".into(), None)]);
    }

    #[test]
    fn lists_and_brackets() {
        assert_eq!(
            texts("分かる、解る"),
            vec![("分かる".into(), None), ("解る".into(), None)]
        );
        assert_eq!(texts("食べる (たべる)"), vec![("食べる".into(), Some("たべる".into()))]);
        assert_eq!(texts("取る（物を）"), vec![("取る".into(), None)]);
        assert_eq!(texts("〜ながら"), vec![("ながら".into(), None)]);
    }

    #[test]
    fn width_normalization() {
        assert_eq!(texts("ｺﾝﾋﾟｭｰﾀｰ"), vec![("コンピューター".into(), None)]);
    }

    #[test]
    fn marked_words_in_sentences() {
        assert_eq!(marked_text("<b>長篠</b>の戦いは日本の歴史で有名です。"), vec!["長篠"]);
        assert_eq!(marked_text("彼は<i>急いで</i>駅へ<u>向かった</u>。"), vec!["急いで", "向かった"]);
        assert_eq!(marked_text("<b><i>猫</i></b>が好き"), vec!["<i>猫</i>"]);
        assert_eq!(marked_text(r#"<span style="font-weight: bold;">本</span>を読む"#), vec!["本"]);
        assert_eq!(marked_text(r#"<span style="font-weight:700">本</span>"#), vec!["本"]);
        assert!(marked_text("太字なし").is_empty());
        assert!(marked_text("<b> </b>文").is_empty());
    }

    #[test]
    fn empty() {
        assert!(clean_field("<br>").is_empty());
        assert!(clean_field("[sound:x.mp3]").is_empty());
    }
}
