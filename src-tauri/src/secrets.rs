//! API keys in the OS keychain (ADR 0006): Windows Credential Manager, macOS Keychain, or the
//! Secret Service on Linux. Never in Ankura's database.

use ankura_core::llm::Provider;

const SERVICE: &str = "io.github.drkthng.ankura";

fn account(p: Provider) -> &'static str {
    match p {
        Provider::Anthropic => "anthropic-api-key",
        Provider::OpenRouter => "openrouter-api-key",
    }
}

/// Development and tests: `ANKURA_API_KEY` stands in for the keychain.
pub fn api_key(p: Provider) -> Result<Option<String>, String> {
    if let Ok(k) = std::env::var("ANKURA_API_KEY") {
        return Ok(Some(k));
    }
    let entry = keyring::Entry::new(SERVICE, account(p)).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("couldn't read the API key from the system keychain: {e}")),
    }
}

/// An empty key removes it.
pub fn set_api_key(p: Provider, key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, account(p)).map_err(|e| e.to_string())?;
    let r = if key.is_empty() {
        match entry.delete_credential() {
            Err(keyring::Error::NoEntry) => Ok(()),
            other => other,
        }
    } else {
        entry.set_password(key)
    };
    r.map_err(|e| format!("couldn't save the API key in the system keychain: {e}"))
}
