//! Configuration persistence for ytrs-client

use semver::Version;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::fs;

use crate::theme::AppTheme;

// Re-export AudioVisualizer from iceplayer
pub use iceplayer::AudioVisualizer;

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
    /// Convert from common::LanguageOption
    pub fn from_language_option(lang: &common::LanguageOption) -> Self {
        Self {
            hl: lang.hl.to_string(),
            gl: lang.gl.to_string(),
        }
    }

    /// Find matching LanguageOption from common
    pub fn to_language_option(&self) -> Option<common::LanguageOption> {
        common::get_all_languages()
            .iter()
            .find(|lang| lang.hl == self.hl && lang.gl == self.gl)
            .cloned()
    }
}

/// Subscription format from v0.1.x
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChannelSubscriptionV01 {
    channel_id: String,
    channel_name: String,
    channel_handle: Option<String>,
    thumbnail_url: String,
    subscribed_at: String,
}

/// Config format from v0.1.x
#[derive(Debug, Clone, Deserialize, Default)]
struct AppConfigV01 {
    #[serde(default)]
    default_language: Option<SerializableLanguageOption>,
    #[serde(default)]
    theme: AppTheme,
    #[serde(default)]
    subscriptions: Vec<ChannelSubscriptionV01>,
}

impl From<AppConfigV01> for AppConfig {
    fn from(old: AppConfigV01) -> Self {
        Self {
            default_language: old.default_language,
            theme: old.theme,
            show_scrollbar: true,
            audio_visualizer: AudioVisualizer::default(),
            channels: old
                .subscriptions
                .into_iter()
                .map(|sub| {
                    let config = common::ChannelConfig {
                        platform_name: "youtube".to_string(), // v0.1 only had YouTube
                        platform_icon: common::PlatformIcon {
                            name: "youtube".to_string(),
                            icon_type: common::IconType::Brand,
                        },
                        channel_id: sub.channel_id,
                        channel_name: sub.channel_name,
                        channel_handle: sub.channel_handle,
                        thumbnail_url: sub.thumbnail_url,
                        instance: None,
                        subscribed: true,
                        subscribed_at: Some(sub.subscribed_at),
                        language: None,
                    };
                    (config.key(), config)
                })
                .collect(),
        }
    }
}

/// Cached video data for a single channel in the subscription view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedChannelVideos {
    pub videos: Vec<common::Video>,
    pub fetched_at: i64, // unix timestamp
}

/// Custom serialization for HashMap<ChannelKey, CachedChannelVideos> using string keys
mod cache_map_serde {
    use super::CachedChannelVideos;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(
        map: &HashMap<common::ChannelKey, CachedChannelVideos>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: HashMap<String, &CachedChannelVideos> =
            map.iter().map(|(k, v)| (k.to_string(), v)).collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<common::ChannelKey, CachedChannelVideos>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: HashMap<String, CachedChannelVideos> = HashMap::deserialize(deserializer)?;
        string_map
            .into_iter()
            .map(|(k, v)| {
                k.parse::<common::ChannelKey>()
                    .map(|key| (key, v))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

/// Cache for subscription videos
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubscriptionVideoCache {
    #[serde(default, with = "cache_map_serde")]
    pub channels: std::collections::HashMap<common::ChannelKey, CachedChannelVideos>,
}

impl SubscriptionVideoCache {
    /// Get the path to the cache file
    fn cache_path() -> Option<PathBuf> {
        dirs::cache_dir().map(|mut path| {
            path.push("ytrs");
            path.push("subscription_videos.json");
            path
        })
    }

    /// Load cache from disk asynchronously
    pub async fn load() -> Result<Self, ConfigError> {
        let path = Self::cache_path().context(NoConfigDirectorySnafu)?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&path).await.context(ReadConfigSnafu)?;
        let cache: Self = serde_json::from_str(&contents).context(ParseConfigSnafu)?;
        Ok(cache)
    }

    /// Save cache to disk asynchronously
    pub async fn save(&self) -> Result<(), ConfigError> {
        let path = Self::cache_path().context(NoConfigDirectorySnafu)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .context(CreateConfigDirectorySnafu)?;
        }

        let contents = serde_json::to_string_pretty(self).context(SerializeConfigSnafu)?;
        fs::write(&path, contents).await.context(WriteConfigSnafu)?;

        Ok(())
    }

    /// Check if a channel's videos are stale (older than 10 hours)
    pub fn is_stale(&self, key: &common::ChannelKey) -> bool {
        const TEN_HOURS_SECS: i64 = 10 * 60 * 60;

        match self.channels.get(key) {
            Some(cached) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                now - cached.fetched_at > TEN_HOURS_SECS
            }
            None => true,
        }
    }
}

/// Custom serialization for HashMap<ChannelKey, ChannelConfig> using string keys
mod channel_map_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(
        map: &HashMap<common::ChannelKey, common::ChannelConfig>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: HashMap<String, &common::ChannelConfig> =
            map.iter().map(|(k, v)| (k.to_string(), v)).collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<common::ChannelKey, common::ChannelConfig>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: HashMap<String, common::ChannelConfig> =
            HashMap::deserialize(deserializer)?;
        string_map
            .into_iter()
            .map(|(k, v)| {
                k.parse::<common::ChannelKey>()
                    .map(|key| (key, v))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
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
    /// Whether to show scrollbars in scrollable views
    #[serde(default = "default_show_scrollbar")]
    pub show_scrollbar: bool,
    /// Audio visualizer style for audio-only playback
    #[serde(default)]
    pub audio_visualizer: AudioVisualizer,
    /// Saved channel configurations (subscriptions and/or language overrides)
    #[serde(default, with = "channel_map_serde")]
    pub channels: std::collections::HashMap<common::ChannelKey, common::ChannelConfig>,
}

fn default_show_scrollbar() -> bool {
    true
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

        let Some(config_value) = raw_ytrs_config.get(field_name!(YtrsConfig, config)) else {
            return Ok(Self::default());
        };

        let stored_version = raw_ytrs_config
            .get(field_name!(YtrsConfig, version))
            .and_then(|v| v.as_str())
            .and_then(|v| Version::parse(v).ok())
            .unwrap_or_else(|| Version::new(0, 1, 0));

        let needs_migration = stored_version < *current_version();

        // Create backup before migration
        if needs_migration {
            let backup_path = path.with_extension(format!("v{}.json.bak", stored_version));
            if let Err(e) = fs::copy(&path, &backup_path).await {
                eprintln!("Warning: failed to create config backup: {}", e);
            } else {
                eprintln!("Created config backup: {:?}", backup_path);
            }
        }

        let config = if stored_version < Version::new(0, 3, 0) {
            // Migrate from v0.1.x/v0.2.x format with "subscriptions" to new "channels"
            let old_config: AppConfigV01 =
                serde_json::from_value(config_value.clone()).context(DeserializeConfigSnafu)?;
            AppConfig::from(old_config)
        } else {
            serde_json::from_value(config_value.clone()).context(DeserializeConfigSnafu)?
        };

        Ok(Self {
            config,
            ..Default::default()
        })
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
    use common::ChannelConfig;

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
        let channel = ChannelConfig {
            platform_name: "youtube".to_string(),
            platform_icon: common::PlatformIcon {
                name: "youtube".to_string(),
                icon_type: common::IconType::Brand,
            },
            channel_id: "test_id".to_string(),
            channel_name: String::new(),
            channel_handle: None,
            thumbnail_url: String::new(),
            instance: None,
            subscribed: true,
            subscribed_at: Some(String::new()),
            language: None,
        };
        let config = YtrsConfig {
            version: current_version(),
            config: AppConfig {
                default_language: Some(SerializableLanguageOption {
                    hl: "ja".to_string(),
                    gl: "JP".to_string(),
                }),
                theme: AppTheme::TokyoNight,
                channels: [(channel.key(), channel)].into_iter().collect(),
                show_scrollbar: true,
                audio_visualizer: AudioVisualizer::LedSpectrum,
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

    #[test]
    fn test_migrate_v02_to_v03() {
        // Test migrating from v0.2.x config with "subscriptions" to v0.2.x with "channels"
        let v01_config_json = r#"{
            "version": "0.2.0",
            "config": {
                "default_language": {
                    "hl": "ja",
                    "gl": "JP"
                },
                "theme": "TokyoNight",
                "subscriptions": [
                    {
                        "channelId": "UC123",
                        "channelName": "Test Channel",
                        "channelHandle": "@testchannel",
                        "thumbnailUrl": "https://example.com/thumb.jpg",
                        "subscribedAt": "2024-01-15T12:00:00Z"
                    },
                    {
                        "channelId": "UC456",
                        "channelName": "Another Channel",
                        "channelHandle": null,
                        "thumbnailUrl": "https://example.com/thumb2.jpg",
                        "subscribedAt": "2024-02-20T15:30:00Z"
                    }
                ]
            }
        }"#;

        let raw_config: serde_json::Value =
            serde_json::from_str(v01_config_json).expect("Failed to parse v0.1 config JSON");

        let config_value = raw_config.get("config").expect("Expected config field");

        // Parse as old config and convert
        let old_config: AppConfigV01 =
            serde_json::from_value(config_value.clone()).expect("Failed to parse as AppConfigV01");
        let migrated: AppConfig = AppConfig::from(old_config);

        // Verify migration
        assert_eq!(migrated.default_language.as_ref().unwrap().hl, "ja");
        assert_eq!(migrated.theme, AppTheme::TokyoNight);
        assert_eq!(migrated.channels.len(), 2);

        // Check first channel
        let key1 = common::ChannelKey::new("youtube", "UC123");
        let ch1 = migrated
            .channels
            .get(&key1)
            .expect("Channel UC123 should exist");
        assert_eq!(ch1.channel_id, "UC123");
        assert_eq!(ch1.channel_name, "Test Channel");
        assert_eq!(ch1.channel_handle, Some("@testchannel".to_string()));
        assert_eq!(ch1.thumbnail_url, "https://example.com/thumb.jpg");
        assert!(ch1.subscribed);
        assert_eq!(ch1.subscribed_at, Some("2024-01-15T12:00:00Z".to_string()));
        assert!(ch1.language.is_none());

        // Check second channel
        let key2 = common::ChannelKey::new("youtube", "UC456");
        let ch2 = migrated
            .channels
            .get(&key2)
            .expect("Channel UC456 should exist");
        assert_eq!(ch2.channel_id, "UC456");
        assert_eq!(ch2.channel_name, "Another Channel");
        assert!(ch2.channel_handle.is_none());
        assert!(ch2.subscribed);
        assert!(ch2.language.is_none());
    }
}
