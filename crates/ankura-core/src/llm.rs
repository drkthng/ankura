//! LLM providers (ADR 0006, ADR 0010): Anthropic's Messages API and OpenRouter's OpenAI-compatible
//! API, behind one interface that returns JSON matching a schema. Raw HTTPS: there's no official
//! Rust SDK.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    #[default]
    Anthropic,
    OpenRouter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: Provider,
    pub model: String,
    /// Anthropic `output_config.effort`: low | medium | high | xhigh | max.
    pub effort: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        LlmConfig { provider: Provider::Anthropic, model: DEFAULT_ANTHROPIC_MODEL.into(), effort: "medium".into() }
    }
}

pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-opus-5-5";
const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2026-10-21";
/// `fallbacks: "default"`: if a safety classifier declines, Anthropic re-runs the request on its
/// recommended fallback model.
const ANTHROPIC_FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// One call: a system prompt, a user message, and a JSON schema the answer must follow.
pub trait Llm: Send + Sync {
    fn json(&self, system: &str, user: &str, schema: &Value) -> Result<Value>;
}

pub struct HttpLlm {
    config: LlmConfig,
    api_key: String,
    /// Overrides the provider URL (tests).
    url: Option<String>,
    agent: ureq::Agent,
}

impl HttpLlm {
    pub fn new(config: LlmConfig, api_key: String) -> Self {
        let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
        let agent = ureq::Agent::config_builder()
            .tls_config(tls)
            .timeout_global(Some(Duration::from_secs(600)))
            .http_status_as_error(false)
            .build()
            .new_agent();
        HttpLlm { config, api_key, url: None, agent }
    }

    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    fn post(&self, url: &str, headers: &[(&str, &str)], body: &Value) -> Result<Value> {
        let mut req = self.agent.post(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let mut resp = req.send_json(body).map_err(|e| Error::Llm(format!("can't reach the AI provider: {e}")))?;
        let status = resp.status().as_u16();
        let v: Value = resp.body_mut().read_json().map_err(|e| Error::Llm(format!("unreadable reply ({status}): {e}")))?;
        if status >= 400 {
            let msg = v["error"]["message"].as_str().or(v["error"].as_str()).unwrap_or("unknown error").to_string();
            let hint = match status {
                401 | 403 => " Check the API key in Settings.",
                429 => " Rate limited; try again in a minute.",
                529 | 503 => " The provider is overloaded; try again shortly.",
                _ => "",
            };
            return Err(Error::Llm(format!("{status}: {msg}.{hint}")));
        }
        Ok(v)
    }

    fn anthropic(&self, system: &str, user: &str, schema: &Value) -> Result<Value> {
        let body = json!({
            "model": self.config.model,
            "max_tokens": 16000,
            "system": system,
            "messages": [{"role": "user", "content": user}],
            "output_config": {
                "effort": self.config.effort,
                "format": {"type": "json_schema", "schema": schema},
            },
            "fallbacks": "default",
        });
        let headers = [
            ("x-api-key", self.api_key.as_str()),
            ("anthropic-version", ANTHROPIC_VERSION),
            ("anthropic-beta", ANTHROPIC_FALLBACK_BETA),
        ];
        let v = self.post(self.url.as_deref().unwrap_or(ANTHROPIC_URL), &headers, &body)?;
        match v["stop_reason"].as_str() {
            Some("refusal") => {
                let why = v["stop_details"]["explanation"].as_str().unwrap_or("no reason given");
                return Err(Error::Llm(format!("the model declined this request ({why})")));
            }
            Some("max_tokens") => return Err(Error::Llm("the answer was cut off (too long)".into())),
            _ => {}
        }
        let text: String = v["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["type"] == "text")
            .filter_map(|b| b["text"].as_str())
            .collect();
        serde_json::from_str(&text).map_err(|e| Error::Llm(format!("the answer wasn't valid JSON: {e}")))
    }

    fn openrouter(&self, system: &str, user: &str, schema: &Value) -> Result<Value> {
        let body = json!({
            "model": self.config.model,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
            "response_format": {"type": "json_schema", "json_schema": {"name": "ankura", "strict": true, "schema": schema}},
            // Only route to providers that honour the schema.
            "provider": {"require_parameters": true},
        });
        let auth = format!("Bearer {}", self.api_key);
        let headers = [("authorization", auth.as_str()), ("x-title", "Ankura")];
        let v = self.post(self.url.as_deref().unwrap_or(OPENROUTER_URL), &headers, &body)?;
        let content = v["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| Error::Llm("the provider sent no answer".into()))?;
        serde_json::from_str(content).map_err(|e| Error::Llm(format!("the answer wasn't valid JSON: {e}")))
    }
}

impl Llm for HttpLlm {
    fn json(&self, system: &str, user: &str, schema: &Value) -> Result<Value> {
        if self.api_key.trim().is_empty() {
            return Err(Error::Llm("no API key set; add one in Settings".into()));
        }
        match self.config.provider {
            Provider::Anthropic => self.anthropic(system, user, schema),
            Provider::OpenRouter => self.openrouter(system, user, schema),
        }
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::sync::{Arc, Mutex};
    use std::thread;

    use serde_json::{Value, json};

    /// Serves canned provider replies in order and records the request bodies.
    pub fn serve(replies: Vec<(u16, Value)>) -> (String, Arc<Mutex<Vec<Value>>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr().to_ip().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        thread::spawn(move || {
            let mut replies = replies.into_iter();
            for mut req in server.incoming_requests() {
                let mut body = String::new();
                req.as_reader().read_to_string(&mut body).unwrap();
                log.lock().unwrap().push(serde_json::from_str::<Value>(&body).unwrap_or(json!(null)));
                let (status, reply) = replies.next().unwrap_or((500, json!({"error": {"message": "no more replies"}})));
                req.respond(tiny_http::Response::from_string(reply.to_string()).with_status_code(status)).unwrap();
            }
        });
        (url, seen)
    }

    /// An Anthropic Messages reply whose text is `answer` serialized.
    pub fn anthropic_reply(answer: &Value) -> Value {
        json!({"stop_reason": "end_turn", "content": [{"type": "thinking", "thinking": ""}, {"type": "text", "text": answer.to_string()}]})
    }
}

#[cfg(test)]
mod tests {
    use super::fake::*;
    use super::*;

    fn schema() -> Value {
        json!({"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"], "additionalProperties": false})
    }

    #[test]
    fn anthropic_request_and_reply() {
        let (url, seen) = serve(vec![(200, anthropic_reply(&json!({"ok": true})))]);
        let llm = HttpLlm::new(LlmConfig::default(), "key".into()).with_url(&url);
        assert_eq!(llm.json("sys", "hi", &schema()).unwrap(), json!({"ok": true}));
        let body = &seen.lock().unwrap()[0];
        assert_eq!(body["model"], "claude-opus-5-5");
        assert_eq!(body["output_config"]["effort"], "medium");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["fallbacks"], "default");
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn anthropic_errors_are_readable() {
        let (url, _) = serve(vec![
            (401, json!({"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}})),
            (200, json!({"stop_reason": "refusal", "stop_details": {"explanation": "policy"}, "content": []})),
        ]);
        let llm = HttpLlm::new(LlmConfig::default(), "bad".into()).with_url(&url);
        let e = llm.json("s", "u", &schema()).unwrap_err().to_string();
        assert!(e.contains("401") && e.contains("API key"), "{e}");
        let e = llm.json("s", "u", &schema()).unwrap_err().to_string();
        assert!(e.contains("declined"), "{e}");
        let none = HttpLlm::new(LlmConfig::default(), "".into()).with_url(&url);
        assert!(none.json("s", "u", &schema()).unwrap_err().to_string().contains("no API key"));
    }

    #[test]
    fn openrouter_request_and_reply() {
        let (url, seen) =
            serve(vec![(200, json!({"choices": [{"message": {"content": "{\"ok\": false}"}}]}))]);
        let cfg = LlmConfig { provider: Provider::OpenRouter, model: "some/model".into(), effort: "medium".into() };
        let llm = HttpLlm::new(cfg, "key".into()).with_url(&url);
        assert_eq!(llm.json("sys", "hi", &schema()).unwrap(), json!({"ok": false}));
        let body = &seen.lock().unwrap()[0];
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert_eq!(body["provider"]["require_parameters"], true);
        assert_eq!(body["messages"][0]["role"], "system");
    }
}
