//! Message and View types for the ytrs-client application

use crate::config::AppConfig;
use crate::theme::AppTheme;
use std::sync::Arc;
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
    // Video player messages
    PlayVideo(String), // video_id - play with iced_video_player
    VideoLoaded(Result<Arc<iced_video_player::Video>, String>), // Video loaded from yt-dlp (Arc for Clone)
    VideoEnded,
    TogglePlayPause,
    ToggleFullscreen,
    BackFromVideo,
    VideoError(String),
    VideoMouseMoved,      // Mouse moved over video - show controls
    VideoControlsTimeout, // Timer fired - hide controls if no recent activity
    SeekVideo(f64),       // Seek to position (0.0 to 1.0 percentage)
    VideoTick,            // Periodic tick to update progress bar
}
