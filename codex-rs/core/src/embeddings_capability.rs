//! Capability checks for embeddings providers.
//! Per policy: OpenAI embeddings require an OpenAI API key. ChatGPT login
//! (web tokens) is not sufficient and does not grant embeddings access.

use crate::config::Config;
use codex_login::{OPENAI_API_KEY_ENV_VAR, get_auth_file, try_read_auth_json};

/// Returns true if OpenAI embeddings can be used in the current environment.
///
/// Rules:
/// - True when `OPENAI_API_KEY` is available via env or auth.json.
/// - ChatGPT web login tokens do NOT enable OpenAI embeddings.
pub fn can_use_openai_embeddings(config: &Config) -> bool {
    // 1) Check environment variable directly
    if std::env::var(OPENAI_API_KEY_ENV_VAR)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    // 2) Check auth.json stored API key
    let auth_file = get_auth_file(&config.codex_home);
    if let Ok(auth) = try_read_auth_json(&auth_file)
        && auth
            .openai_api_key
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    {
        return true;
    }

    false
}

/// Optional helper to explain why embeddings are unavailable.
pub fn openai_embeddings_unavailable_reason(config: &Config) -> Option<String> {
    if can_use_openai_embeddings(config) {
        return None;
    }
    Some("OpenAI embeddings require an OPENAI_API_KEY. ChatGPT login tokens do not grant API access.".to_string())
}
