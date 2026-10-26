//! From Anki card state to word state (architecture-options §3).

use serde::{Deserialize, Serialize};

/// What Ankura knows about one Anki card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardFacts {
    pub card_id: i64,
    pub note_id: i64,
    /// Anki card type: 0 new, 1 learning, 2 review, 3 relearning.
    pub card_type: i64,
    /// Anki queue: negative values are suspended (-1) or buried (-2, -3).
    pub queue: i64,
    /// Interval in days (0 for learning cards).
    pub interval: i64,
    pub lapses: i64,
    /// Lower bound of the FSRS stability bucket, in days, when FSRS is on.
    pub stability: Option<f64>,
    /// Lower bound of the FSRS retrievability bucket (0-1), when FSRS is on.
    pub retrievability: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordState {
    /// No card, no mark.
    Unknown,
    /// Cards exist but none was reviewed yet: a word you're "supposed to know soon".
    New,
    /// Reviewed, not yet stable.
    Learning,
    /// Known, but Anki estimates it's slipping today.
    Fading,
    Known,
    /// The user marked it as known, or as something to ignore (names, foreign words).
    MarkedKnown,
    Ignored,
}

impl WordState {
    pub fn as_str(self) -> &'static str {
        match self {
            WordState::Unknown => "unknown",
            WordState::New => "new",
            WordState::Learning => "learning",
            WordState::Fading => "fading",
            WordState::Known => "known",
            WordState::MarkedKnown => "marked_known",
            WordState::Ignored => "ignored",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "unknown" => WordState::Unknown,
            "new" => WordState::New,
            "learning" => WordState::Learning,
            "fading" => WordState::Fading,
            "known" => WordState::Known,
            "marked_known" => WordState::MarkedKnown,
            "ignored" => WordState::Ignored,
            _ => return None,
        })
    }

    /// Counts as understood for the comprehension score.
    pub fn is_known(self) -> bool {
        matches!(self, WordState::Known | WordState::Fading | WordState::MarkedKnown | WordState::Ignored)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSettings {
    /// Stability (or interval, for SM-2 cards) at which a word counts as known.
    pub known_days: f64,
    /// Below this retrievability a known word shows as fading.
    pub fading_below: f64,
    /// Whether suspended cards still count.
    pub count_suspended: bool,
}

impl Default for KnowledgeSettings {
    fn default() -> Self {
        KnowledgeSettings { known_days: 21.0, fading_below: 0.8, count_suspended: false }
    }
}

/// Strength of one card in days; `None` if the card shouldn't count at all.
pub fn card_strength(c: &CardFacts, s: &KnowledgeSettings) -> Option<f64> {
    if c.queue == -1 && !s.count_suspended {
        return None;
    }
    Some(match c.card_type {
        0 => 0.0,
        1 | 3 => c.stability.unwrap_or(0.5).min(s.known_days - 0.01).max(0.5),
        _ => c.stability.unwrap_or(c.interval as f64).max(1.0),
    })
}

/// The word's state from all cards mapped to it. One strong card is enough (AnkiMorphs' rule).
pub fn word_state<'a>(cards: impl IntoIterator<Item = &'a CardFacts>, s: &KnowledgeSettings) -> (WordState, f64) {
    let mut best: Option<(&CardFacts, f64)> = None;
    for c in cards {
        if let Some(strength) = card_strength(c, s) {
            if best.is_none_or(|(_, b)| strength > b) {
                best = Some((c, strength));
            }
        }
    }
    let Some((card, strength)) = best else {
        return (WordState::Unknown, 0.0);
    };
    let state = if card.card_type == 0 {
        WordState::New
    } else if strength < s.known_days {
        WordState::Learning
    } else if card.retrievability.is_some_and(|r| r < s.fading_below) {
        WordState::Fading
    } else {
        WordState::Known
    };
    (state, strength)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(card_type: i64, queue: i64, interval: i64, s: Option<f64>, r: Option<f64>) -> CardFacts {
        CardFacts { card_id: 1, note_id: 1, card_type, queue, interval, lapses: 0, stability: s, retrievability: r }
    }

    #[test]
    fn states() {
        let st = KnowledgeSettings::default();
        assert_eq!(word_state([], &st).0, WordState::Unknown);
        assert_eq!(word_state([&card(0, 0, 0, None, None)], &st).0, WordState::New);
        assert_eq!(word_state([&card(1, 1, 0, None, None)], &st).0, WordState::Learning);
        assert_eq!(word_state([&card(2, 2, 10, None, None)], &st).0, WordState::Learning);
        assert_eq!(word_state([&card(2, 2, 30, None, None)], &st).0, WordState::Known);
        assert_eq!(word_state([&card(2, 2, 30, Some(90.0), Some(0.7))], &st).0, WordState::Fading);
    }

    #[test]
    fn stability_wins_over_interval() {
        let st = KnowledgeSettings::default();
        // FSRS says stability is low even though the interval is long.
        assert_eq!(word_state([&card(2, 2, 40, Some(7.0), Some(0.9))], &st).0, WordState::Learning);
    }

    #[test]
    fn strongest_card_wins_and_suspended_is_skipped() {
        let st = KnowledgeSettings::default();
        let weak = card(2, 2, 3, None, None);
        let strong = card(2, 2, 100, None, None);
        assert_eq!(word_state([&weak, &strong], &st).0, WordState::Known);
        let suspended = card(2, -1, 100, None, None);
        assert_eq!(word_state([&weak, &suspended], &st).0, WordState::Learning);
        assert_eq!(word_state([&suspended], &st).0, WordState::Unknown);
        let with_suspended = KnowledgeSettings { count_suspended: true, ..st };
        assert_eq!(word_state([&suspended], &with_suspended).0, WordState::Known);
    }
}
