//! Channel types for all platforms

use serde::{Deserialize, Serialize};

use crate::{Thumbnail, Video};

/// Composite key for channel lookups (platform_name + channel_id)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelKey {
    pub platform_name: String,
    pub channel_id: String,
}

impl ChannelKey {
    pub fn new(platform_name: impl Into<String>, channel_id: impl Into<String>) -> Self {
        Self {
            platform_name: platform_name.into(),
            channel_id: channel_id.into(),
        }
    }
}

impl std::fmt::Display for ChannelKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.platform_name, self.channel_id)
    }
}

impl std::str::FromStr for ChannelKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (platform_name, channel_id) = s
            .split_once(':')
            .ok_or_else(|| format!("Invalid ChannelKey format: {}", s))?;
        Ok(Self {
            platform_name: platform_name.to_string(),
            channel_id: channel_id.to_string(),
        })
    }
}

/// Detailed channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub platform_name: String,
    pub id: String,
    pub name: String,
    pub handle: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub subscriber_count: Option<String>,
    pub video_count: Option<u64>,
    pub verified: Option<bool>,
    pub thumbnails: Vec<Thumbnail>,
    pub banner: Option<Vec<Thumbnail>>,
    /// Instance URL for federated platforms (e.g., PeerTube)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl ChannelInfo {
    /// Get the key for this channel
    pub fn key(&self) -> ChannelKey {
        ChannelKey::new(&self.platform_name, &self.id)
    }
}

/// Channel videos response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelVideos {
    pub videos: Vec<Video>,
    pub continuation: Option<String>,
    pub sort_filters: Option<Vec<SortFilter>>,
    /// Detected locale (hl, gl)
    pub detected_locale: Option<(String, String)>,
}

/// Sort filter information for channel videos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortFilter {
    pub label: String,
    pub is_selected: bool,
    pub continuation_token: Option<String>,
}

/// Channel tab types for browsing different content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ChannelTab {
    /// Videos tab
    #[default]
    Videos,
    /// Shorts tab
    Shorts,
    /// Live streams tab
    Streams,
}

/// Saved channel configuration (subscription and/or language override)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelConfig {
    pub platform_name: String,
    pub channel_id: String,
    pub channel_name: String,
    pub channel_handle: Option<String>,
    pub thumbnail_url: String,
    /// Instance URL for federated platforms (e.g., PeerTube)
    #[serde(default)]
    pub instance: Option<String>,
    /// Whether the user is subscribed to this channel
    #[serde(default)]
    pub subscribed: bool,
    /// Timestamp when subscribed (ISO 8601), None if never subscribed
    pub subscribed_at: Option<String>,
    /// Per-channel language override (hl, gl)
    #[serde(default)]
    pub language: Option<(String, String)>,
}

impl ChannelConfig {
    /// Get the key for this channel
    pub fn key(&self) -> ChannelKey {
        ChannelKey::new(&self.platform_name, &self.channel_id)
    }
}
