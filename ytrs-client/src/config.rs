//! Configuration persistence for ytrs-client

use semver::Version;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::fs;

use crate::theme::AppTheme;

/// Macro to extract field names from any struct at compile time
/// Usage: field_name!(YtrsConfig, version) returns "version"
/// The macro ensures at compile time that the field actually exists
macro_rules! field_name {
    ($struct:ty, $field:ident) => {{
        let _ = |x: &$struct| {
            let _ = &x.$field;
        }; // Compile-time check that field exists
        stringify!($field)
    }};
}

/// Cached current version parsed from CARGO_PKG_VERSION
static CURRENT_VERSION: OnceLock<Version> = OnceLock::new();

fn current_version() -> &'static Version {
    CURRENT_VERSION.get_or_init(|| {
        Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("CARGO_PKG_VERSION should always be a valid semver")
    })
}

#[derive(Debug, Snafu)]
pub enum ConfigError {
    #[snafu(display("Could not determine config directory"))]
    NoConfigDirectory,

    #[snafu(display("Failed to read config file: {source}"))]
    ReadConfig { source: std::io::Error },

    #[snafu(display("Failed to parse config file: {source}"))]
    ParseConfig { source: serde_json::Error },

    #[snafu(display("Invalid or missing version in config file"))]
    InvalidVersion,

    #[snafu(display("Failed to deserialize config: {source}"))]
    DeserializeConfig { source: serde_json::Error },

    #[snafu(display("Failed to create config directory: {source}"))]
    CreateConfigDirectory { source: std::io::Error },

    #[snafu(display("Failed to serialize config: {source}"))]
    SerializeConfig { source: serde_json::Error },

    #[snafu(display("Failed to write config file: {source}"))]
    WriteConfig { source: std::io::Error },
}

/// Serializable version of LanguageOption for config persistence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SerializableLanguageOption {
    pub hl: String,
    pub gl: String,
}

impl SerializableLanguageOption {
    /// Convert from ytrs_lib::LanguageOption
    pub fn from_language_option(lang: &ytrs_lib::LanguageOption) -> Self {
        Self {
            hl: lang.hl.to_string(),
            gl: lang.gl.to_string(),
        }
    }

    /// Find matching LanguageOption from ytrs_lib
    pub fn to_language_option(&self) -> Option<ytrs_lib::LanguageOption> {
        ytrs_lib::get_all_languages()
            .iter()
            .find(|lang| lang.hl == self.hl && lang.gl == self.gl)
            .cloned()
    }
}

/// Application configuration data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    /// Default language for search results and channel videos
    #[serde(default)]
    pub default_language: Option<SerializableLanguageOption>,
    /// Selected theme
    #[serde(default)]
    pub theme: AppTheme,
}

/// Top-level configuration file with version for future migrations
#[derive(Debug, Clone, Serialize)]
pub struct YtrsConfig {
    pub version: &'static Version,
    pub config: AppConfig,
}

impl Default for YtrsConfig {
    fn default() -> Self {
        Self {
            version: current_version(),
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
    pub async fn load_if_exists() -> Result<Self, ConfigError> {
        let path = Self::config_path().context(NoConfigDirectorySnafu)?;

        // If config file doesn't exist, return default
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path).await.context(ReadConfigSnafu)?;

        // First, deserialize just to get the version
        let raw_ytrs_config: serde_json::Value =
            serde_json::from_str(&contents).context(ParseConfigSnafu)?;

        match raw_ytrs_config.get(field_name!(YtrsConfig, config)) {
            Some(c) => {
                /* Future migration logic
                let stored_version = raw_ytrs_config
                    .get(field_name!(YtrsConfig, version))
                    .and_then(|v| v.as_str())
                    .and_then(|v| Version::parse(v).ok())
                    .ok_or("Invalid or missing version in config file")?;

                let migration_handled_config = if stored_version != current_version() {
                    if stored_version > current_version() {
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
                    config: serde_json::from_value(c.clone()).context(DeserializeConfigSnafu)?,
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
    pub async fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path().context(NoConfigDirectorySnafu)?;

        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .context(CreateConfigDirectorySnafu)?;
        }

        // Serialize to JSON
        let contents = serde_json::to_string_pretty(self).context(SerializeConfigSnafu)?;

        // Write to file
        fs::write(&path, contents).await.context(WriteConfigSnafu)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = YtrsConfig::default();
        assert_eq!(config.version, current_version());
        assert_eq!(config.config.default_language, None);
        assert_eq!(config.config.theme, AppTheme::Cyberpunk);
    }

    #[test]
    fn test_serialization() {
        let config = YtrsConfig {
            version: current_version(),
            config: AppConfig {
                default_language: Some(SerializableLanguageOption {
                    hl: "ja".to_string(),
                    gl: "JP".to_string(),
                }),
                theme: AppTheme::TokyoNight,
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let raw_ytrs_config: serde_json::Value = serde_json::from_str(&json)
            .expect("Expected to deserialize config to serde_json::Value");

        let version: semver::Version = serde_json::from_value(
            raw_ytrs_config
                .get(field_name!(YtrsConfig, version))
                .expect("Expected serialized config to have a version field")
                .clone(),
        )
        .expect("Expected version to be a semver::Version");
        assert_eq!(config.version, &version);

        let deserialized: AppConfig = serde_json::from_value(
            raw_ytrs_config
                .get(field_name!(YtrsConfig, config))
                .expect("Expected serialized config to have a config field")
                .clone(),
        )
        .expect("Expected to deserialize config");

        assert_eq!(config.config, deserialized);
    }

    #[test]
    fn test_deserialize_config_without_theme() {
        // Test deserializing an old config file that doesn't have the theme field
        // This simulates loading a config from before the theme feature was added
        let old_config_json = r#"{
            "version": "0.1.0",
            "config": {
                "default_language": {
                    "hl": "en",
                    "gl": "US"
                }
            }
        }"#;

        let raw_ytrs_config: serde_json::Value =
            serde_json::from_str(old_config_json).expect("Failed to parse old config JSON");

        let deserialized: AppConfig = serde_json::from_value(
            raw_ytrs_config
                .get(field_name!(YtrsConfig, config))
                .expect("Expected config field")
                .clone(),
        )
        .expect("Failed to deserialize old config");

        // Should use default theme when not specified
        assert_eq!(deserialized.theme, AppTheme::Cyberpunk);
        assert!(deserialized.default_language.is_some());
        assert_eq!(deserialized.default_language.unwrap().hl, "en");
    }
}
