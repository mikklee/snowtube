//! YouTube signature cipher and URL decryption module.
//!
//! This module handles the extraction and execution of YouTube's signature
//! decipher and n-transform functions, which are required to obtain working
//! video stream URLs.

mod extractor;
mod player;
mod runtime;
#[cfg(test)]
mod tests;

pub use player::PlayerContext;

use crate::{Error, Result};

/// Deciphers a YouTube video URL by applying signature and n-transform.
pub struct CipherContext {
    player: PlayerContext,
}

impl CipherContext {
    /// Create a new cipher context from a player JS URL.
    pub async fn new(player_url: &str) -> Result<Self> {
        let player = PlayerContext::fetch(player_url).await?;
        Ok(Self { player })
    }

    /// Decipher a signature parameter.
    pub fn decipher_signature(&mut self, sig: &str) -> Result<String> {
        self.player.run_decipher(sig)
    }

    /// Transform the n parameter (throttling bypass).
    pub fn transform_n(&mut self, n: &str) -> Result<String> {
        self.player.run_n_transform(n)
    }

    /// Decipher a full URL, applying both signature and n-transform.
    pub fn decipher_url(&mut self, url: &str) -> Result<String> {
        use url::Url;

        let mut parsed = Url::parse(url).map_err(|e| Error::Parse(e.to_string()))?;

        let mut new_params: Vec<(String, String)> = Vec::new();
        let mut sig_param = None;
        let mut sp_param = String::from("signature");
        let mut n_param = None;

        for (key, value) in parsed.query_pairs() {
            match key.as_ref() {
                "s" => sig_param = Some(value.to_string()),
                "sp" => sp_param = value.to_string(),
                "n" => n_param = Some(value.to_string()),
                _ => new_params.push((key.to_string(), value.to_string())),
            }
        }

        // Decipher signature if present
        if let Some(sig) = sig_param {
            let deciphered = self.decipher_signature(&sig)?;
            new_params.push((sp_param, deciphered));
        }

        // Transform n parameter if present
        if let Some(n) = n_param {
            let transformed = self.transform_n(&n)?;
            new_params.push(("n".to_string(), transformed));
        }

        parsed.query_pairs_mut().clear();
        for (key, value) in new_params {
            parsed.query_pairs_mut().append_pair(&key, &value);
        }

        Ok(parsed.to_string())
    }
}
