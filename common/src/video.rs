//! Video types for all platforms

use serde::{Deserialize, Serialize};

/// Unified video metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    /// Platform identifier (e.g., "youtube", "peertube")
    pub platform_name: String,
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// Duration in seconds
    pub duration: Option<u64>,
    /// Duration formatted as "HH:MM:SS" or "MM:SS"
    pub duration_string: Option<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    /// Relative time like "2 days ago"
    pub published_text: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub channel: Option<Channel>,
    /// Public watch URL for this video
    pub watch_url: String,
    /// Instance URL for federated platforms (e.g., PeerTube)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// Whether this is a premium/paid video
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_premium: Option<bool>,
    /// Whether this is a short-form video
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_short: Option<bool>,
    /// Video badges (e.g., "4K", "CC")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badges: Option<Vec<String>>,
}

/// Channel/creator information (basic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Option<String>,
    pub name: String,
    pub url: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub verified: Option<bool>,
}

/// Thumbnail image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Continuation token with platform information for multi-platform search
#[derive(Debug, Clone)]
pub struct ContinuationToken {
    pub platform_name: String,
    pub token: String,
}

/// Search results container
#[derive(Debug, Clone)]
pub struct SearchResults {
    pub results: Vec<Video>,
    /// Continuation tokens for each platform that has more results
    pub continuations: Vec<ContinuationToken>,
    /// Detected locale from YouTube (hl, gl)
    pub detected_locale: Option<(String, String)>,
}

impl Video {
    /// Get the best thumbnail URL
    pub fn thumbnail_url(&self) -> Option<&str> {
        self.thumbnails.last().map(|t| t.url.as_str())
    }

    /// Get channel name if available
    pub fn channel_name(&self) -> Option<&str> {
        self.channel.as_ref().map(|c| c.name.as_str())
    }

    /// Get channel ID if available
    pub fn channel_id(&self) -> Option<&str> {
        self.channel.as_ref().and_then(|c| c.id.as_deref())
    }
}
