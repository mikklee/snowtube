//! Configuration persistence for ytrs-client

use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Serializable version of LanguageOption for config persistence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SerializableLanguageOption {
    pub hl: String,
    pub gl: String,
}

impl SerializableLanguageOption {
    /// Convert from ytrs::LanguageOption
    pub fn from_language_option(lang: &ytrs::LanguageOption) -> Self {
        Self {
            hl: lang.hl.to_string(),
            gl: lang.gl.to_string(),
        }
    }

    /// Find matching LanguageOption from ytrs
    pub fn to_language_option(&self) -> Option<ytrs::LanguageOption> {
        ytrs::get_all_languages()
            .iter()
            .find(|lang| lang.hl == self.hl && lang.gl == self.gl)
            .cloned()
    }
}

/// Application configuration data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    /// Default language for search results and channel videos
    pub default_language: Option<SerializableLanguageOption>,
}

/// Top-level configuration file with version for future migrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtrsConfig {
    pub version: Version,
    pub config: AppConfig,
}

impl Default for YtrsConfig {
    fn default() -> Self {
        Self {
            version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            config: AppConfig::default(),
        }
    }
}

impl YtrsConfig {
    /// Get the path to the config file
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("ytrs");
            path.push("config.json");
            path
        })
    }

    /// Load configuration from disk asynchronously
    /// If it does not exists, returns YtrsConfig::default()
    pub async fn load_if_exists() -> Result<Self, String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;

        // If config file doesn't exist, return default
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        // First, deserialize just to get the version
        let raw_ytrs_config: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;

        match raw_ytrs_config.get("config") {
            Some(c) => {
                /* Future migration logic
                let stored_version = raw_ytrs_config
                    .get("version")
                    .and_then(|v| v.as_str())
                    .and_then(|v| Version::parse(v).ok())
                    .ok_or("Invalid or missing version in config file")?;

                let current_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
                let migration_handled_config = if stored_version != current_version {
                    if stored_version > current_version {
                        // Dowgrade code here
                    } else {
                        // Upgrade code here
                    }
                } else {
                    serde_json::from_value(raw_ytrs_config)
                        .map_err(|e| format!("Failed to deserialize config"))?
                };
                */
                Ok(Self {
                    config: serde_json::from_value(c.clone())
                        .map_err(|e| format!("Failed to deserialize config {}", e))?,
                    ..Default::default()
                })
            }
            None =>
            // Config does not exist, so return default config
            {
                Ok(Self::default())
            }
        }
    }

    /// Save configuration to disk asynchronously
    pub async fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;

        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        // Serialize to JSON
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Write to file
        fs::write(&path, contents)
            .await
            .map_err(|e| format!("Failed to write config file: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = YtrsConfig::default();
        let expected_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(config.version, expected_version);
        assert_eq!(config.config.default_language, None);
    }

    #[test]
    fn test_serialization() {
        let config = YtrsConfig {
            version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            config: AppConfig {
                default_language: Some(SerializableLanguageOption {
                    hl: "ja".to_string(),
                    gl: "JP".to_string(),
                }),
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: YtrsConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.version, deserialized.version);
        assert_eq!(config.config, deserialized.config);
    }
}
