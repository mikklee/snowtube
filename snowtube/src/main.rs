mod config;
mod helpers;
mod messages;
mod providers;
mod theme;
mod views;
mod widgets;

use iceplayer::{PlayerEvent, VideoPlayerMessage, VideoPlayerState, VideoSource};

use iced::widget::combo_box;
use iced::{Element, Size, Subscription, Task, Theme, event};
use std::cell::Cell;
use std::collections::HashMap;

use common::{
    ChannelConfig, ChannelInfo, ChannelTab, LanguageOption, SortFilter, Video, get_all_languages,
};
use std::sync::OnceLock;

use config::{AppConfig, SerializableLanguageOption, YtrsConfig};
use messages::{Message, TabId, View};
use theme::AppTheme;

/// Cached HashMap for O(1) language lookups by (hl, gl) tuple
static LOCALE_TO_LANGUAGE: OnceLock<HashMap<(String, String), &'static LanguageOption>> =
    OnceLock::new();

fn get_language_by_locale(hl: &str, gl: &str) -> Option<&'static LanguageOption> {
    let map = LOCALE_TO_LANGUAGE.get_or_init(|| {
        get_all_languages()
            .iter()
            .map(|lang| ((lang.hl.to_string(), lang.gl.to_string()), lang))
            .collect()
    });
    map.get(&(hl.to_string(), gl.to_string())).copied()
}

/// Helper to create a task that saves the config to disk
fn save_config(config: AppConfig) -> Task<Message> {
    Task::perform(
        async move {
            let new_config = YtrsConfig {
                config,
                ..Default::default()
            };
            new_config.save().await.map_err(|e| e.to_string())
        },
        Message::ConfigSaved,
    )
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    iced::application(App::new, App::update, App::view)
        .title(cosmic_title)
        .theme(app_theme)
        .subscription(App::subscription)
        .font(include_bytes!("../fonts/Inter-Regular.ttf"))
        .font(include_bytes!("../fonts/MPLUSRounded1c-Regular.ttf"))
        .default_font(iced::Font {
            family: iced::font::Family::Name("Rounded Mplus 1c"),
            ..iced::Font::DEFAULT
        })
        .run()
}

fn cosmic_title(_: &App) -> String {
    "SnowTube".to_string()
}

fn app_theme(app: &App) -> Theme {
    app.current_theme.clone()
}

pub struct App {
    // Shared state
    pub query: String,
    pub video_thumbs: HashMap<String, iced::widget::image::Handle>, // watch_url -> thumbnail
    pub channel_avatars: HashMap<common::ChannelKey, iced::widget::image::Handle>,
    pub subscription_videos: HashMap<common::ChannelKey, Vec<Video>>, // channel_key -> videos
    pub subscription_videos_cache: config::SubscriptionVideoCache,    // Persistent cache
    pub subscription_videos_loading: std::collections::HashSet<common::ChannelKey>, // Channels currently being fetched
    pub current_view: View,
    pub previous_view: View, // Track which view to return to from config
    pub active_tab: TabId,   // Current active tab in TabBar
    pub last_view_for_timing: Cell<Option<View>>, // Track last view to detect tab switches
    pub language_combo_state: combo_box::State<LanguageOption>,
    pub selected_language: Option<LanguageOption>, // User's manual language override (global)

    pub config: AppConfig,    // Persistent configuration
    pub window_width: f32,    // Current window width for responsive layout
    pub window_height: f32,   // Current window height for responsive layout
    pub current_theme: Theme, // Current theme
    pub pending_avatar_updates: Vec<(common::ChannelKey, Vec<u8>)>, // Batched avatar updates
    pub last_thumb_update: Option<std::time::Instant>, // Last time we processed thumb updates

    // Search-specific state
    pub search_results: Vec<common::Video>,
    pub search_next_page_tokens: Vec<common::NextPageToken>,
    pub search_preload_count: usize,
    pub search_locale: (String, String),
    pub searching: bool,
    pub search_loading_more: bool,
    pub search_preloading: bool,

    // Channel-specific state
    pub channel_results: Vec<Video>,
    pub channel_continuation: Option<String>,
    pub channel_preload_count: usize,
    pub channel_locale: (String, String),
    pub current_channel: Option<ChannelInfo>,
    pub current_tab: ChannelTab,
    pub banner: Option<iced::widget::image::Handle>,
    pub loading_channel: bool,
    pub channel_loading_more: bool,
    pub channel_preloading: bool,
    pub available_sort_filters: Vec<SortFilter>,
    pub selected_sort_label: Option<String>,

    // Video player state (using new high-level API)
    pub video_player: Option<iceplayer::VideoPlayerState>,
    pub playing_video_id: Option<String>, // Current video ID (for actions like mpv, copy URL)
    pub playing_video_info: Option<Video>, // Full video info for display

    // Notifications
    pub notifications: Vec<messages::Notification>,
    pub notification_counter: usize, // For generating unique IDs
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                // Shared state
                query: String::new(),
                video_thumbs: HashMap::new(),
                channel_avatars: HashMap::new(),
                subscription_videos: HashMap::new(),
                subscription_videos_cache: config::SubscriptionVideoCache::default(),
                subscription_videos_loading: std::collections::HashSet::new(),
                current_view: View::Search,
                previous_view: View::Search,
                active_tab: TabId::Search,
                last_view_for_timing: Cell::new(None),
                language_combo_state: combo_box::State::new(get_all_languages().to_vec()),
                selected_language: None,

                config: AppConfig::default(),
                window_width: 800.0,
                window_height: 600.0,
                current_theme: AppTheme::default().to_iced_theme(),
                pending_avatar_updates: Vec::new(),
                last_thumb_update: None,

                // Search-specific state
                search_results: Vec::new(),
                search_next_page_tokens: Vec::new(),
                search_preload_count: 0,
                search_locale: ("en".to_string(), "GB".to_string()),
                searching: false,
                search_loading_more: false,
                search_preloading: false,

                // Channel-specific state
                channel_results: Vec::new(),
                channel_continuation: None,
                channel_preload_count: 0,
                channel_locale: ("en".to_string(), "GB".to_string()),
                current_channel: None,
                current_tab: ChannelTab::Videos,
                banner: None,
                loading_channel: false,
                channel_loading_more: false,
                channel_preloading: false,
                available_sort_filters: Vec::new(),
                selected_sort_label: None,

                // Video player state (using new high-level API)
                video_player: None,
                playing_video_id: None,
                playing_video_info: None,

                // Notifications
                notifications: Vec::new(),
                notification_counter: 0,
            },
            // Load config asynchronously on startup
            Task::perform(
                async {
                    YtrsConfig::load_if_exists()
                        .await
                        .map(|config_file| config_file.config)
                        .map_err(|e| e.to_string())
                },
                Message::ConfigLoaded,
            ),
        )
    }

    /// Show an error notification and log it
    fn show_error(&mut self, message: impl Into<String>) {
        let msg = message.into();
        tracing::error!("{}", msg);

        // Ignore duplicate messages already in queue
        if self.notifications.iter().any(|n| n.message == msg) {
            return;
        }

        self.notification_counter += 1;
        self.notifications.push(messages::Notification {
            id: self.notification_counter,
            message: msg,
            created_at: std::time::Instant::now(),
            level: messages::NotificationLevel::Error,
        });
    }

    /// Fetch videos for subscribed channels that are stale (>10h old or not cached)
    fn fetch_stale_subscription_videos(&mut self) -> Task<Message> {
        // Collect channels to fetch first to avoid borrow issues
        let channels_to_fetch: Vec<_> = self
            .config
            .channels
            .values()
            .filter(|c| c.subscribed)
            .filter(|c| {
                let key = c.key();
                !self.subscription_videos_loading.contains(&key)
                    && self.subscription_videos_cache.is_stale(&key)
            })
            .map(|c| {
                let mut config = c.clone();
                // Apply default language if channel doesn't have one
                if config.language.is_none() {
                    config.language = self
                        .config
                        .default_language
                        .as_ref()
                        .map(|l| (l.hl.clone(), l.gl.clone()));
                }
                config
            })
            .collect();

        // Mark channels as loading
        for config in &channels_to_fetch {
            self.subscription_videos_loading.insert(config.key());
        }

        let tasks: Vec<Task<Message>> = channels_to_fetch
            .into_iter()
            .map(|config| {
                let key = config.key();
                Task::perform(
                    async move { providers::get_channel_videos(&config, ChannelTab::Videos).await },
                    move |res| Message::SubscriptionVideosLoaded(key, res),
                )
            })
            .collect();

        Task::batch(tasks)
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::InputChanged(v) => {
                self.query = v;
                Task::none()
            }
            Message::Search => {
                if self.query.is_empty() || self.searching {
                    return Task::none();
                }
                self.searching = true;
                self.search_results.clear();
                self.search_next_page_tokens.clear();
                self.search_preload_count = 0;
                self.search_preloading = true;
                let q = self.query.clone();
                let (hl, gl) = self.search_locale.clone();

                // Search all enabled providers in parallel
                Task::perform(
                    async move { providers::search_with_locale(&q, &hl, &gl).await },
                    Message::SearchDone,
                )
            }
            Message::SearchDone(res) => {
                match res {
                    Ok(search_results) => {
                        // Check if this is a "load more" operation (appending results)
                        let is_load_more = self.search_loading_more;
                        self.search_loading_more = false;

                        // Store the new results to load thumbnails for
                        let new_results = search_results.results;

                        // Store continuation tokens for pagination
                        self.search_next_page_tokens = search_results.next_page_tokens;

                        // Store detected locale only if no manual language is selected
                        if self.selected_language.is_none()
                            && let Some(locale) = search_results.detected_locale
                        {
                            self.search_locale = locale;
                        }

                        // Update results (replace on first search, append on continuation/preload)
                        if !is_load_more && self.search_results.is_empty() {
                            self.search_results = new_results.clone();
                            // Don't clear thumbs - they're cached to disk and shared across views
                        } else {
                            // Appending results (load more or preloading)
                            self.search_results.extend(new_results.clone());
                        }

                        // Auto-preload: fetch more pages until we have enough displayable results
                        // (after filtering out shorts and premium videos)
                        const MIN_DISPLAYABLE_RESULTS: usize = 60;

                        // Count displayable results (not shorts, not premium)
                        let displayable_count = self
                            .search_results
                            .iter()
                            .filter(|r| r.is_premium != Some(true) && r.is_short != Some(true))
                            .count();

                        if self.search_preloading {
                            self.search_preload_count += 1;

                            // Keep fetching if we don't have enough displayable results and have next page tokens
                            if displayable_count < MIN_DISPLAYABLE_RESULTS
                                && !self.search_next_page_tokens.is_empty()
                            {
                                let next_page_tokens = self.search_next_page_tokens.clone();
                                let (hl, gl) = self.search_locale.clone();

                                // Start loading thumbnails for current batch while fetching next page
                                let thumb_tasks = helpers::create_thumbnail_tasks(&new_results);

                                // Fetch next page with stored locale
                                let next_page_task = Task::perform(
                                    async move {
                                        providers::search_next_page(&next_page_tokens, &hl, &gl)
                                            .await
                                    },
                                    Message::SearchDone,
                                );

                                return Task::batch([Task::batch(thumb_tasks), next_page_task]);
                            } else {
                                // Preloading complete
                                self.search_preloading = false;
                                self.searching = false;
                                self.search_loading_more = false;
                            }
                        }

                        // Load thumbnails ONLY for the new results (not all results)
                        Task::batch(helpers::create_thumbnail_tasks(&new_results))
                    }
                    Err(e) => {
                        self.show_error(format!("Search failed: {}", e));
                        self.search_preloading = false;
                        self.searching = false;
                        self.search_loading_more = false;
                        Task::none()
                    }
                }
            }
            Message::VideoThumbLoaded(watch_url, res) => {
                if let Ok(bytes) = res {
                    self.video_thumbs
                        .insert(watch_url, iced::widget::image::Handle::from_bytes(bytes));
                }
                Task::none()
            }
            Message::ChannelAvatarLoaded(key, res) => {
                if let Ok(bytes) = res {
                    // Batch avatar updates instead of updating immediately
                    self.pending_avatar_updates.push((key, bytes));

                    let now = std::time::Instant::now();
                    let should_flush = match self.last_thumb_update {
                        None => true,
                        Some(last) => {
                            // Flush if we have 10+ pending or 100ms has passed
                            self.pending_avatar_updates.len() >= 10
                                || now.duration_since(last).as_millis() >= 100
                        }
                    };

                    if should_flush {
                        // Process all pending updates at once
                        for (avatar_key, avatar_bytes) in self.pending_avatar_updates.drain(..) {
                            self.channel_avatars.insert(
                                avatar_key,
                                iced::widget::image::Handle::from_bytes(avatar_bytes),
                            );
                        }
                        self.last_thumb_update = Some(now);
                    }
                }
                Task::none()
            }
            Message::BannerLoaded(res) => {
                if let Ok(bytes) = res {
                    self.banner = Some(iced::widget::image::Handle::from_bytes(bytes));
                }
                Task::none()
            }
            Message::ViewChannel(channel_config) => {
                self.loading_channel = true;
                self.current_view = View::Channel;
                self.active_tab = TabId::Channels;
                self.banner = None;
                // Don't clear search state! Only initialize channel state
                self.channel_results.clear();
                self.current_tab = ChannelTab::Videos;
                self.available_sort_filters.clear();
                self.selected_sort_label = None;
                self.channel_continuation = None;
                self.channel_preload_count = 0;
                self.channel_preloading = true;

                // Determine channel language:
                // 1. Use per-channel saved language if set (from config or passed config)
                // 2. Otherwise use global default from config
                // 3. Otherwise auto-detect
                let channel_language = channel_config.language.clone().or_else(|| {
                    self.config
                        .channels
                        .get(&channel_config.key())
                        .and_then(|c| c.language.clone())
                });

                if let Some((hl, gl)) = channel_language {
                    // This channel has a specific language set
                    self.channel_locale = (hl.clone(), gl.clone());
                    self.selected_language = providers::get_language_by_locale(&hl, &gl).cloned();
                } else if let Some(ref lang_config) = self.config.default_language {
                    // Use global default language
                    self.channel_locale = (lang_config.hl.clone(), lang_config.gl.clone());
                    self.selected_language = lang_config.to_language_option();
                } else {
                    // No language set - will auto-detect
                    self.selected_language = None;
                }

                // First load channel info, then use channel name for locale detection when loading videos
                let config = channel_config.clone();
                Task::perform(
                    async move { providers::get_channel(&config).await },
                    Message::ChannelLoaded,
                )
            }
            Message::ChannelLoaded(res) => {
                self.loading_channel = false;
                match res {
                    Ok(channel) => {
                        // Load banner image if available
                        let banner_task = if let Some(banner_images) = &channel.banner {
                            if let Some(banner) = banner_images.last() {
                                let url = banner.url.clone();
                                Task::perform(
                                    async move {
                                        helpers::load_thumb(&url).await.map_err(|e| e.to_string())
                                    },
                                    Message::BannerLoaded,
                                )
                            } else {
                                Task::none()
                            }
                        } else {
                            Task::none()
                        };

                        // Load channel avatar (circular) if not already cached
                        let channel_key =
                            common::ChannelKey::new(&channel.platform_name, &channel.id);
                        let avatar_task = if self.channel_avatars.contains_key(&channel_key) {
                            Task::none()
                        } else if let Some(thumb) = channel.thumbnails.first() {
                            let url = thumb.url.clone();
                            Task::perform(
                                async move {
                                    helpers::load_circular_thumb(&url, 80)
                                        .await
                                        .map_err(|e| e.to_string())
                                },
                                move |r| Message::ChannelAvatarLoaded(channel_key.clone(), r),
                            )
                        } else {
                            Task::none()
                        };

                        // Load channel videos - use manual language if selected, otherwise auto-detect
                        let tab = self.current_tab;

                        // Create ChannelConfig from ChannelInfo
                        let channel_config = ChannelConfig {
                            platform_name: channel.platform_name.clone(),
                            channel_id: channel.id.clone(),
                            channel_name: channel.name.clone(),
                            channel_handle: channel.handle.clone(),
                            thumbnail_url: channel
                                .thumbnails
                                .first()
                                .map(|t| t.url.clone())
                                .unwrap_or_default(),
                            instance: channel.instance.clone(),
                            subscribed: false,
                            subscribed_at: None,
                            language: self
                                .selected_language
                                .as_ref()
                                .map(|l| (l.hl.to_string(), l.gl.to_string())),
                        };
                        // Store or update config for this channel
                        let key = channel.key();
                        if let Some(existing) = self.config.channels.get_mut(&key) {
                            // Update language if set
                            if channel_config.language.is_some() {
                                existing.language = channel_config.language.clone();
                            }
                        } else {
                            self.config.channels.insert(key, channel_config.clone());
                        }
                        let videos_task = Task::perform(
                            async move { providers::get_channel_videos(&channel_config, tab).await },
                            Message::ChannelVideosLoaded,
                        );

                        self.current_channel = Some(channel);
                        Task::batch(vec![banner_task, avatar_task, videos_task])
                    }
                    Err(e) => {
                        self.show_error(format!("Failed to load channel: {}", e));
                        self.loading_channel = false;
                        Task::none()
                    }
                }
            }
            Message::ChannelVideosLoaded(res) => {
                match res {
                    Ok(videos) => {
                        // Check if this is a "load more" operation (appending videos)
                        let is_load_more = self.channel_loading_more;
                        self.channel_loading_more = false;

                        // Store the new videos to load thumbnails for
                        let new_videos = videos.videos;

                        // Update videos (append if load more, replace otherwise)
                        if is_load_more || self.channel_preloading {
                            self.channel_results.extend(new_videos.clone());
                        } else {
                            self.channel_results = new_videos.clone();
                        }

                        // Store continuation token for pagination
                        self.channel_continuation = videos.continuation;

                        // Store detected locale only if no manual language is selected
                        if self.selected_language.is_none()
                            && let Some(locale) = videos.detected_locale
                        {
                            self.channel_locale = locale;
                        }

                        // Update sort filters if available
                        if let Some(filters) = videos.sort_filters {
                            self.selected_sort_label = filters
                                .iter()
                                .find(|f| f.is_selected)
                                .map(|f| f.label.clone());
                            self.available_sort_filters = filters;
                        }

                        // Auto-preload: fetch pages until we have enough playable (non-premium) videos
                        const MAX_PRELOAD_PAGES: usize = 10; // Maximum pages to fetch
                        const MIN_PLAYABLE_VIDEOS: usize = 30;

                        if self.channel_preloading {
                            self.channel_preload_count += 1;

                            // Count non-premium videos
                            let playable_count = self
                                .channel_results
                                .iter()
                                .filter(|r| r.is_premium != Some(true))
                                .count();

                            // Keep loading if we have a continuation AND either:
                            // - We don't have enough playable videos yet (primary goal)
                            // - We haven't reached the absolute maximum page limit (safety limit)
                            let should_continue = self.channel_continuation.is_some()
                                && playable_count < MIN_PLAYABLE_VIDEOS
                                && self.channel_preload_count < MAX_PRELOAD_PAGES;

                            if should_continue {
                                let token = self.channel_continuation.as_ref().unwrap().clone();
                                let (hl, gl) = self.channel_locale.clone();

                                // Look up config from current_channel
                                let config = self
                                    .current_channel
                                    .as_ref()
                                    .and_then(|ch| self.config.channels.get(&ch.key()).cloned());

                                // Start loading thumbnails for current batch while fetching next page
                                let thumb_tasks = helpers::create_thumbnail_tasks(&new_videos);

                                // Fetch next page with stored locale
                                let next_page_task = if let Some(cfg) = config {
                                    Task::perform(
                                        async move {
                                            providers::get_channel_videos_continuation(
                                                &cfg, &token, &hl, &gl,
                                            )
                                            .await
                                        },
                                        Message::ChannelVideosLoaded,
                                    )
                                } else {
                                    Task::none()
                                };

                                return Task::batch([Task::batch(thumb_tasks), next_page_task]);
                            } else {
                                // Preloading complete
                                self.channel_preloading = false;
                                self.loading_channel = false;
                                self.channel_loading_more = false;
                            }
                        }

                        // Load thumbnails ONLY for the new videos (not all videos)
                        Task::batch(helpers::create_thumbnail_tasks(&new_videos))
                    }
                    Err(e) => {
                        self.show_error(format!("Failed to load channel videos: {}", e));
                        self.channel_preloading = false;
                        self.loading_channel = false;
                        self.channel_loading_more = false;
                        Task::none()
                    }
                }
            }
            Message::ChangeChannelTab(tab) => {
                if let Some(ref channel) = self.current_channel {
                    self.current_tab = tab;
                    self.channel_results.clear();
                    self.available_sort_filters.clear();
                    self.selected_sort_label = None;
                    self.channel_continuation = None;
                    self.channel_preload_count = 0;
                    self.channel_preloading = true;
                    self.loading_channel = true;

                    // Look up config from channels HashMap using channel's key
                    let config = self.config.channels.get(&channel.key()).cloned();

                    if let Some(mut cfg) = config {
                        // Apply locale override if set
                        let (hl, gl) = self.channel_locale.clone();
                        cfg.language = Some((hl.clone(), gl.clone()));

                        Task::perform(
                            async move {
                                providers::get_channel_videos_with_locale(&cfg, tab, &hl, &gl).await
                            },
                            Message::ChannelVideosLoaded,
                        )
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            Message::ChangeSortFilter(label) => {
                // Find the filter with the matching label and use its continuation token
                if let Some(filter) = self
                    .available_sort_filters
                    .iter()
                    .find(|f| f.label == label)
                    && let Some(ref token) = filter.continuation_token
                    && let Some(ref channel) = self.current_channel
                {
                    self.selected_sort_label = Some(label);
                    self.channel_results.clear();
                    self.channel_continuation = None; // Will be updated with new continuation
                    self.channel_preload_count = 0;
                    self.channel_preloading = true;
                    self.loading_channel = true;

                    // Look up config from channels HashMap
                    let config = self.config.channels.get(&channel.key()).cloned();

                    if let Some(cfg) = config {
                        let token = token.clone();
                        let (hl, gl) = self.channel_locale.clone();
                        return Task::perform(
                            async move {
                                providers::get_channel_videos_continuation(&cfg, &token, &hl, &gl)
                                    .await
                            },
                            Message::ChannelVideosLoaded,
                        );
                    }
                }
                Task::none()
            }

            Message::LoadMoreVideos => {
                if let Some(ref token) = self.channel_continuation
                    && !self.channel_loading_more
                    && let Some(ref channel) = self.current_channel
                {
                    self.channel_loading_more = true;
                    // Enable preloading to fetch 3 more pages
                    self.channel_preload_count = 0;
                    self.channel_preloading = true;

                    // Look up config from channels HashMap
                    let config = self.config.channels.get(&channel.key()).cloned();

                    if let Some(cfg) = config {
                        let token = token.clone();
                        // Use stored locale for consistent results
                        let (hl, gl) = self.channel_locale.clone();
                        return Task::perform(
                            async move {
                                providers::get_channel_videos_continuation(&cfg, &token, &hl, &gl)
                                    .await
                            },
                            Message::ChannelVideosLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::LoadMoreSearchResults => {
                if !self.search_next_page_tokens.is_empty() && !self.search_loading_more {
                    self.search_loading_more = true;
                    // Enable preloading to fetch 3 more pages
                    self.search_preload_count = 0;
                    self.search_preloading = true;

                    let next_page_tokens = self.search_next_page_tokens.clone();
                    // Use stored locale for consistent results
                    let (hl, gl) = self.search_locale.clone();
                    return Task::perform(
                        async move { providers::search_next_page(&next_page_tokens, &hl, &gl).await },
                        Message::SearchDone,
                    );
                }
                Task::none()
            }
            Message::BackToChannels => {
                self.current_view = View::Channels;
                self.active_tab = TabId::Channels;
                // Clear only channel state, preserve search state!
                self.current_channel = None;
                self.channel_results.clear();
                self.banner = None;
                self.available_sort_filters.clear();
                self.selected_sort_label = None;
                self.channel_continuation = None;
                self.channel_preload_count = 0;
                self.channel_preloading = false;
                self.loading_channel = false;
                self.channel_loading_more = false;

                // Load avatars for any newly subscribed channels
                let tasks: Vec<Task<Message>> = self
                    .config
                    .channels
                    .values()
                    .filter(|c| c.subscribed)
                    .filter(|c| !self.channel_avatars.contains_key(&c.key()))
                    .map(|c| {
                        let channel_key = c.key();
                        let url = c.thumbnail_url.clone();
                        Task::perform(
                            async move {
                                helpers::load_circular_thumb(&url, 80)
                                    .await
                                    .map_err(|e| e.to_string())
                            },
                            move |res| Message::ChannelAvatarLoaded(channel_key.clone(), res),
                        )
                    })
                    .collect();

                Task::batch(tasks)
            }
            Message::LanguageSelected(language) => {
                self.selected_language = Some(language.clone());
                let hl = language.hl.to_string();
                let gl = language.gl.to_string();

                // Update both locales to the manually selected language
                self.search_locale = (hl.clone(), gl.clone());
                self.channel_locale = (hl.clone(), gl.clone());

                match self.current_view {
                    View::Channel => {
                        // Re-fetch channel with this locale
                        if let Some(ref channel) = self.current_channel {
                            // Save language preference for this channel
                            let language_tuple = (hl.clone(), gl.clone());
                            let key = channel.key();

                            if let Some(channel_config) = self.config.channels.get_mut(&key) {
                                // Update existing channel config
                                channel_config.language = Some(language_tuple);
                            } else {
                                // Create new channel config with just language
                                let thumbnail_url = channel
                                    .thumbnails
                                    .last()
                                    .map(|t| t.url.clone())
                                    .unwrap_or_default();

                                let new_config = ChannelConfig {
                                    platform_name: channel.platform_name.clone(),
                                    channel_id: channel.id.clone(),
                                    channel_name: channel.name.clone(),
                                    channel_handle: channel.handle.clone(),
                                    thumbnail_url,
                                    instance: channel.instance.clone(),
                                    subscribed: false,
                                    subscribed_at: None,
                                    language: Some((hl.clone(), gl.clone())),
                                };
                                self.config.channels.insert(key.clone(), new_config);
                            }

                            let save_task = save_config(self.config.clone());

                            self.channel_results.clear();
                            self.channel_continuation = None;
                            self.channel_preload_count = 0;
                            self.channel_preloading = true;
                            self.loading_channel = true;

                            // Fetch channel info using the config
                            let channel_config = self.config.channels.get(&key).cloned();
                            let fetch_task = if let Some(config) = channel_config {
                                Task::perform(
                                    async move { providers::get_channel(&config).await },
                                    Message::ChannelLoaded,
                                )
                            } else {
                                Task::none()
                            };

                            Task::batch([save_task, fetch_task])
                        } else {
                            Task::none()
                        }
                    }
                    View::Channels => {
                        // No action needed for channels view
                        Task::none()
                    }
                    View::Search => {
                        // Re-run search with new locale if there's an active query
                        if !self.query.is_empty() && !self.searching {
                            self.searching = true;
                            self.search_results.clear();
                            self.search_next_page_tokens.clear();
                            self.search_preload_count = 0;
                            self.search_preloading = true;
                            let q = self.query.clone();

                            Task::perform(
                                async move { providers::search_with_locale(&q, &hl, &gl).await },
                                Message::SearchDone,
                            )
                        } else {
                            Task::none()
                        }
                    }
                    View::Config => {
                        // Update config and save
                        if let Some(ref lang) = self.selected_language {
                            self.config.default_language =
                                Some(SerializableLanguageOption::from_language_option(lang));
                        } else {
                            self.config.default_language = None;
                        }

                        save_config(self.config.clone())
                    }
                    View::Video => {
                        // No action needed for video view
                        Task::none()
                    }
                }
            }
            Message::ConfigLoaded(result) => {
                match result {
                    Ok(config) => {
                        self.config = config;

                        // Apply theme from config
                        self.current_theme = self.config.theme.to_iced_theme();

                        // Apply default language if set
                        if let Some(ref lang_config) = self.config.default_language
                            && let Some(lang) = lang_config.to_language_option()
                        {
                            self.selected_language = Some(lang.clone());
                            self.search_locale = (lang.hl.to_string(), lang.gl.to_string());
                            self.channel_locale = (lang.hl.to_string(), lang.gl.to_string());
                        }
                    }
                    Err(e) => {
                        self.show_error(format!("Failed to load config: {}", e));
                    }
                }
                Task::none()
            }
            Message::ConfigSaved(result) => {
                if let Err(e) = result {
                    self.show_error(format!("Failed to save config: {}", e));
                }
                Task::none()
            }
            Message::ThemeChanged(new_theme) => {
                self.current_theme = new_theme.to_iced_theme();
                self.config.theme = new_theme;
                save_config(self.config.clone())
            }
            Message::ShowScrollbarToggled(show) => {
                self.config.show_scrollbar = show;
                save_config(self.config.clone())
            }
            Message::AudioVisualizerChanged(visualizer) => {
                self.config.audio_visualizer = visualizer;
                save_config(self.config.clone())
            }
            Message::Resized(width, height) => {
                self.window_width = width;
                self.window_height = height;
                Task::none()
            }
            Message::SubscribeToChannel => {
                if let Some(ref channel) = self.current_channel {
                    let key = channel.key();

                    if let Some(channel_config) = self.config.channels.get_mut(&key) {
                        // Channel config exists, just mark as subscribed
                        if !channel_config.subscribed {
                            channel_config.subscribed = true;
                            channel_config.subscribed_at = Some(chrono::Utc::now().to_rfc3339());
                        }
                    } else {
                        // Get the best quality thumbnail
                        let thumbnail_url = channel
                            .thumbnails
                            .last()
                            .map(|t| t.url.clone())
                            .unwrap_or_default();

                        // Create new channel config
                        let channel_config = ChannelConfig {
                            platform_name: channel.platform_name.clone(),
                            channel_id: channel.id.clone(),
                            channel_name: channel.name.clone(),
                            channel_handle: channel.handle.clone(),
                            thumbnail_url,
                            instance: channel.instance.clone(),
                            subscribed: true,
                            subscribed_at: Some(chrono::Utc::now().to_rfc3339()),
                            language: None,
                        };

                        // Add to config
                        self.config.channels.insert(key, channel_config);
                    }

                    return save_config(self.config.clone());
                }
                Task::none()
            }
            Message::UnsubscribeFromChannel(key) => {
                if let Some(channel_config) = self.config.channels.get_mut(&key) {
                    channel_config.subscribed = false;
                    channel_config.subscribed_at = None;

                    // If no language override, remove the entry entirely
                    if channel_config.language.is_none() {
                        self.config.channels.remove(&key);
                    }
                }

                save_config(self.config.clone())
            }

            Message::TabSelected(tab_id) => {
                self.active_tab = tab_id;

                // If leaving video view, clean up video player
                if self.current_view == View::Video {
                    // Abort any in-progress loading
                    if let Some(ref player) = self.video_player
                        && let Some(ref handle) = player.loading_handle
                    {
                        handle.abort();
                    }
                    self.video_player = None;
                    self.playing_video_id = None;
                }

                match tab_id {
                    TabId::Search => {
                        self.current_view = View::Search;
                        Task::none()
                    }
                    TabId::Channels => {
                        self.current_view = View::Channels;
                        // Load circular thumbnails for all subscribed channels that aren't already loaded
                        let avatar_tasks: Vec<Task<Message>> = self
                            .config
                            .channels
                            .values()
                            .filter(|c| c.subscribed)
                            .filter(|c| !self.channel_avatars.contains_key(&c.key()))
                            .map(|c| {
                                let channel_key = c.key();
                                let url = c.thumbnail_url.clone();
                                Task::perform(
                                    async move {
                                        helpers::load_circular_thumb(&url, 80)
                                            .await
                                            .map_err(|e| e.to_string())
                                    },
                                    move |res| {
                                        Message::ChannelAvatarLoaded(channel_key.clone(), res)
                                    },
                                )
                            })
                            .collect();

                        // Load subscription video cache from disk if not already loaded
                        let cache_task = if self.subscription_videos_cache.channels.is_empty() {
                            Task::perform(
                                async { config::SubscriptionVideoCache::load().await },
                                |res| {
                                    Message::SubscriptionVideosCacheLoaded(
                                        res.map_err(|e| e.to_string()),
                                    )
                                },
                            )
                        } else {
                            // Cache already loaded, fetch stale videos
                            self.fetch_stale_subscription_videos()
                        };

                        Task::batch(avatar_tasks).chain(cache_task)
                    }
                    TabId::Settings => {
                        self.current_view = View::Config;
                        Task::none()
                    }
                }
            }
            Message::NoOp => Task::none(),
            Message::SubscriptionVideosCacheLoaded(result) => {
                match result {
                    Ok(cache) => {
                        // Populate subscription_videos from cache and collect videos for thumbnail loading
                        let mut all_videos: Vec<Video> = Vec::new();
                        for (key, cached) in &cache.channels {
                            // Look up channel name from config
                            let channel_config = self.config.channels.get(key);
                            let channel_name = channel_config.map(|c| c.channel_name.clone());
                            // Populate channel info on videos if missing
                            let videos: Vec<Video> = cached
                                .videos
                                .iter()
                                .cloned()
                                .map(|mut v| {
                                    if v.channel.is_none()
                                        && let Some(ref name) = channel_name
                                    {
                                        v.channel = Some(common::Channel {
                                            id: Some(key.channel_id.clone()),
                                            name: name.clone(),
                                            url: None,
                                            thumbnails: vec![],
                                            verified: None,
                                        });
                                    }
                                    v
                                })
                                .collect();
                            all_videos.extend(videos.clone());
                            self.subscription_videos.insert(key.clone(), videos);
                        }
                        self.subscription_videos_cache = cache;

                        // Load thumbnails for cached videos
                        let thumb_tasks = helpers::create_thumbnail_tasks(&all_videos);

                        // Now fetch stale videos
                        let fetch_task = self.fetch_stale_subscription_videos();
                        Task::batch(thumb_tasks).chain(fetch_task)
                    }
                    Err(e) => {
                        self.show_error(format!("Failed to load subscription cache: {}", e));
                        self.fetch_stale_subscription_videos()
                    }
                }
            }
            Message::SubscriptionVideosLoaded(key, result) => {
                self.subscription_videos_loading.remove(&key);
                // Look up channel name from config
                let channel_name = self
                    .config
                    .channels
                    .get(&key)
                    .map(|c| c.channel_name.clone())
                    .unwrap_or_default();

                match result {
                    Ok(channel_videos) => {
                        // Store full first page of videos with channel info populated
                        let videos: Vec<Video> = channel_videos
                            .videos
                            .into_iter()
                            .map(|mut v| {
                                // Populate channel info if not present
                                if v.channel.is_none() {
                                    v.channel = Some(common::Channel {
                                        id: Some(key.channel_id.clone()),
                                        name: channel_name.clone(),
                                        url: None,
                                        thumbnails: vec![],
                                        verified: None,
                                    });
                                }
                                v
                            })
                            .collect();

                        // Load thumbnails for these videos
                        let thumb_tasks = helpers::create_thumbnail_tasks(&videos);

                        // Update cache
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;
                        self.subscription_videos_cache.channels.insert(
                            key.clone(),
                            config::CachedChannelVideos {
                                videos: videos.clone(),
                                fetched_at: now,
                            },
                        );

                        self.subscription_videos.insert(key, videos);

                        // Save cache to disk only when all channels are done loading
                        let save_task = if self.subscription_videos_loading.is_empty() {
                            let cache = self.subscription_videos_cache.clone();
                            Task::perform(async move { cache.save().await }, |_| Message::NoOp)
                        } else {
                            Task::none()
                        };

                        Task::batch(thumb_tasks).chain(save_task)
                    }
                    Err(e) => {
                        self.show_error(format!(
                            "Failed to load videos for {}: {}",
                            channel_name, e
                        ));
                        // Save cache when all done, even if some failed
                        if self.subscription_videos_loading.is_empty() {
                            let cache = self.subscription_videos_cache.clone();
                            Task::perform(async move { cache.save().await }, |_| Message::NoOp)
                        } else {
                            Task::none()
                        }
                    }
                }
            }
            Message::RefreshSubscriptionVideos => {
                // Clear cache timestamps to force refetch
                self.subscription_videos_cache.channels.clear();
                self.subscription_videos.clear();
                self.fetch_stale_subscription_videos()
            }
            Message::ExportSearchResults => {
                if !self.search_results.is_empty() {
                    let mut content = String::new();
                    content.push_str("=== SnowTube Search Results Export ===\n\n");

                    for (idx, result) in self.search_results.iter().enumerate() {
                        content.push_str(&format!("{}. {}\n", idx + 1, result.title));
                        if let Some(ref channel) = result.channel {
                            content.push_str(&format!("   Channel: {}\n", channel.name));
                        }
                        if let Some(views) = result.view_count {
                            content.push_str(&format!("   Views: {}\n", views));
                        }
                        if let Some(ref duration) = result.duration_string {
                            content.push_str(&format!("   Duration: {}\n", duration));
                        }
                        content.push_str(&format!("   Video ID: {}\n", result.id));
                        content.push('\n');
                    }

                    let filename = format!(
                        "snowtube-search-export-{}.txt",
                        chrono::Utc::now().format("%Y%m%d-%H%M%S")
                    );

                    Task::perform(
                        async move {
                            tokio::fs::write(&filename, content)
                                .await
                                .map_err(|e| e.to_string())
                                .map(|_| filename)
                        },
                        |result| match result {
                            Ok(_filename) => Message::NoOp,
                            Err(e) => Message::ShowError(format!("Failed to export: {}", e)),
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::PlayVideo(video) => {
                let video_id_str = video.id.clone();
                let title = Some(video.title.clone());
                let duration = video.duration.map(std::time::Duration::from_secs);

                // Store the video info
                self.playing_video_info = Some((*video).clone());

                self.playing_video_id = Some(video_id_str.clone());
                // Only update previous_view if we're not already in Video view
                if self.current_view != View::Video {
                    self.previous_view = self.current_view;
                }
                self.current_view = View::Video;

                // Abort any in-progress loading from the previous player
                if let Some(ref player) = self.video_player
                    && let Some(ref handle) = player.loading_handle
                {
                    handle.abort();
                }

                // Create video player state with the new high-level API
                let source = match VideoSource::from_video(&video) {
                    Ok(s) => s,
                    Err(e) => {
                        return Task::done(Message::ShowError(format!("Cannot play video: {}", e)));
                    }
                };
                let mut state = VideoPlayerState::new(source.clone())
                    .with_visualizer(self.config.audio_visualizer);
                if let Some(t) = title {
                    state = state.with_title(t);
                }
                if let Some(d) = duration {
                    state = state.with_duration(d);
                }
                // Don't set thumbnail here - wait for high-res version
                self.video_player = Some(state);

                // Fetch high-res video thumbnail
                let video_for_thumb = video.clone();
                let thumb_task = Task::perform(
                    async move { providers::fetch_thumbnail_for_video(&video_for_thumb).await },
                    Message::VideoThumbnailLoaded,
                );

                // Fetch video metadata (full description, channel info including avatar)
                let video_for_metadata = video.clone();
                let metadata_task = Task::perform(
                    async move { providers::get_video_metadata(&video_for_metadata).await },
                    Message::VideoMetadataLoaded,
                );

                Task::batch([thumb_task, metadata_task])
            }
            Message::PlayAudioOnly(video) => {
                // Similar to PlayVideo but uses audio-only source
                let video_id_str = video.id.clone();
                let title = Some(video.title.clone());
                let duration = video.duration.map(std::time::Duration::from_secs);

                // Store the video info
                self.playing_video_info = Some((*video).clone());

                self.playing_video_id = Some(video_id_str.clone());
                // Only update previous_view if we're not already in Video view
                if self.current_view != View::Video {
                    self.previous_view = self.current_view;
                }
                self.current_view = View::Video;

                // Abort any in-progress loading from the previous player
                if let Some(ref player) = self.video_player
                    && let Some(ref handle) = player.loading_handle
                {
                    handle.abort();
                }

                // Create video player state with audio-only source and auto-start loading
                let source = match VideoSource::from_video_audio_only(&video) {
                    Ok(s) => s,
                    Err(e) => {
                        return Task::done(Message::ShowError(format!("Cannot play audio: {}", e)));
                    }
                };
                let mut state = VideoPlayerState::new(source.clone())
                    .with_visualizer(self.config.audio_visualizer);
                if let Some(t) = title {
                    state = state.with_title(t);
                }
                if let Some(d) = duration {
                    state = state.with_duration(d);
                }
                // Auto-start loading since user clicked from an active video view
                state.loading = true;
                state.loading_status = Some("Initializing...".to_string());
                let (load_task, load_handle) = iceplayer::widget::start_loading(source);
                state.loading_handle = Some(load_handle);
                self.video_player = Some(state);

                // Start loading the audio
                let load_task = load_task.map(Message::VideoPlayer);

                // Fetch high-res video thumbnail (shows while audio plays)
                let video_for_thumb = video.clone();
                let thumb_task = Task::perform(
                    async move { providers::fetch_thumbnail_for_video(&video_for_thumb).await },
                    Message::VideoThumbnailLoaded,
                );

                // Fetch video metadata (full description, channel info including avatar)
                let video_for_metadata = video.clone();
                let metadata_task = Task::perform(
                    async move { providers::get_video_metadata(&video_for_metadata).await },
                    Message::VideoMetadataLoaded,
                );

                Task::batch([load_task, thumb_task, metadata_task])
            }
            Message::VideoPlayer(msg) => {
                // Delegate to the video player widget's update function
                if let Some(ref mut state) = self.video_player {
                    let (event, task) = iceplayer::widget::update(state, msg);

                    // Handle any events emitted by the player
                    let event_task = if let Some(ev) = event {
                        Task::done(Message::VideoEvent(ev))
                    } else {
                        Task::none()
                    };

                    Task::batch([task.map(Message::VideoPlayer), event_task])
                } else {
                    Task::none()
                }
            }
            Message::SeekTo(position) => {
                // Seek by setting preview position and releasing
                Task::batch([
                    Task::done(Message::VideoPlayer(VideoPlayerMessage::SeekPreview(
                        position,
                    ))),
                    Task::done(Message::VideoPlayer(VideoPlayerMessage::SeekRelease)),
                ])
            }
            Message::SeekRelative(seconds) => {
                // Seek relative to current position
                if let Some(ref state) = self.video_player {
                    let position = state.position().as_secs_f64();
                    let duration = state.duration().as_secs_f64();
                    if duration > 0.0 {
                        let new_pos = if seconds >= 0 {
                            (position + seconds as f64).min(duration) / duration
                        } else {
                            (position + seconds as f64).max(0.0) / duration
                        };
                        return Task::done(Message::SeekTo(new_pos));
                    }
                }
                Task::none()
            }
            Message::ExitFullscreen => {
                // Only exit fullscreen if currently in fullscreen
                if let Some(ref state) = self.video_player
                    && state.fullscreen
                {
                    return Task::done(Message::VideoPlayer(VideoPlayerMessage::ToggleFullscreen));
                }
                Task::none()
            }
            Message::VideoThumbnailLoaded(result) => {
                if let Ok(bytes) = result
                    && let Some(ref mut state) = self.video_player
                {
                    let handle = iced::widget::image::Handle::from_bytes(bytes);
                    state.thumbnail = Some(handle);
                }
                Task::none()
            }

            Message::VideoMetadataLoaded(result) => {
                if let Ok(metadata) = result {
                    let channel_id = metadata.channel_id.clone();
                    let avatar_url = metadata.channel_avatar_url.clone();
                    let platform_name = self
                        .playing_video_info
                        .as_ref()
                        .map(|v| v.platform_name.clone());

                    if let Some(ref mut video_info) = self.playing_video_info {
                        // Update description if we got a full one
                        if let Some(desc) = metadata.description {
                            video_info.description = Some(desc);
                        }
                        // Update channel info if available
                        if let Some(ref mut channel) = video_info.channel {
                            if let Some(name) = metadata.channel_name {
                                channel.name = name;
                            }
                            if let Some(ref id) = channel_id {
                                channel.id = Some(id.clone());
                            }
                        }
                    }

                    // Fetch channel avatar if we have URL and channel_id, and not already cached
                    if let (Some(platform), Some(cid), Some(url)) =
                        (platform_name, channel_id, avatar_url)
                    {
                        let channel_key = common::ChannelKey::new(&platform, &cid);
                        if !self.channel_avatars.contains_key(&channel_key) {
                            return Task::perform(
                                async move {
                                    helpers::load_circular_thumb(&url, 80)
                                        .await
                                        .map_err(|e| e.to_string())
                                },
                                move |res| Message::ChannelAvatarLoaded(channel_key, res),
                            );
                        }
                    }
                }
                Task::none()
            }

            Message::VideoEvent(event) => {
                match event {
                    PlayerEvent::FullscreenChanged(fullscreen) => {
                        // Handle window fullscreen mode
                        #[cfg(target_os = "macos")]
                        {
                            if fullscreen {
                                iced::window::latest()
                                    .and_then(|id| iced::window::maximize(id, true))
                            } else {
                                iced::window::latest()
                                    .and_then(|id| iced::window::maximize(id, false))
                            }
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            let mode = if fullscreen {
                                iced::window::Mode::Fullscreen
                            } else {
                                iced::window::Mode::Windowed
                            };
                            iced::window::latest()
                                .and_then(move |id| iced::window::set_mode(id, mode))
                        }
                    }
                    PlayerEvent::Ready { duration: _ } => Task::none(),
                    PlayerEvent::Ended => Task::none(),
                    PlayerEvent::Error(err) => {
                        tracing::error!("Video error: {}", err);
                        Task::none()
                    }
                    PlayerEvent::PlayStateChanged { playing: _ } => Task::none(),
                }
            }
            Message::BackFromVideo => {
                // Abort any in-progress loading
                if let Some(ref player) = self.video_player
                    && let Some(ref handle) = player.loading_handle
                {
                    handle.abort();
                }
                // Clean up video player state
                self.video_player = None;
                self.playing_video_id = None;
                self.current_view = self.previous_view;

                // Exit fullscreen if we were in it
                iced::window::latest()
                    .and_then(|id| iced::window::set_mode(id, iced::window::Mode::Windowed))
            }
            Message::LaunchInMpv(video_id) => {
                // Launch video in mpv
                let url = format!("https://www.youtube.com/watch?v={}", video_id);
                Task::perform(
                    async move {
                        tokio::process::Command::new("mpv")
                            .arg(&url)
                            .arg("--ytdl=yes")
                            .arg("--script-opts=ytdl_hook-ytdl_path=yt-dlp")
                            .spawn()
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                    |result| match result {
                        Ok(_) => Message::NoOp,
                        Err(e) => Message::ShowError(format!("Failed to launch mpv: {}", e)),
                    },
                )
            }
            Message::CopyVideoUrl(url) => iced::clipboard::write(url),

            // Notification handling
            Message::ShowError(msg) => {
                self.show_error(msg);
                Task::none()
            }
            Message::DismissNotification(id) => {
                self.notifications.retain(|n| n.id != id);
                Task::none()
            }
            Message::NotificationTick => {
                // Auto-dismiss notifications older than 10 seconds
                let now = std::time::Instant::now();
                self.notifications
                    .retain(|n| now.duration_since(n.created_at).as_secs() < 10);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        use iced::Length;
        use iced::widget::{button, column, container, row, stack, text};

        let content = match self.current_view {
            View::Search => views::search::view(self),
            View::Channel => views::channel::view(self, get_language_by_locale),
            View::Config => views::config::view(self),
            View::Channels => views::subscriptions::view(self),
            View::Video => views::video::view(self),
        };

        // Build notification toasts (show first 3 from queue)
        let notifications_overlay: Element<'_, Message> = if self.notifications.is_empty() {
            container(iced::widget::Space::new()).into()
        } else {
            let toasts: Vec<Element<'_, Message>> = self
                .notifications
                .iter()
                .take(3)
                .map(|n| {
                    let dismiss_btn = button(text("×").size(16))
                        .on_press(Message::DismissNotification(n.id))
                        .padding(4)
                        .style(|_theme: &iced::Theme, _status| button::Style {
                            text_color: iced::Color::WHITE,
                            background: None,
                            ..Default::default()
                        });

                    container(
                        row![
                            text(&n.message)
                                .size(14)
                                .color(iced::Color::WHITE)
                                .width(Length::Fill),
                            dismiss_btn
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                    )
                    .padding(12)
                    .width(Length::Fixed(400.0))
                    .style(|_theme: &iced::Theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgba(
                            0.7, 0.1, 0.1, 0.95,
                        ))),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
                })
                .collect();

            container(column(toasts).spacing(8))
                .width(Length::Fill)
                .padding(20)
                .align_x(iced::alignment::Horizontal::Right)
                .into()
        };

        // In video view fullscreen, skip the tab bar entirely
        if self.current_view == View::Video
            && let Some(ref state) = self.video_player
            && state.fullscreen
        {
            return stack![
                container(content).width(Length::Fill).height(Length::Fill),
                notifications_overlay
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        // Create iOS-style tab bar at the bottom
        let tab_bar = widgets::tab_bar(self.active_tab, &widgets::default_tab_items());

        // Stack: content fills the screen, tab bar floats at bottom (overlapping)
        // Bottom padding is now inside each view's scrollable content
        stack![
            container(content).width(Length::Fill).height(Length::Fill),
            container(tab_bar)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom),
            notifications_overlay
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let events = event::listen_with(|ev, _status, _id| match ev {
            iced::Event::Window(iced::window::Event::Resized(Size { width, height })) => {
                Some(Message::Resized(width, height))
            }
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Character(c),
                modifiers,
                ..
            }) if modifiers.control() && c.as_ref() == "e" => Some(Message::ExportSearchResults),
            _ => None,
        });

        // Video player subscription (controls timeout, position updates)
        let video_sub = if let Some(ref state) = self.video_player {
            iceplayer::widget::subscription(state).map(Message::VideoPlayer)
        } else {
            Subscription::none()
        };

        // Video player keyboard shortcuts (only active when video is loaded and actually playing)
        let video_keys = if self.current_view == View::Video
            && let Some(ref state) = self.video_player
            && state.video.is_some()
            && state.started
            && state.position().as_millis() > 0
        {
            let position_ms = state.position().as_millis() as u64;
            let duration_ms = state.duration().as_millis() as u64;
            event::listen().with((position_ms, duration_ms)).filter_map(
                |((_position_ms, _duration_ms), ev)| {
                    if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        key, ..
                    }) = ev
                    {
                        match key {
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::Space) => {
                                return Some(Message::VideoPlayer(
                                    VideoPlayerMessage::TogglePlayPause,
                                ));
                            }
                            // F to toggle fullscreen
                            iced::keyboard::Key::Character(ref c) if c.as_str() == "f" => {
                                return Some(Message::VideoPlayer(
                                    VideoPlayerMessage::ToggleFullscreen,
                                ));
                            }
                            // Escape or Q to exit fullscreen
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                                return Some(Message::ExitFullscreen);
                            }
                            iced::keyboard::Key::Character(ref c) if c.as_str() == "q" => {
                                return Some(Message::ExitFullscreen);
                            }
                            // Arrow right/left: seek (position calculated in SeekTo handler)
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                                return Some(Message::SeekRelative(5));
                            }
                            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                                return Some(Message::SeekRelative(-5));
                            }
                            _ => {}
                        }
                    }
                    None
                },
            )
        } else {
            Subscription::none()
        };

        // Notification auto-dismiss timer (tick every second if there are notifications)
        let notification_timer = if !self.notifications.is_empty() {
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::NotificationTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([events, video_sub, video_keys, notification_timer])
    }
}
