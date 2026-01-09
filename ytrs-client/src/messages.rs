//! Message and View types for the ytrs-client application

use crate::config::AppConfig;
use crate::theme::AppTheme;
use common::{ChannelInfo, ChannelTab, ChannelVideos, LanguageOption};
use iceplayer::AudioVisualizer;
use iceplayer::{PlayerEvent, VideoPlayerMessage};

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
    // Unified search results
    SearchDone(Result<common::SearchResults, String>),
    ThumbLoaded(String, Result<Vec<u8>, String>),
    BannerLoaded(Result<Vec<u8>, String>),
    ViewChannel(common::ChannelConfig), // channel config (platform + id)
    ChannelLoaded(Result<ChannelInfo, String>),
    ChannelVideosLoaded(Result<ChannelVideos, String>),
    ChangeChannelTab(ChannelTab),
    ChangeSortFilter(String), // sort filter label
    LoadMoreVideos,
    LoadMoreSearchResults,
    BackToChannels,
    LanguageSelected(LanguageOption),
    // Config-related messages
    ConfigLoaded(Result<AppConfig, String>),
    ConfigSaved(Result<(), String>),
    ThemeChanged(AppTheme),
    ShowScrollbarToggled(bool),
    AudioVisualizerChanged(AudioVisualizer),
    // Window events
    Resized(f32, f32), // width, height
    // Subscription-related messages
    SubscribeToChannel,
    UnsubscribeFromChannel(common::ChannelKey), // channel key (platform + id)
    SubscriptionChannelThumbLoaded(String, Result<Vec<u8>, String>), // channel_id, thumb_data
    SubscriptionVideosLoaded(common::ChannelKey, Result<ChannelVideos, String>), // channel_key, videos
    SubscriptionVideosCacheLoaded(Result<crate::config::SubscriptionVideoCache, String>),
    RefreshSubscriptionVideos,
    // No-op message for non-interactive elements
    NoOp,
    // Tab selection
    TabSelected(TabId),
    // Export search results
    ExportSearchResults,
    // Video player messages - use Video for platform info
    PlayVideo(Box<common::Video>),                 // video to play
    PlayAudioOnly(Box<common::Video>),             // video to play (audio-only mode)
    VideoPlayer(VideoPlayerMessage),               // Internal player messages
    VideoEvent(PlayerEvent),                       // High-level events from player
    VideoThumbnailLoaded(Result<Vec<u8>, String>), // High-res thumbnail for player
    BackFromVideo,                                 // Navigate back from video view
    LaunchInMpv(String),                           // Launch video in mpv (video_id)
    CopyVideoUrl(String),                          // Copy video URL to clipboard
    SeekTo(f64),                                   // Seek to position (0.0 to 1.0)
    SeekRelative(i64),                             // Seek relative seconds (positive = forward)
    ExitFullscreen,                                // Exit fullscreen (only if in fullscreen)

    // Error notifications
    ShowError(String),          // Display an error notification
    DismissNotification(usize), // Dismiss a specific notification by ID
    ClearNotifications,         // Clear all notifications
    NotificationTick,           // Timer tick to auto-dismiss old notifications
}

/// A notification to display to the user
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: usize,
    pub message: String,
    pub created_at: std::time::Instant,
    pub level: NotificationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Error,
    Warning,
    Info,
}
