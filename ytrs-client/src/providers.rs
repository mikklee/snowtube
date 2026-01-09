//! Video provider management for multi-platform support.
//!
//! This module provides a platform-agnostic API for the client.
//! The client should never need to know about specific platforms.

use common::{
    ChannelConfig, ChannelInfo, ChannelTab, ChannelVideos, ContinuationToken, LanguageOption,
    SearchResults, Video, VideoService,
};
use std::sync::OnceLock;

/// Global video service instance - initialized once, reused everywhere
static VIDEO_SERVICE: OnceLock<VideoService> = OnceLock::new();

/// Get or initialize the global video service
pub fn service() -> &'static VideoService {
    VIDEO_SERVICE.get_or_init(|| {
        let mut service = VideoService::new();

        #[cfg(feature = "youtube")]
        {
            // Use Default impl for sync initialization
            let client = ytrs_lib::InnerTube::default();
            service = service.with_channel_provider(client);
            tracing::info!("YouTube provider initialized");
        }

        #[cfg(feature = "peertube")]
        {
            let client = ptrs_lib::PeerTubeClient::default();
            service = service.with_channel_provider(client);
            tracing::info!("PeerTube provider initialized");
        }

        service
    })
}

// =============================================================================
// Search operations (fully platform-agnostic)
// =============================================================================

/// Search all enabled providers
pub async fn search(query: &str) -> Result<SearchResults, String> {
    service().search(query).await.map_err(|e| e.to_string())
}

/// Search all enabled providers (alias for search)
pub async fn search_all(query: &str) -> Result<SearchResults, String> {
    search(query).await
}

/// Search with locale
pub async fn search_with_locale(query: &str, hl: &str, gl: &str) -> Result<SearchResults, String> {
    service()
        .search_with_locale(query, hl, gl)
        .await
        .map_err(|e| e.to_string())
}

/// Continue search using continuation tokens
pub async fn search_continuation(
    continuations: &[ContinuationToken],
    hl: &str,
    gl: &str,
) -> Result<SearchResults, String> {
    service()
        .search_continuation(continuations, hl, gl)
        .await
        .map_err(|e| e.to_string())
}

// =============================================================================
// Channel operations (use ChannelConfig for platform info)
// =============================================================================

/// Get channel info
pub async fn get_channel(config: &ChannelConfig) -> Result<ChannelInfo, String> {
    service()
        .get_channel(config)
        .await
        .map_err(|e| e.to_string())
}

/// Get channel videos
pub async fn get_channel_videos(
    config: &ChannelConfig,
    tab: ChannelTab,
) -> Result<ChannelVideos, String> {
    service()
        .get_channel_videos(config, tab)
        .await
        .map_err(|e| e.to_string())
}

/// Get channel videos with locale override
pub async fn get_channel_videos_with_locale(
    config: &ChannelConfig,
    tab: ChannelTab,
    hl: &str,
    gl: &str,
) -> Result<ChannelVideos, String> {
    service()
        .get_channel_videos_with_locale(config, tab, hl, gl)
        .await
        .map_err(|e| e.to_string())
}

/// Get more channel videos using continuation (for a specific channel config)
pub async fn get_channel_videos_continuation(
    config: &ChannelConfig,
    token: &str,
    hl: &str,
    gl: &str,
) -> Result<ChannelVideos, String> {
    service()
        .get_channel_videos_continuation(config, token, hl, gl)
        .await
        .map_err(|e| e.to_string())
}

/// Get language option by locale codes
pub fn get_language_by_locale(hl: &str, gl: &str) -> Option<&'static LanguageOption> {
    common::get_language_by_locale(hl, gl)
}

// =============================================================================
// Video/thumbnail operations (use Video for platform info)
// =============================================================================

/// Fetch thumbnail for a video
pub async fn fetch_thumbnail_for_video(video: &Video) -> Result<Vec<u8>, String> {
    service()
        .fetch_thumbnail_for_video(video)
        .await
        .map_err(|e| e.to_string())
}
