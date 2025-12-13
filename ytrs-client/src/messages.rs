//! Message and View types for the ytrs-client application

use crate::config::AppConfig;
use crate::theme::AppTheme;
use iceplayer::{PlayerEvent, VideoPlayerMessage};
use ytrs_lib::{ChannelInfo, ChannelTab, ChannelVideos, SearchResults};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Search,
    Channel,
    Config,
    Channels,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabId {
    Search,
    Channels,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    Search,
    SearchDone(Result<SearchResults, String>),
    ThumbLoaded(String, Result<Vec<u8>, String>),
    BannerLoaded(Result<Vec<u8>, String>),
    ViewChannel(String), // channel_id
    ChannelLoaded(Result<ChannelInfo, String>),
    ChannelVideosLoaded(Result<ChannelVideos, String>),
    ChangeChannelTab(ChannelTab),
    ChangeSortFilter(String), // sort filter label
    LoadMoreVideos,
    LoadMoreSearchResults,
    BackToChannels,
    LanguageSelected(ytrs_lib::LanguageOption),
    // Config-related messages
    ConfigLoaded(Result<AppConfig, String>),
    ConfigSaved(Result<(), String>),
    ThemeChanged(AppTheme),
    ShowScrollbarToggled(bool),
    // Window events
    Resized(f32, f32), // width, height
    // Subscription-related messages
    SubscribeToChannel,
    UnsubscribeFromChannel(String), // channel_id
    SubscriptionChannelThumbLoaded(String, Result<Vec<u8>, String>), // channel_id, thumb_data
    SubscriptionVideosLoaded(String, String, Result<ChannelVideos, String>), // channel_id, channel_name, videos
    SubscriptionVideosCacheLoaded(Result<crate::config::SubscriptionVideoCache, String>),
    RefreshSubscriptionVideos,
    // No-op message for non-interactive elements
    NoOp,
    // Tab selection
    TabSelected(TabId),
    // Export search results
    ExportSearchResults,
    // Video player messages (new high-level API)
    PlayVideo(String, Option<String>, Option<String>), // video_id, channel_name, channel_id
    PlayAudioOnly(String, Option<String>, Option<String>), // video_id, channel_name, channel_id (audio-only)
    VideoPlayer(VideoPlayerMessage),                       // Internal player messages
    VideoEvent(PlayerEvent),                               // High-level events from player
    VideoThumbnailLoaded(Result<Vec<u8>, String>),         // High-res thumbnail for player
    BackFromVideo,                                         // Navigate back from video view
    LaunchInMpv(String),                                   // Launch video in mpv (video_id)
    CopyVideoUrl(String),                                  // Copy video URL to clipboard
    SeekTo(f64),                                           // Seek to position (0.0 to 1.0)
    SeekRelative(i64), // Seek relative seconds (positive = forward)
    ExitFullscreen,    // Exit fullscreen (only if in fullscreen)
}
