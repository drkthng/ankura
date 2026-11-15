//! Character classes and kana conversion for Japanese.

pub fn is_hiragana(c: char) -> bool {
    ('ぁ'..='ゖ').contains(&c) || c == 'ゝ' || c == 'ゞ'
}

pub fn is_katakana(c: char) -> bool {
    ('ァ'..='ヺ').contains(&c) || c == 'ヽ' || c == 'ヾ' || ('ｦ'..='ﾟ').contains(&c)
}

/// Kana plus the long-vowel mark and the iteration marks.
pub fn is_kana(c: char) -> bool {
    is_hiragana(c) || is_katakana(c) || c == 'ー'
}

/// CJK ideographs, including the iteration mark 々 and the compatibility block.
pub fn is_kanji(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2FFFF}'
        | '々' | '〆' | 'ヶ')
}

pub fn has_kanji(s: &str) -> bool {
    s.chars().any(is_kanji)
}

pub fn kanji_set(s: &str) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for c in s.chars().filter(|&c| is_kanji(c) && c != '々') {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// Katakana to hiragana. Characters without a hiragana counterpart (ー, ヷ, …) are kept, and so
/// are ヵ/ヶ, which act as counters (三ヶ月) rather than as kana.
pub fn to_hiragana(s: &str) -> String {
    s.chars()
        .map(|c| {
            if ('ァ'..='ヴ').contains(&c) || c == 'ヽ' || c == 'ヾ' {
                char::from_u32(c as u32 - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// Hiragana to katakana; everything else is kept.
pub fn to_katakana(s: &str) -> String {
    s.chars()
        .map(|c| if ('ぁ'..='ゖ').contains(&c) || c == 'ゝ' || c == 'ゞ' { char::from_u32(c as u32 + 0x60).unwrap_or(c) } else { c })
        .collect()
}

pub fn is_all_kana(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_kana)
}

/// Consonant(s) before a vowel → the kana for a, i, u, e, o ("" where there is none).
const ROMAJI: &[(&str, [&str; 5])] = &[
    ("", ["あ", "い", "う", "え", "お"]),
    ("k", ["か", "き", "く", "け", "こ"]),
    ("g", ["が", "ぎ", "ぐ", "げ", "ご"]),
    ("s", ["さ", "し", "す", "せ", "そ"]),
    ("z", ["ざ", "じ", "ず", "ぜ", "ぞ"]),
    ("t", ["た", "ち", "つ", "て", "と"]),
    ("d", ["だ", "ぢ", "づ", "で", "ど"]),
    ("n", ["な", "に", "ぬ", "ね", "の"]),
    ("h", ["は", "ひ", "ふ", "へ", "ほ"]),
    ("b", ["ば", "び", "ぶ", "べ", "ぼ"]),
    ("p", ["ぱ", "ぴ", "ぷ", "ぺ", "ぽ"]),
    ("m", ["ま", "み", "む", "め", "も"]),
    ("r", ["ら", "り", "る", "れ", "ろ"]),
    ("y", ["や", "", "ゆ", "", "よ"]),
    ("w", ["わ", "", "", "", "を"]),
    ("f", ["ふぁ", "ふぃ", "ふ", "ふぇ", "ふぉ"]),
    ("j", ["じゃ", "じ", "じゅ", "じぇ", "じょ"]),
    ("sh", ["しゃ", "し", "しゅ", "しぇ", "しょ"]),
    ("ch", ["ちゃ", "ち", "ちゅ", "ちぇ", "ちょ"]),
    ("ts", ["", "", "つ", "", ""]),
    ("ky", ["きゃ", "", "きゅ", "", "きょ"]),
    ("gy", ["ぎゃ", "", "ぎゅ", "", "ぎょ"]),
    ("sy", ["しゃ", "", "しゅ", "", "しょ"]),
    ("zy", ["じゃ", "", "じゅ", "", "じょ"]),
    ("jy", ["じゃ", "", "じゅ", "", "じょ"]),
    ("ty", ["ちゃ", "", "ちゅ", "", "ちょ"]),
    ("dy", ["ぢゃ", "", "ぢゅ", "", "ぢょ"]),
    ("ny", ["にゃ", "", "にゅ", "", "にょ"]),
    ("hy", ["ひゃ", "", "ひゅ", "", "ひょ"]),
    ("by", ["びゃ", "", "びゅ", "", "びょ"]),
    ("py", ["ぴゃ", "", "ぴゅ", "", "ぴょ"]),
    ("my", ["みゃ", "", "みゅ", "", "みょ"]),
    ("ry", ["りゃ", "", "りゅ", "", "りょ"]),
];

fn vowel_index(c: char) -> Option<usize> {
    "aiueo".chars().position(|v| v == c)
}

/// Romaji as typed on a keyboard without a Japanese input method (neko, gakkou, shinbun, kon'ya)
/// to hiragana. `None` when something in it isn't romaji.
pub fn romaji_to_hiragana(s: &str) -> Option<String> {
    let c: Vec<char> = s.trim().to_lowercase().chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '-' {
            out.push('ー');
            i += 1;
            continue;
        }
        if ch == ' ' || ch == '\'' {
            i += 1;
            continue;
        }
        if !ch.is_ascii_lowercase() {
            return None;
        }
        let next = c.get(i + 1).copied();
        if ch == 'n' {
            match next {
                // na, nya: a syllable, handled below.
                Some(v) if vowel_index(v).is_some() || v == 'y' => {}
                // nn before a vowel or y is ん + a syllable (onna); otherwise nn is one ん.
                Some('n') => {
                    out.push('ん');
                    let after = c.get(i + 2).copied();
                    i += if after.is_some_and(|a| vowel_index(a).is_some() || a == 'y') { 1 } else { 2 };
                    continue;
                }
                _ => {
                    out.push('ん');
                    i += 1;
                    continue;
                }
            }
        } else if vowel_index(ch).is_none() && (next == Some(ch) || (ch == 't' && next == Some('c'))) {
            // A doubled consonant (gakkou, matcha).
            out.push('っ');
            i += 1;
            continue;
        }
        let mut matched = false;
        for len in (0..=2).rev() {
            let Some(v) = c.get(i + len).copied().and_then(vowel_index) else { continue };
            let prefix: String = c[i..i + len].iter().collect();
            if let Some((_, kana)) = ROMAJI.iter().find(|(p, _)| *p == prefix) {
                if !kana[v].is_empty() {
                    out.push_str(kana[v]);
                    i += len + 1;
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_romaji() {
        let r = |s: &str| romaji_to_hiragana(s).unwrap();
        assert_eq!(r("neko"), "ねこ");
        assert_eq!(r("Kyou"), "きょう");
        assert_eq!(r("gakkou"), "がっこう");
        assert_eq!(r("shinbun"), "しんぶん");
        assert_eq!(r("konnichiha"), "こんにちは");
        assert_eq!(r("onna"), "おんな");
        assert_eq!(r("kan'i"), "かんい");
        assert_eq!(r("kann"), "かん");
        assert_eq!(r("matcha"), "まっちゃ");
        assert_eq!(r("chiisai"), "ちいさい");
        assert_eq!(r("tsukue"), "つくえ");
        assert_eq!(r("wasureru"), "わすれる");
        assert_eq!(r("ryokou"), "りょこう");
        assert_eq!(r("fune"), "ふね");
        assert_eq!(r("jisho"), "じしょ");
        assert_eq!(r("ra-men"), "らーめん");
        assert_eq!(romaji_to_hiragana("xq"), None);
        assert_eq!(romaji_to_hiragana("ねこ"), None);
    }

    #[test]
    fn converts_katakana() {
        assert_eq!(to_hiragana("タベル"), "たべる");
        assert_eq!(to_hiragana("コンピューター"), "こんぴゅーたー");
        assert_eq!(to_hiragana("三ヶ月"), "三ヶ月");
    }

    #[test]
    fn classifies() {
        assert!(is_kanji('漢'));
        assert!(is_kanji('々'));
        assert!(!is_kanji('か'));
        assert!(is_all_kana("ひらがなカタカナー"));
        assert!(!is_all_kana("漢字"));
        assert_eq!(kanji_set("日本語の日々"), vec!['日', '本', '語']);
    }
}
