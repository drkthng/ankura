//! VOICEVOX (ADR 0017): a free, offline Japanese speech engine the learner runs next to Ankura.
//! Ankura asks its local HTTP API for speech and keeps every result on disk, so a sentence is
//! synthesized once (ADR 0016).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, Result};

pub const DEFAULT_URL: &str = "http://127.0.0.1:50021";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub name: String,
    pub id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speaker {
    pub name: String,
    pub styles: Vec<Style>,
}

pub struct Voicevox {
    url: String,
    agent: ureq::Agent,
}

impl Voicevox {
    pub fn new(url: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(2)))
            .timeout_global(Some(Duration::from_secs(120)))
            .build()
            .new_agent();
        Voicevox { url: url.trim_end_matches('/').to_string(), agent }
    }

    fn unreachable(&self, e: impl std::fmt::Display) -> Error {
        Error::Tts(format!("can't reach VOICEVOX at {}: {e}. Is it running?", self.url))
    }

    /// The voices the engine offers (each speaker has styles: normal, happy, whisper …).
    pub fn speakers(&self) -> Result<Vec<Speaker>> {
        let mut r = self.agent.get(format!("{}/speakers", self.url)).call().map_err(|e| self.unreachable(e))?;
        r.body_mut().read_json().map_err(|e| Error::Tts(format!("unexpected answer from VOICEVOX: {e}")))
    }

    /// WAV audio for a text; `speed` 1.0 is normal.
    pub fn synthesize(&self, text: &str, style: u32, speed: f64) -> Result<Vec<u8>> {
        let mut q: Value = self
            .agent
            .post(format!("{}/audio_query", self.url))
            .query("text", text)
            .query("speaker", style.to_string())
            .send_empty()
            .map_err(|e| self.unreachable(e))?
            .body_mut()
            .read_json()
            .map_err(|e| Error::Tts(format!("unexpected answer from VOICEVOX: {e}")))?;
        q["speedScale"] = serde_json::json!(speed.clamp(0.5, 2.0));
        let mut r = self
            .agent
            .post(format!("{}/synthesis", self.url))
            .query("speaker", style.to_string())
            .send_json(&q)
            .map_err(|e| self.unreachable(e))?;
        r.body_mut()
            .with_config()
            .limit(50 * 1024 * 1024)
            .read_to_vec()
            .map_err(|e| Error::Tts(format!("VOICEVOX audio: {e}")))
    }

    /// Like `synthesize`, but from `cache_dir` when this text was spoken before with the same
    /// voice and speed; new audio is stored there.
    pub fn speak(&self, cache_dir: &Path, text: &str, style: u32, speed: f64) -> Result<Vec<u8>> {
        let path = cache_path(cache_dir, text, style, speed);
        if let Ok(bytes) = std::fs::read(&path) {
            return Ok(bytes);
        }
        let bytes = self.synthesize(text, style, speed)?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Written under another name first, so a half-written file is never read as audio.
        let tmp = path.with_extension("part");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &path)?;
        Ok(bytes)
    }
}

/// FNV-1a: a stable hash for cache file names (std's hasher may change between Rust versions).
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
}

fn cache_path(dir: &Path, text: &str, style: u32, speed: f64) -> PathBuf {
    dir.join(style.to_string()).join(format!("{:016x}.wav", fnv(&format!("{speed:.2}|{text}"))))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A stand-in engine: answers /speakers, /audio_query and /synthesis, and records requests.
    fn serve() -> (String, Arc<Mutex<Vec<String>>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for mut req in server.incoming_requests() {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                log.lock().unwrap().push(format!("{} {} {body}", req.method(), req.url()));
                let reply = if req.url().starts_with("/speakers") {
                    tiny_http::Response::from_string(r#"[{"name":"四国めたん","speaker_uuid":"x","styles":[{"name":"ノーマル","id":2}]}]"#)
                } else if req.url().starts_with("/audio_query") {
                    tiny_http::Response::from_string(r#"{"accent_phrases":[],"speedScale":1.0}"#)
                } else {
                    tiny_http::Response::from_data(b"RIFF....WAVEfake".to_vec())
                };
                let _ = req.respond(reply);
            }
        });
        (url, seen)
    }

    #[test]
    fn speakers_and_cached_speech() {
        let (url, seen) = serve();
        let v = Voicevox::new(&url);
        let sp = v.speakers().unwrap();
        assert_eq!((sp[0].name.as_str(), sp[0].styles[0].id), ("四国めたん", 2));
        let dir = tempfile::tempdir().unwrap();
        let a = v.speak(dir.path(), "雨が降っています。", 2, 0.9).unwrap();
        assert_eq!(a, b"RIFF....WAVEfake");
        let log = seen.lock().unwrap().clone();
        assert!(log.iter().any(|l| l.starts_with("POST /audio_query?text=") && l.contains("speaker=2")), "{log:?}");
        assert!(log.iter().any(|l| l.starts_with("POST /synthesis?speaker=2") && l.replace([' ', '\n'], "").contains("\"speedScale\":0.9")), "{log:?}");
        // Spoken again: from the cache, no request.
        let n = log.len();
        assert_eq!(v.speak(dir.path(), "雨が降っています。", 2, 0.9).unwrap(), a);
        assert_eq!(seen.lock().unwrap().len(), n);
        // Another speed is another file.
        v.speak(dir.path(), "雨が降っています。", 2, 1.2).unwrap();
        assert_eq!(seen.lock().unwrap().len(), n + 2);
    }

    #[test]
    fn says_when_the_engine_is_off() {
        let e = Voicevox::new("http://127.0.0.1:9").speakers().unwrap_err().to_string();
        assert!(e.contains("Is it running?"), "{e}");
    }
}
