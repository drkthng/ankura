//! Serial stories (ADR 0019): episodes with the same characters and setting, each bringing back
//! words from earlier episodes, so target words recur over days (narrow reading).

use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::{Store, now_ms};
use crate::{Error, Result};

pub(crate) const SERIES_SCHEMA: &str = r#"
CREATE TABLE series (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    topic TEXT NOT NULL,
    bible TEXT NOT NULL DEFAULT '',  -- characters and setting, written with episode 1
    polite INTEGER NOT NULL,
    length INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
"#;

/// Earlier words brought back in each episode.
pub const CARRY: usize = 3;
/// Episode summaries the model sees.
const SO_FAR: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    pub id: String,
    pub title: String,
    pub topic: String,
    pub bible: String,
    pub polite: bool,
    pub length: usize,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Episode {
    pub text_id: String,
    pub title: String,
    pub episode: usize,
    pub targets: Vec<String>,
    pub summary: String,
    pub created_at: i64,
    pub opened_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SeriesInfo {
    pub series: Series,
    pub episodes: Vec<Episode>,
}

pub fn create(store: &Store, title: &str, topic: &str, polite: bool, length: usize) -> Result<Series> {
    let now = now_ms();
    let s = Series {
        id: Uuid::new_v4().to_string(),
        title: title.trim().to_string(),
        topic: topic.trim().to_string(),
        bible: String::new(),
        polite,
        length: length.clamp(150, 2000),
        created_at: now,
        updated_at: now,
    };
    store.conn.execute(
        "INSERT INTO series (id, title, topic, bible, polite, length, created_at, updated_at) VALUES (?1, ?2, ?3, '', ?4, ?5, ?6, ?6)",
        params![s.id, s.title, s.topic, s.polite, s.length as i64, now],
    )?;
    Ok(s)
}

pub fn get(store: &Store, id: &str) -> Result<Series> {
    store
        .conn
        .query_row(
            "SELECT id, title, topic, bible, polite, length, created_at, updated_at FROM series WHERE id = ?1",
            [id],
            |r| {
                Ok(Series {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    topic: r.get(2)?,
                    bible: r.get(3)?,
                    polite: r.get(4)?,
                    length: r.get::<_, i64>(5)? as usize,
                    created_at: r.get(6)?,
                    updated_at: r.get(7)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| Error::Llm("this series no longer exists".into()))
}

/// A series' episodes, oldest first.
pub fn episodes(store: &Store, id: &str) -> Result<Vec<Episode>> {
    let mut st = store.conn.prepare(
        "SELECT id, title, created_at, opened_at, extra FROM texts WHERE json_extract(extra, '$.series') = ?1 ORDER BY created_at",
    )?;
    let rows = st.query_map([id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, Option<i64>>(3)?, r.get::<_, String>(4)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (text_id, title, created_at, opened_at, extra) = row?;
        let v: serde_json::Value = serde_json::from_str(&extra).unwrap_or_default();
        out.push(Episode {
            text_id,
            title,
            episode: v["episode"].as_u64().unwrap_or(out.len() as u64 + 1) as usize,
            targets: v["targets"].as_array().into_iter().flatten().filter_map(|t| t.as_str().map(String::from)).collect(),
            summary: v["summary"].as_str().unwrap_or_default().to_string(),
            created_at,
            opened_at,
        });
    }
    Ok(out)
}

/// All series, the most recently continued first.
pub fn list(store: &Store) -> Result<Vec<SeriesInfo>> {
    let ids: Vec<String> = {
        let mut st = store.conn.prepare("SELECT id FROM series ORDER BY updated_at DESC")?;
        st.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
    };
    ids.iter().map(|id| Ok(SeriesInfo { series: get(store, id)?, episodes: episodes(store, id)? })).collect()
}

/// Deletes a series; its episodes stay in the library as ordinary texts.
pub fn delete(store: &Store, id: &str) -> Result<()> {
    store.conn.execute("DELETE FROM series WHERE id = ?1", [id])?;
    Ok(())
}

/// Earlier words to use again: from the last two episodes, the ones used least so far.
pub fn carry(eps: &[Episode], exclude: &[String]) -> Vec<String> {
    let mut count: HashMap<&str, usize> = HashMap::new();
    for e in eps {
        for t in &e.targets {
            *count.entry(t.as_str()).or_default() += 1;
        }
    }
    let mut recent: Vec<&String> = Vec::new();
    for e in eps.iter().rev().take(2) {
        for t in &e.targets {
            if !recent.contains(&t) && !exclude.contains(t) {
                recent.push(t);
            }
        }
    }
    // Stable sort: among equal counts, the most recent episode's words first.
    recent.sort_by_key(|t| count.get(t.as_str()).copied().unwrap_or(0));
    recent.into_iter().take(CARRY).cloned().collect()
}

/// What the model needs to write the next episode.
pub(crate) struct Context {
    pub series: Series,
    pub episode: usize,
    pub carry: Vec<String>,
    pub prompt: String,
}

pub(crate) fn context(store: &Store, id: &str, targets: &[String]) -> Result<Context> {
    let series = get(store, id)?;
    let eps = episodes(store, id)?;
    let episode = eps.len() + 1;
    let carry = carry(&eps, targets);
    let mut p = format!(
        "\nThis text is episode {episode} of a serial story called 「{}」. Keep the same characters, setting and style.\n",
        series.title
    );
    if series.bible.trim().is_empty() {
        p.push_str("Story bible: none yet. Invent two or three characters and a setting that suit the topic, and describe them in `bible` (a few lines in English: names with readings, who they are, where it happens).\n");
    } else {
        p.push_str(&format!("Story bible:\n{}\nReturn it in `bible`, adding any new lasting facts.\n", series.bible.trim()));
    }
    let so_far: Vec<&Episode> = eps.iter().rev().take(SO_FAR).collect::<Vec<_>>().into_iter().rev().collect();
    if !so_far.is_empty() {
        p.push_str("Story so far:\n");
        for e in so_far {
            p.push_str(&format!("- Episode {}: {}\n", e.episode, e.summary));
        }
        p.push_str("Continue from the last episode.\n");
    }
    p.push_str("End with something that makes the reader want the next episode. In `summary`, sum up this episode in one or two English sentences.\n");
    Ok(Context { series, episode, carry, prompt: p })
}

/// After an episode is saved: keep the bible, and move the series to the top of the list.
pub(crate) fn after_episode(store: &Store, id: &str, bible: &str) -> Result<()> {
    if bible.trim().is_empty() {
        store.conn.execute("UPDATE series SET updated_at = ?2 WHERE id = ?1", params![id, now_ms()])?;
    } else {
        store.conn.execute("UPDATE series SET bible = ?2, updated_at = ?3 WHERE id = ?1", params![id, bible.trim(), now_ms()])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(n: usize, targets: &[&str]) -> Episode {
        Episode {
            text_id: format!("t{n}"),
            title: String::new(),
            episode: n,
            targets: targets.iter().map(|t| t.to_string()).collect(),
            summary: format!("summary {n}"),
            created_at: n as i64,
            opened_at: None,
        }
    }

    #[test]
    fn carries_the_least_used_recent_words() {
        let eps = vec![ep(1, &["傘", "駅", "猫"]), ep(2, &["傘", "雨", "電車"]), ep(3, &["雨", "窓", "鍵"])];
        // From episodes 2-3; 傘 and 雨 came twice, so the once-used ones come first.
        assert_eq!(carry(&eps, &[]), ["窓", "鍵", "電車"]);
        // Equal counts: the more recent episode's word first (雨 before 傘).
        assert_eq!(carry(&eps, &["窓".to_string()]), ["鍵", "電車", "雨"]);
        assert!(carry(&[], &[]).is_empty());
    }

    #[test]
    fn series_and_context() {
        let s = Store::open_in_memory().unwrap();
        let x = create(&s, "駅の猫", "a cat at a station", false, 500).unwrap();
        let c = context(&s, &x.id, &["傘".into()]).unwrap();
        assert_eq!(c.episode, 1);
        assert!(c.prompt.contains("episode 1") && c.prompt.contains("Story bible: none yet"), "{}", c.prompt);
        s.add_text_with_extra("ja", "第一話", "本文", "generated", Some(&serde_json::json!({"series": x.id, "episode": 1, "targets": ["傘", "駅"], "summary": "A cat appears."}))).unwrap();
        after_episode(&s, &x.id, "Mika (みか), a station clerk; Tama, the cat.").unwrap();
        let c = context(&s, &x.id, &["雨".into()]).unwrap();
        assert_eq!(c.episode, 2);
        assert_eq!(c.carry, ["傘", "駅"]);
        assert!(c.prompt.contains("Mika") && c.prompt.contains("Episode 1: A cat appears."), "{}", c.prompt);
        let all = list(&s).unwrap();
        assert_eq!((all.len(), all[0].episodes.len()), (1, 1));
        delete(&s, &x.id).unwrap();
        assert!(list(&s).unwrap().is_empty());
        assert_eq!(s.list_texts().unwrap().len(), 1, "episodes stay as texts");
    }
}
