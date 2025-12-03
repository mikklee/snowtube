//! YouTube player.js fetching and caching.

use crate::{Error, Result};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

use super::extractor::CipherFunctions;
use super::runtime::JsExecutor;

/// Cache for player contexts by player ID.
static PLAYER_CACHE: OnceLock<RwLock<HashMap<String, CachedPlayer>>> = OnceLock::new();

fn get_cache() -> &'static RwLock<HashMap<String, CachedPlayer>> {
    PLAYER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[derive(Clone)]
struct CachedPlayer {
    decipher_fn: String,
    n_transform_fn: String,
}

/// Context for a YouTube player, containing extracted cipher functions.
pub struct PlayerContext {
    player_id: String,
    executor: JsExecutor,
    decipher_fn_name: String,
    n_transform_fn_name: String,
}

impl PlayerContext {
    /// Fetch and parse a YouTube player.js file.
    pub async fn fetch(player_url: &str) -> Result<Self> {
        let player_id = extract_player_id(player_url)?;

        // Check cache first
        {
            let cache = get_cache().read().await;
            if let Some(cached) = cache.get(&player_id) {
                return Self::from_cached(&player_id, cached);
            }
        }

        // Fetch player.js
        let client = reqwest::Client::new();
        let player_js = client
            .get(player_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .send()
            .await
            .map_err(|e| Error::Request(e.to_string()))?
            .text()
            .await
            .map_err(|e| Error::Request(e.to_string()))?;

        // Extract cipher functions
        let functions = CipherFunctions::extract(&player_js)?;

        // Cache the extracted functions
        {
            let mut cache = get_cache().write().await;
            cache.insert(
                player_id.clone(),
                CachedPlayer {
                    decipher_fn: functions.decipher_fn.clone(),
                    n_transform_fn: functions.n_transform_fn.clone(),
                },
            );
        }

        Self::from_functions(&player_id, functions)
    }

    fn from_cached(player_id: &str, cached: &CachedPlayer) -> Result<Self> {
        let functions = CipherFunctions {
            decipher_fn: cached.decipher_fn.clone(),
            n_transform_fn: cached.n_transform_fn.clone(),
        };
        Self::from_functions(player_id, functions)
    }

    fn from_functions(player_id: &str, functions: CipherFunctions) -> Result<Self> {
        let mut executor = JsExecutor::new()?;

        // Extract the actual function names from the code
        let decipher_fn_name = extract_function_name(&functions.decipher_fn)
            .unwrap_or("decipher")
            .to_string();
        let n_transform_fn_name = extract_function_name(&functions.n_transform_fn)
            .unwrap_or("ntransform")
            .to_string();

        // Load the cipher functions into the JS runtime
        // The extracted code defines the functions, we just execute it
        executor.execute_void(&functions.decipher_fn)?;
        executor.execute_void(&functions.n_transform_fn)?;

        Ok(Self {
            player_id: player_id.to_string(),
            executor,
            decipher_fn_name,
            n_transform_fn_name,
        })
    }

    /// Run the decipher function on a signature.
    pub fn run_decipher(&mut self, sig: &str) -> Result<String> {
        let code = format!("{}('{}')", self.decipher_fn_name, escape_js_string(sig));
        self.executor.execute(&code)
    }

    /// Run the n-transform function.
    pub fn run_n_transform(&mut self, n: &str) -> Result<String> {
        let code = format!("{}('{}')", self.n_transform_fn_name, escape_js_string(n));
        self.executor.execute(&code)
    }

    /// Get the player ID.
    pub fn player_id(&self) -> &str {
        &self.player_id
    }
}

/// Extract player ID from a player URL.
/// e.g., "https://www.youtube.com/s/player/abcd1234/player_ias.vflset/en_US/base.js" -> "abcd1234"
fn extract_player_id(url: &str) -> Result<String> {
    let re = regex::Regex::new(r"/player/([a-zA-Z0-9_-]+)/").unwrap();
    re.captures(url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| Error::Parse("Failed to extract player ID from URL".to_string()))
}

/// Extract the function name from a function definition.
/// Looks for patterns like:
/// - `function wpa(a){...}`
/// - `iha=function(a){...}`
fn extract_function_name(code: &str) -> Option<&str> {
    // First try: function name(...)
    let fn_re = regex::Regex::new(r"function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(").ok()?;
    if let Some(caps) = fn_re.captures(code) {
        return caps.get(1).map(|m| m.as_str());
    }

    // Second try: name=function(...)
    let assign_re = regex::Regex::new(r"([a-zA-Z_$][a-zA-Z0-9_$]*)\s*=\s*function\s*\(").ok()?;
    if let Some(caps) = assign_re.captures(code) {
        return caps.get(1).map(|m| m.as_str());
    }

    None
}

/// Escape a string for use in JavaScript.
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
