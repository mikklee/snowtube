//! Message and View types for the ytrs-client application

use crate::config::AppConfig;
use ytrs::{ChannelInfo, ChannelTab, ChannelVideos, SearchResults};

#[derive(Debug, Clone)]
pub enum View {
    Search,
    Channel,
    Config,
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
    BackToSearch,
    LanguageSelected(ytrs::LanguageOption),
    // Config-related messages
    OpenConfig,
    CloseConfig,
    ConfigLoaded(Result<AppConfig, String>),
    ConfigSaved(Result<(), String>),
    // Window events
    Resized(f32, f32), // width, height
}
