//! Ankura's memory model for its own review schedule (ADR 0022): FSRS-5 with the default
//! weights, as in open-spaced-repetition/rs-fsrs. Only what Review needs: three grades, no
//! learning steps, no fuzz.

/// FSRS-5 default weights.
const W: [f64; 19] = [
    0.4072, 1.1829, 3.1262, 15.4722, 7.2102, 0.5316, 1.0651, 0.0234, 1.616, 0.1544, 1.0824, 1.9813, 0.0953, 0.2975, 2.2042,
    0.2407, 2.9466, 0.5034, 0.6567,
];
const DECAY: f64 = -0.5;
/// 0.9^(1 / DECAY) - 1: with it, recall is 90% when the elapsed time equals the stability.
const FACTOR: f64 = 19.0 / 81.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// Not remembered.
    Again = 1,
    /// Remembered with help (a hint, choices, hearing it).
    Hard = 2,
    /// Recalled.
    Good = 3,
}

/// What Ankura remembers about one item: stability in days (the time until recall falls to 90%)
/// and difficulty (1-10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Memory {
    pub stability: f64,
    pub difficulty: f64,
}

/// Chance of recall after `elapsed_days` at this stability.
pub fn retrievability(elapsed_days: f64, stability: f64) -> f64 {
    (1.0 + FACTOR * elapsed_days.max(0.0) / stability.max(0.01)).powf(DECAY)
}

/// Days until recall falls to `retention` (0.9: the stability itself).
pub fn interval(stability: f64, retention: f64) -> f64 {
    stability / FACTOR * (retention.powf(1.0 / DECAY) - 1.0)
}

fn init_difficulty(grade: f64) -> f64 {
    (W[4] - (W[5] * (grade - 1.0)).exp() + 1.0).clamp(1.0, 10.0)
}

/// Memory after the first answer to an item Ankura knows nothing about.
pub fn first(grade: Grade) -> Memory {
    let g = grade as i32 as f64;
    Memory { stability: W[grade as usize - 1].max(0.1), difficulty: init_difficulty(g) }
}

/// Memory for a word Anki already schedules: the card's interval (or FSRS stability) in days.
pub fn from_anki(strength_days: f64) -> Memory {
    Memory { stability: strength_days.max(1.0), difficulty: 5.0 }
}

/// Memory after an answer, given the chance of recall just before it.
pub fn next(m: Memory, recall: f64, grade: Grade) -> Memory {
    let g = grade as i32 as f64;
    let d = m.difficulty.clamp(1.0, 10.0);
    let s = m.stability.max(0.01);
    let r = recall.clamp(0.0, 1.0);
    let stability = match grade {
        Grade::Again => (W[11] * d.powf(-W[12]) * ((s + 1.0).powf(W[13]) - 1.0) * ((1.0 - r) * W[14]).exp()).min(s),
        _ => {
            let modifier = if grade == Grade::Hard { W[15] } else { 1.0 };
            s * (1.0 + W[8].exp() * (11.0 - d) * s.powf(-W[9]) * (((1.0 - r) * W[10]).exp() - 1.0) * modifier)
        }
    };
    // Difficulty moves with the grade and drifts back towards the default.
    let moved = d - W[6] * (g - 3.0);
    let difficulty = (W[7] * init_difficulty(4.0) + (1.0 - W[7]) * moved).clamp(1.0, 10.0);
    Memory { stability: stability.max(0.01), difficulty }
}

/// Memory after an answer on the same day as the last one (FSRS-5's short-term rule).
pub fn next_same_day(m: Memory, grade: Grade) -> Memory {
    let g = grade as i32 as f64;
    let stability = m.stability * (W[17] * (g - 3.0 + W[18])).exp();
    let moved = m.difficulty - W[6] * (g - 3.0);
    let difficulty = (W[7] * init_difficulty(4.0) + (1.0 - W[7]) * moved).clamp(1.0, 10.0);
    Memory { stability: stability.max(0.01), difficulty }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_is_90_percent_at_the_stability() {
        assert!((retrievability(10.0, 10.0) - 0.9).abs() < 1e-9);
        assert!((retrievability(0.0, 10.0) - 1.0).abs() < 1e-9);
        assert!(retrievability(100.0, 10.0) < 0.6);
        assert!((interval(10.0, 0.9) - 10.0).abs() < 1e-9);
        assert!(interval(10.0, 0.8) > 10.0);
    }

    #[test]
    fn first_answers() {
        assert!((first(Grade::Good).stability - 3.1262).abs() < 1e-9);
        assert!(first(Grade::Again).stability < 1.0);
        assert!(first(Grade::Again).difficulty > first(Grade::Good).difficulty);
    }

    #[test]
    fn right_answers_grow_and_misses_shrink() {
        let m = Memory { stability: 10.0, difficulty: 5.0 };
        let good = next(m, 0.9, Grade::Good);
        let hard = next(m, 0.9, Grade::Hard);
        let again = next(m, 0.9, Grade::Again);
        assert!(good.stability > hard.stability && hard.stability > m.stability, "{good:?} {hard:?}");
        assert!(again.stability < m.stability && again.difficulty > m.difficulty, "{again:?}");
        // Recalling something nearly forgotten strengthens it more than recalling it early.
        assert!(next(m, 0.5, Grade::Good).stability > good.stability);
        assert!(next(m, 0.99, Grade::Good).stability < good.stability);
        // A miss never raises stability, even for a weak item.
        let weak = Memory { stability: 0.2, difficulty: 5.0 };
        assert!(next(weak, 0.3, Grade::Again).stability <= 0.2);
    }

    #[test]
    fn same_day_answers_move_little() {
        let m = Memory { stability: 2.0, difficulty: 5.0 };
        assert!(next_same_day(m, Grade::Good).stability > 2.0);
        assert!(next_same_day(m, Grade::Good).stability < 3.5);
        assert!(next_same_day(m, Grade::Again).stability < 2.0);
    }

    #[test]
    fn an_old_anki_card_keeps_its_strength() {
        let m = from_anki(300.0);
        let after = next(m, 0.7, Grade::Good);
        assert!(after.stability > 300.0, "{after:?}");
    }
}
