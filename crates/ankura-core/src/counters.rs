//! Readings of number + counter (三匹 さんびき, 一本 いっぽん, 六分 ろっぷん, 二日 ふつか). Sudachi reads
//! the two tokens separately (さん + ひき), so the sound changes are lost. This table restores
//! them for the common counters and the numbers 1-10 and 何 (ADR 0008 consequences).

use crate::tokenize::{Token, TokenKind};

/// Readings for 1-10, then 何, of number + counter.
const TABLE: &[(&str, [&str; 11])] = &[
    ("本", ["いっぽん", "にほん", "さんぼん", "よんほん", "ごほん", "ろっぽん", "ななほん", "はっぽん", "きゅうほん", "じゅっぽん", "なんぼん"]),
    ("匹", ["いっぴき", "にひき", "さんびき", "よんひき", "ごひき", "ろっぴき", "ななひき", "はっぴき", "きゅうひき", "じゅっぴき", "なんびき"]),
    ("杯", ["いっぱい", "にはい", "さんばい", "よんはい", "ごはい", "ろっぱい", "ななはい", "はっぱい", "きゅうはい", "じゅっぱい", "なんばい"]),
    ("分", ["いっぷん", "にふん", "さんぷん", "よんぷん", "ごふん", "ろっぷん", "ななふん", "はっぷん", "きゅうふん", "じゅっぷん", "なんぷん"]),
    ("階", ["いっかい", "にかい", "さんがい", "よんかい", "ごかい", "ろっかい", "ななかい", "はっかい", "きゅうかい", "じゅっかい", "なんがい"]),
    ("回", ["いっかい", "にかい", "さんかい", "よんかい", "ごかい", "ろっかい", "ななかい", "はっかい", "きゅうかい", "じゅっかい", "なんかい"]),
    ("個", ["いっこ", "にこ", "さんこ", "よんこ", "ごこ", "ろっこ", "ななこ", "はっこ", "きゅうこ", "じゅっこ", "なんこ"]),
    ("冊", ["いっさつ", "にさつ", "さんさつ", "よんさつ", "ごさつ", "ろくさつ", "ななさつ", "はっさつ", "きゅうさつ", "じゅっさつ", "なんさつ"]),
    ("歳", ["いっさい", "にさい", "さんさい", "よんさい", "ごさい", "ろくさい", "ななさい", "はっさい", "きゅうさい", "じゅっさい", "なんさい"]),
    ("才", ["いっさい", "にさい", "さんさい", "よんさい", "ごさい", "ろくさい", "ななさい", "はっさい", "きゅうさい", "じゅっさい", "なんさい"]),
    ("軒", ["いっけん", "にけん", "さんげん", "よんけん", "ごけん", "ろっけん", "ななけん", "はっけん", "きゅうけん", "じゅっけん", "なんげん"]),
    ("足", ["いっそく", "にそく", "さんぞく", "よんそく", "ごそく", "ろくそく", "ななそく", "はっそく", "きゅうそく", "じゅっそく", "なんぞく"]),
    ("件", ["いっけん", "にけん", "さんけん", "よんけん", "ごけん", "ろっけん", "ななけん", "はっけん", "きゅうけん", "じゅっけん", "なんけん"]),
    ("頭", ["いっとう", "にとう", "さんとう", "よんとう", "ごとう", "ろくとう", "ななとう", "はっとう", "きゅうとう", "じゅっとう", "なんとう"]),
    ("日", ["ついたち", "ふつか", "みっか", "よっか", "いつか", "むいか", "なのか", "ようか", "ここのか", "とおか", "なんにち"]),
    ("月", ["いちがつ", "にがつ", "さんがつ", "しがつ", "ごがつ", "ろくがつ", "しちがつ", "はちがつ", "くがつ", "じゅうがつ", "なんがつ"]),
    ("時", ["いちじ", "にじ", "さんじ", "よじ", "ごじ", "ろくじ", "しちじ", "はちじ", "くじ", "じゅうじ", "なんじ"]),
    ("人", ["ひとり", "ふたり", "さんにん", "よにん", "ごにん", "ろくにん", "しちにん", "はちにん", "きゅうにん", "じゅうにん", "なんにん"]),
];

/// How the number part of a combined reading ends, longest first, to split it from the counter.
const NUMBER_PARTS: &[&str] = &[
    "ここの", "きゅう", "じゅっ", "じゅう", "つい", "ふつ", "みっ", "よっ", "いつ", "むい", "なの", "よう", "とお", "ひと", "ふた",
    "いっ", "いち", "さん", "よん", "ろっ", "ろく", "なな", "しち", "はっ", "はち", "なん", "に", "ご", "よ", "し", "く",
];

/// 1-10 for a single numeral (一 or 1 … 十 or 10), 11 for 何.
fn number_value(s: &str) -> Option<usize> {
    let v = match s {
        "一" | "1" | "１" => 1,
        "二" | "2" | "２" => 2,
        "三" | "3" | "３" => 3,
        "四" | "4" | "４" => 4,
        "五" | "5" | "５" => 5,
        "六" | "6" | "６" => 6,
        "七" | "7" | "７" => 7,
        "八" | "8" | "８" => 8,
        "九" | "9" | "９" => 9,
        "十" | "10" | "１０" => 10,
        "何" => 11,
        _ => return None,
    };
    Some(v)
}

/// Splits a combined reading into the number's part and the counter's part.
fn split(reading: &str) -> (&str, &str) {
    for p in NUMBER_PARTS {
        if let Some(rest) = reading.strip_prefix(p) {
            if !rest.is_empty() {
                return (&reading[..p.len()], rest);
            }
        }
    }
    (reading, "")
}

/// Fixes the readings of number + counter pairs in place.
pub fn fix_readings(toks: &mut [Token]) {
    for i in 1..toks.len() {
        let Some(n) = number_value(&toks[i - 1].surface) else { continue };
        if !(toks[i - 1].kind == TokenKind::Number || toks[i - 1].surface == "何") {
            continue;
        }
        // A longer number (二十本, 1000円): only the plain 1-10 are in the table.
        if i >= 2 && toks[i - 2].kind == TokenKind::Number {
            continue;
        }
        let Some((_, readings)) = TABLE.iter().find(|(c, _)| *c == toks[i].surface) else { continue };
        // 三分の一 is さんぶんのいち: a fraction, not minutes.
        if toks[i].surface == "分" && toks.get(i + 1).is_some_and(|t| t.surface == "の") {
            continue;
        }
        // 一日 is いちにち (a day), ついたち only as a date, right after a month (一月一日).
        let combined = if toks[i].surface == "日" && n == 1 && !(i >= 2 && toks[i - 2].surface == "月") {
            "いちにち"
        } else {
            readings[n - 1]
        };
        let (num, counter) = split(combined);
        if counter.is_empty() {
            continue;
        }
        // Digits carry no furigana; their reading still matters for the counter's.
        toks[i - 1].reading = num.to_string();
        toks[i].reading = counter.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenize::Tokenizer;
    use crate::tokenize::test_support::sudachi;

    fn readings(s: &str) -> String {
        let t = sudachi().unwrap();
        t.tokenize(s).unwrap().iter().map(|x| format!("{}={}", x.surface, x.reading)).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn splits_readings() {
        assert_eq!(split("さんびき"), ("さん", "びき"));
        assert_eq!(split("いっぽん"), ("いっ", "ぽん"));
        assert_eq!(split("ふつか"), ("ふつ", "か"));
        assert_eq!(split("ここのか"), ("ここの", "か"));
        assert_eq!(split("じゅっぷん"), ("じゅっ", "ぷん"));
    }

    #[test]
    fn counters_get_their_sound_changes() {
        if sudachi().is_none() {
            return;
        }
        assert!(readings("猫が三匹いる。").contains("三=さん 匹=びき"), "{}", readings("猫が三匹いる。"));
        assert!(readings("一本のペン").starts_with("一=いっ 本=ぽん"), "{}", readings("一本のペン"));
        assert!(readings("3匹の犬").starts_with("3=さん 匹=びき"));
        assert!(readings("六分待った").starts_with("六=ろっ 分=ぷん"));
        assert!(readings("一杯").starts_with("一=いっ 杯=ぱい"));
        assert!(readings("三階").starts_with("三=さん 階=がい"));
        assert!(readings("二日目").starts_with("二=ふつ 日=か"), "{}", readings("二日目"));
        assert!(readings("一日中").starts_with("一=いち 日=にち"), "{}", readings("一日中"));
        assert!(readings("一月一日に").contains("一=つい 日=たち"), "{}", readings("一月一日に"));
        // A fraction, and longer numbers, are left alone.
        assert!(readings("三分の一").starts_with("三=さん 分=ぶん"), "{}", readings("三分の一"));
        // 二十 is one token; Sudachi's own 本=ぽん is kept.
        assert!(readings("二十本").starts_with("二十=にじゅう 本=ぽん"), "{}", readings("二十本"));
    }
}
