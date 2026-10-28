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

#[cfg(test)]
mod tests {
    use super::*;

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
