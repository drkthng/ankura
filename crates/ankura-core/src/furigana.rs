//! Splits a word and its reading into ruby segments, so furigana sits only over the kanji.
//!
//! 食べる + たべる → 食(た) べる; 取り扱い + とりあつかい → 取(と) り 扱(あつか) い.

use serde::{Deserialize, Serialize};

use crate::kana::{is_kana, is_kanji, to_hiragana};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub text: String,
    /// Reading to show above `text`, or `None` for kana and other text that needs none.
    pub ruby: Option<String>,
}

impl Segment {
    fn plain(text: &str) -> Self {
        Segment { text: text.to_string(), ruby: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunKind {
    Kana,
    Other,
}

fn runs(surface: &str) -> Vec<(RunKind, Vec<char>)> {
    let mut out: Vec<(RunKind, Vec<char>)> = Vec::new();
    for c in surface.chars() {
        // ヶ counts as kanji here (三ヶ月 → さんかげつ), so check kanji first.
        let kind = if !is_kanji(c) && is_kana(c) { RunKind::Kana } else { RunKind::Other };
        match out.last_mut() {
            Some((k, chars)) if *k == kind => chars.push(c),
            _ => out.push((kind, vec![c])),
        }
    }
    out
}

/// Assigns reading spans to runs. Kana runs must match the reading exactly (compared as
/// hiragana); each other run takes at least one reading character.
fn assign(runs: &[(RunKind, Vec<char>)], reading: &[char], out: &mut Vec<usize>) -> bool {
    let Some(((kind, chars), rest)) = runs.split_first() else {
        return reading.is_empty();
    };
    match kind {
        RunKind::Kana => {
            let want: Vec<char> = to_hiragana(&chars.iter().collect::<String>()).chars().collect();
            if reading.len() >= want.len() && reading[..want.len()] == want[..] {
                out.push(want.len());
                if assign(rest, &reading[want.len()..], out) {
                    return true;
                }
                out.pop();
            }
            false
        }
        RunKind::Other => {
            let min_rest = rest.iter().filter(|(k, _)| *k == RunKind::Other).count()
                + rest.iter().filter(|(k, _)| *k == RunKind::Kana).map(|(_, c)| c.len()).sum::<usize>();
            let max = reading.len().saturating_sub(min_rest);
            for len in 1..=max {
                out.push(len);
                if assign(rest, &reading[len..], out) {
                    return true;
                }
                out.pop();
            }
            false
        }
    }
}

/// `reading` may be katakana or hiragana. Returns one plain segment when no ruby is needed.
pub fn segments(surface: &str, reading: &str) -> Vec<Segment> {
    let reading_h = to_hiragana(reading);
    if surface.is_empty() || reading_h.is_empty() || !surface.chars().any(is_kanji) {
        return vec![Segment::plain(surface)];
    }
    if to_hiragana(surface) == reading_h {
        return vec![Segment::plain(surface)];
    }
    let rs = runs(surface);
    let reading_chars: Vec<char> = reading_h.chars().collect();
    let mut lens = Vec::new();
    if !assign(&rs, &reading_chars, &mut lens) {
        return vec![Segment { text: surface.to_string(), ruby: Some(reading_h) }];
    }
    let mut out = Vec::new();
    let mut pos = 0;
    for ((kind, chars), len) in rs.iter().zip(lens) {
        let text: String = chars.iter().collect();
        let ruby: String = reading_chars[pos..pos + len].iter().collect();
        pos += len;
        let needs_ruby = *kind == RunKind::Other && chars.iter().any(|&c| is_kanji(c));
        out.push(Segment { text, ruby: needs_ruby.then_some(ruby) });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(segs: &[Segment]) -> String {
        segs.iter()
            .map(|s| match &s.ruby {
                Some(r) => format!("{}({})", s.text, r),
                None => s.text.clone(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn okurigana() {
        assert_eq!(render(&segments("食べる", "タベル")), "食(た) べる");
        assert_eq!(render(&segments("取り扱い", "とりあつかい")), "取(と) り 扱(あつか) い");
        assert_eq!(render(&segments("行っ", "イッ")), "行(い) っ");
    }

    #[test]
    fn prefix_kana() {
        assert_eq!(render(&segments("お茶", "おちゃ")), "お 茶(ちゃ)");
    }

    #[test]
    fn whole_word() {
        assert_eq!(render(&segments("東京", "トウキョウ")), "東京(とうきょう)");
        assert_eq!(render(&segments("今日", "きょう")), "今日(きょう)");
        assert_eq!(render(&segments("三ヶ月", "さんかげつ")), "三ヶ月(さんかげつ)");
    }

    #[test]
    fn no_ruby_needed() {
        assert_eq!(render(&segments("コンピューター", "コンピューター")), "コンピューター");
        assert_eq!(render(&segments("きれい", "キレイ")), "きれい");
        assert_eq!(render(&segments("2026", "ニセンニジュウロク")), "2026");
    }

    #[test]
    fn mismatch_falls_back_to_whole_word() {
        // The kana in the surface doesn't occur in the reading.
        assert_eq!(render(&segments("見る", "みた")), "見る(みた)");
    }
}
