//! Message and View types for the ytrs-client application

use crate::config::AppConfig;
use crate::theme::AppTheme;
use ytrs_lib::{ChannelInfo, ChannelTab, ChannelVideos, SearchResults};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Search,
    Channel,
    Config,
    Channels,
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
    Play(String),
    CountdownTick(String), // video_id for the countdown
    ViewChannel(String),   // channel_id
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
    // No-op message for non-interactive elements
    NoOp,
    // Tab selection
    TabSelected(TabId),
    // Export search results
    ExportSearchResults,
}
