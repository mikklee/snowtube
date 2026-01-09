//! Common traits and types for video platform providers.
//!
//! This crate provides a unified interface for video platforms like YouTube and PeerTube.

mod channel;
mod language;
mod relative_time;
mod service;
mod time;
mod video;

pub use channel::*;
pub use language::*;
pub use relative_time::parse_relative_time;
pub use service::*;
pub use time::*;
pub use video::*;

use async_trait::async_trait;

/// Error type for video providers
#[derive(Debug, snafu::Snafu)]
pub enum ProviderError {
    #[snafu(display("Network error: {message}"))]
    Network { message: String },
    #[snafu(display("API error: {message}"))]
    Api { message: String },
    #[snafu(display("Not found: {message}"))]
    NotFound { message: String },
    #[snafu(display("Parse error: {message}"))]
    Parse { message: String },
}

/// Trait for video platform providers
#[async_trait]
pub trait VideoProvider: Send + Sync {
    /// Get the platform name (e.g., "youtube", "peertube")
    fn platform_name(&self) -> &'static str;

    /// Get the platform icon for UI display
    fn platform_icon(&self) -> PlatformIcon;

    /// Search for videos with default locale
    async fn search(&self, query: &str) -> Result<SearchResults, ProviderError> {
        self.search_with_locale(query, "en", "US").await
    }

    /// Search for videos with specific locale
    async fn search_with_locale(
        &self,
        query: &str,
        hl: &str,
        gl: &str,
    ) -> Result<SearchResults, ProviderError>;

    /// Get more search results using continuation token
    async fn search_continuation(
        &self,
        token: &str,
        hl: &str,
        gl: &str,
    ) -> Result<SearchResults, ProviderError>;

    /// Get video details by ID
    async fn get_video(&self, id: &str) -> Result<Video, ProviderError>;

    /// Fetch thumbnail image bytes from URL
    async fn fetch_thumbnail(&self, url: &str) -> Result<Vec<u8>, ProviderError>;

    /// Fetch high-quality thumbnail for a video by ID (platform-specific)
    async fn fetch_hq_thumbnail(&self, video_id: &str) -> Result<Vec<u8>, ProviderError>;
}

/// Trait for providers that support channel browsing
#[async_trait]
pub trait ChannelProvider: VideoProvider {
    /// Get channel information
    async fn get_channel(&self, config: &ChannelConfig) -> Result<ChannelInfo, ProviderError>;

    /// Get videos from a channel with default locale
    async fn get_channel_videos(
        &self,
        config: &ChannelConfig,
        tab: ChannelTab,
    ) -> Result<ChannelVideos, ProviderError> {
        self.get_channel_videos_with_locale(config, tab, "en", "US")
            .await
    }

    /// Get videos from a channel with specific locale
    async fn get_channel_videos_with_locale(
        &self,
        config: &ChannelConfig,
        tab: ChannelTab,
        hl: &str,
        gl: &str,
    ) -> Result<ChannelVideos, ProviderError>;

    /// Get more videos using continuation token with default locale
    async fn get_channel_videos_continuation(
        &self,
        config: &ChannelConfig,
        token: &str,
    ) -> Result<ChannelVideos, ProviderError> {
        self.get_channel_videos_continuation_with_locale(config, token, "en", "US")
            .await
    }

    /// Get more videos using continuation token with specific locale
    async fn get_channel_videos_continuation_with_locale(
        &self,
        config: &ChannelConfig,
        token: &str,
        hl: &str,
        gl: &str,
    ) -> Result<ChannelVideos, ProviderError>;
}
