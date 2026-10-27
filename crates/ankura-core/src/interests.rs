//! The interest profile (ROADMAP M3): topics the learner likes, weighted, and topics to avoid.
//! It steers "Write for me": the topic ideas come from it, and with no topic given Ankura picks one
//! by weight. The AI can suggest topics from what the learner reads.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::llm::Llm;
use crate::store::Store;
use crate::{Error, Result};

const SETTING: &str = "interests";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Topic {
    pub name: String,
    /// 1 less, 2 normal, 3 more.
    pub weight: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Interests {
    pub topics: Vec<Topic>,
    pub avoid: Vec<String>,
}

pub fn get(store: &Store) -> Result<Interests> {
    Ok(store.setting(SETTING)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
}

pub fn set(store: &Store, i: &Interests) -> Result<()> {
    let mut clean = i.clone();
    clean.topics.retain(|t| !t.name.trim().is_empty());
    for t in &mut clean.topics {
        t.name = t.name.trim().to_string();
        t.weight = t.weight.clamp(1, 3);
    }
    clean.avoid = clean.avoid.iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
    store.set_setting(SETTING, &serde_json::to_string(&clean)?)
}

/// A topic picked by weight; `roll` is a number in 0..1 (random in the app, fixed in tests).
pub fn pick(i: &Interests, roll: f64) -> Option<String> {
    let total: u32 = i.topics.iter().map(|t| u32::from(t.weight)).sum();
    if total == 0 {
        return None;
    }
    let mut at = roll.clamp(0.0, 0.999_999) * f64::from(total);
    for t in &i.topics {
        at -= f64::from(t.weight);
        if at < 0.0 {
            return Some(t.name.clone());
        }
    }
    i.topics.last().map(|t| t.name.clone())
}

/// A random number in 0..1 without a random-number crate.
pub fn roll() -> f64 {
    (uuid::Uuid::new_v4().as_u128() % 1_000_000) as f64 / 1_000_000.0
}

const SUGGEST_SYSTEM: &str = "You help a Japanese learner find topics they'd enjoy reading about. From the texts \
they chose to read and the topics they asked for, suggest short topic labels (2-5 words each, in English) that \
capture their interests: specific enough to write a story or article about, varied, not repeating the ones they \
already have.";

/// Topics the AI suggests from what the learner pasted and the topics they asked for.
pub fn suggest(store: &Store, llm: &dyn Llm) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    {
        let mut st = store.conn.prepare(
            "SELECT title, substr(body, 1, 160), source, json_extract(extra, '$.topic') FROM texts
             ORDER BY coalesce(opened_at, created_at) DESC LIMIT 30",
        )?;
        let rows = st.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<String>>(3)?))
        })?;
        for row in rows {
            let (title, start, source, topic) = row?;
            match (source.as_str(), topic) {
                ("generated", Some(t)) if !t.trim().is_empty() => lines.push(format!("- asked for a text about: {t}")),
                ("generated", _) => {}
                _ => lines.push(format!("- read: {title} — {}", start.replace('\n', " "))),
            }
        }
    }
    if lines.is_empty() {
        return Err(Error::Llm("read or write a few texts first; there's nothing to go on yet".into()));
    }
    let have = get(store)?;
    let mut prompt = format!("What the learner read or asked for, newest first:\n{}\n", lines.join("\n"));
    if !have.topics.is_empty() {
        prompt.push_str(&format!(
            "\nTopics they already have: {}\n",
            have.topics.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    if !have.avoid.is_empty() {
        prompt.push_str(&format!("Topics they want to avoid: {}\n", have.avoid.join(", ")));
    }
    let schema = json!({
        "type": "object",
        "properties": {"topics": {"type": "array", "items": {"type": "string"}}},
        "required": ["topics"],
        "additionalProperties": false
    });
    let v: Value = llm.json(SUGGEST_SYSTEM, &prompt, &schema)?;
    let existing: Vec<String> = have.topics.iter().map(|t| t.name.to_lowercase()).collect();
    Ok(v["topics"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t.as_str())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && !existing.contains(&t.to_lowercase()))
        .take(10)
        .collect())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn stored_cleaned_and_picked_by_weight() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(get(&s).unwrap(), Interests::default());
        set(&s, &Interests {
            topics: vec![
                Topic { name: " cooking ".into(), weight: 1 },
                Topic { name: "".into(), weight: 3 },
                Topic { name: "trains".into(), weight: 9 },
            ],
            avoid: vec!["horror".into(), " ".into()],
        })
        .unwrap();
        let i = get(&s).unwrap();
        assert_eq!(i.topics, vec![Topic { name: "cooking".into(), weight: 1 }, Topic { name: "trains".into(), weight: 3 }]);
        assert_eq!(i.avoid, vec!["horror"]);
        // cooking takes the first quarter, trains the rest.
        assert_eq!(pick(&i, 0.1).as_deref(), Some("cooking"));
        assert_eq!(pick(&i, 0.3).as_deref(), Some("trains"));
        assert_eq!(pick(&i, 1.0).as_deref(), Some("trains"));
        assert_eq!(pick(&Interests::default(), 0.5), None);
        assert!((0.0..1.0).contains(&roll()));
    }

    struct Answer(Mutex<Vec<String>>);
    impl Llm for Answer {
        fn json(&self, _s: &str, user: &str, _schema: &Value) -> Result<Value> {
            self.0.lock().unwrap().push(user.to_string());
            Ok(json!({"topics": ["Railway travel", "trains", "Osaka food"]}))
        }
    }

    #[test]
    fn suggests_from_reading() {
        let s = Store::open_in_memory().unwrap();
        let llm = Answer(Mutex::new(vec![]));
        assert!(suggest(&s, &llm).is_err(), "nothing read yet");
        s.add_text("ja", "駅の朝", "毎朝、私は七時の電車に乗って会社へ行く。", "paste").unwrap();
        s.add_text_with_extra("ja", "雨", "…", "generated", Some(&json!({"topic": "a train journey"}))).unwrap();
        set(&s, &Interests { topics: vec![Topic { name: "Trains".into(), weight: 2 }], avoid: vec!["horror".into()] }).unwrap();
        assert_eq!(suggest(&s, &llm).unwrap(), vec!["Railway travel", "Osaka food"], "known topics are left out");
        let prompt = &llm.0.lock().unwrap()[0];
        assert!(prompt.contains("read: 駅の朝 — 毎朝") && prompt.contains("asked for a text about: a train journey"), "{prompt}");
        assert!(prompt.contains("avoid: horror"), "{prompt}");
    }
}
