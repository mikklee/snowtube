//! Video source types for the video player.

/// Represents the source of a video to be played.
#[derive(Debug, Clone)]
pub enum VideoSource {
    /// YouTube video by ID (e.g., "dQw4w9WgXcQ")
    YouTube(String),
    /// Direct video URL (e.g., file:// or https://)
    DirectUrl(String),
    /// Live stream URL (HLS/DASH)
    Live(String),
}

impl VideoSource {
    /// Create a YouTube source from a video ID.
    pub fn youtube(id: impl Into<String>) -> Self {
        Self::YouTube(id.into())
    }

    /// Create a direct URL source.
    pub fn direct_url(url: impl Into<String>) -> Self {
        Self::DirectUrl(url.into())
    }

    /// Create a live stream source.
    pub fn live(url: impl Into<String>) -> Self {
        Self::Live(url.into())
    }
}
