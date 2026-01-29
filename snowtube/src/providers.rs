//! Video provider management for multi-platform support.
//!
//! This module provides a platform-agnostic API for the client.
//! The client should never need to know about specific platforms.

use common::{
    ChannelConfig, ChannelInfo, ChannelTab, ChannelVideos, LanguageOption, NextPageToken,
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

        // Initialize platform icons now so views never block
        let _ = PLATFORM_ICONS.get_or_init(init_platform_icons);

        service
    })
}

// =============================================================================
// Search operations (fully platform-agnostic)
// =============================================================================
/// Search with locale
pub async fn search_with_locale(query: &str, hl: &str, gl: &str) -> Result<SearchResults, String> {
    service()
        .search_with_locale(query, hl, gl)
        .await
        .map_err(|e| e.to_string())
}

/// Continue search using next page tokens
pub async fn search_next_page(next_page_tokens: &[NextPageToken]) -> Result<SearchResults, String> {
    service()
        .search_next_page(next_page_tokens)
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

/// Fetch additional video metadata (full description, channel info)
pub async fn get_video_metadata(
    video: &Video,
    hl: &str,
    gl: &str,
) -> Result<common::VideoMetadata, String> {
    service()
        .get_video_metadata(video, hl, gl)
        .await
        .map_err(|e| e.to_string())
}

/// Fetch subtitles for a video
pub async fn get_subtitles(video: &Video) -> Result<Vec<common::Subtitle>, String> {
    service()
        .get_subtitles(video)
        .await
        .map_err(|e| e.to_string())
}

// =============================================================================
// Platform icons
// =============================================================================

use iced::widget::svg;
use iced::{Element, Theme};
use iced_font_awesome::fa_icon_brands;
use std::collections::HashMap;

/// PeerTube icon (Public Domain)
const PEERTUBE_ICON: &[u8] = include_bytes!("../assets/peertube.svg");

/// Fallback icon for unknown platforms
const FALLBACK_ICON: &[u8] = include_bytes!("../assets/fallback.svg");

/// Cached SVG handles for platform icons (initialized once at first access)
static PLATFORM_ICONS: OnceLock<HashMap<String, svg::Handle>> = OnceLock::new();

fn init_platform_icons() -> HashMap<String, svg::Handle> {
    let mut icons = HashMap::new();

    #[cfg(feature = "peertube")]
    icons.insert(
        "peertube".to_string(),
        svg::Handle::from_memory(PEERTUBE_ICON),
    );

    // Load fallback
    icons.insert(
        "_fallback".to_string(),
        svg::Handle::from_memory(FALLBACK_ICON),
    );

    icons
}

/// Get platform icon as an Element by platform name.
/// YouTube uses Font Awesome (CC BY 4.0), PeerTube uses SVG (Public Domain).
pub fn get_platform_icon<'a, M: 'a>(platform_name: &str) -> Element<'a, M, Theme> {
    // YouTube uses Font Awesome brands icon
    if platform_name == "youtube" {
        return fa_icon_brands("youtube").size(16.0).into();
    }

    // Other platforms use SVG
    let icons = PLATFORM_ICONS.get_or_init(init_platform_icons);
    let handle = icons
        .get(platform_name)
        .or_else(|| icons.get("_fallback"))
        .cloned()
        .expect("Fallback icon should always exist");

    svg::Svg::new(handle).width(16).height(16).into()
}
