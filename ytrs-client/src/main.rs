mod config;
mod helpers;
mod messages;
mod theme;
mod views;
mod widgets;

use iced_video_player::Video;

use iced::widget::combo_box;
use iced::{Element, Size, Subscription, Task, Theme, event};
use std::cell::Cell;
use std::collections::HashMap;

use std::sync::OnceLock;
use tracing::trace;
use ytrs_lib::{
    ChannelInfo, ChannelTab, InnerTube, LanguageOption, SearchResult, SortFilter, get_all_languages,
};

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
        .font(include_bytes!("../fonts/JetBrainsMonoNerdFont-Regular.ttf"))
        .default_font(iced::Font {
            family: iced::font::Family::Name("Rounded Mplus 1c"),
            ..iced::Font::DEFAULT
        })
        .run()
}

fn cosmic_title(_: &App) -> String {
    "ytrs".to_string()
}

fn app_theme(app: &App) -> Theme {
    app.current_theme.clone()
}

pub struct App {
    // Shared state
    pub query: String,
    pub thumbs: HashMap<String, iced::widget::image::Handle>,
    pub subscription_thumbs: HashMap<String, iced::widget::image::Handle>, // Channel avatars for subscriptions
    pub subscription_videos: HashMap<String, Vec<SearchResult>>, // channel_id -> last 2 videos
    pub subscription_videos_cache: config::SubscriptionVideoCache, // Persistent cache
    pub subscription_videos_loading: std::collections::HashSet<String>, // Channels currently being fetched
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
    pub pending_thumb_updates: Vec<(String, Vec<u8>)>, // Batched thumbnail updates
    pub last_thumb_update: Option<std::time::Instant>, // Last time we processed thumb updates

    // Search-specific state
    pub search_results: Vec<SearchResult>,
    pub search_continuation: Option<String>,
    pub search_preload_count: usize,
    pub search_locale: (String, String),
    pub searching: bool,
    pub search_loading_more: bool,
    pub search_preloading: bool,

    // Channel-specific state
    pub channel_results: Vec<SearchResult>,
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

    // Video player state
    pub video: Option<Video>,
    pub playing_video_title: Option<String>,
    pub video_fullscreen: bool,
    pub video_controls_visible: bool,
    pub video_last_mouse_move: Option<std::time::Instant>,
    pub video_seek_preview: Option<f64>, // Preview position while dragging slider (0.0 to 1.0)
    pub notification: Option<String>,    // Temporary notification message
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                // Shared state
                query: String::new(),
                thumbs: HashMap::new(),
                subscription_thumbs: HashMap::new(),
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
                pending_thumb_updates: Vec::new(),
                last_thumb_update: None,

                // Search-specific state
                search_results: Vec::new(),
                search_continuation: None,
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

                // Video player state
                video: None,
                playing_video_title: None,
                video_fullscreen: false,
                video_controls_visible: true,
                video_last_mouse_move: None,
                video_seek_preview: None,
                notification: None,
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

    /// Fetch videos for subscribed channels that are stale (>10h old or not cached)
    fn fetch_stale_subscription_videos(&mut self) -> Task<Message> {
        // Collect channels to fetch first to avoid borrow issues
        let channels_to_fetch: Vec<_> = self
            .config
            .channels
            .iter()
            .filter(|c| c.subscribed)
            .filter(|c| {
                !self.subscription_videos_loading.contains(&c.channel_id)
                    && self.subscription_videos_cache.is_stale(&c.channel_id)
            })
            .map(|c| {
                let (hl, gl) = c
                    .language
                    .clone()
                    .or_else(|| {
                        self.config
                            .default_language
                            .as_ref()
                            .map(|l| (l.hl.clone(), l.gl.clone()))
                    })
                    .unwrap_or_else(|| ("en".to_string(), "US".to_string()));
                (c.channel_id.clone(), c.channel_name.clone(), hl, gl)
            })
            .collect();

        // Mark channels as loading
        for (channel_id, _, _, _) in &channels_to_fetch {
            self.subscription_videos_loading.insert(channel_id.clone());
        }

        let tasks: Vec<Task<Message>> = channels_to_fetch
            .into_iter()
            .map(|(channel_id, channel_name, hl, gl)| {
                let channel_id_for_msg = channel_id.clone();
                let channel_name_for_msg = channel_name.clone();
                Task::perform(
                    async move {
                        let client = ytrs_lib::InnerTube::new()
                            .await
                            .map_err(|e| e.to_string())?;
                        client
                            .get_channel_videos_with_explicit_locale(
                                &channel_id,
                                ytrs_lib::ChannelTab::Videos,
                                &hl,
                                &gl,
                            )
                            .await
                            .map_err(|e| e.to_string())
                    },
                    move |res| {
                        Message::SubscriptionVideosLoaded(
                            channel_id_for_msg,
                            channel_name_for_msg,
                            res,
                        )
                    },
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
                self.search_continuation = None;
                self.search_preload_count = 0;
                self.search_preloading = true;
                let q = self.query.clone();

                // Use manual locale if selected, otherwise auto-detect
                if let Some(ref language) = self.selected_language {
                    let hl = language.hl.to_string();
                    let gl = language.gl.to_string();
                    Task::perform(
                        async move {
                            let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                            client
                                .search_with_locale(&q, &hl, &gl)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        Message::SearchDone,
                    )
                } else {
                    Task::perform(
                        async move {
                            let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                            client.search(&q).await.map_err(|e| e.to_string())
                        },
                        Message::SearchDone,
                    )
                }
            }
            Message::SearchDone(res) => {
                match res {
                    Ok(search_results) => {
                        // Check if this is a "load more" operation (appending results)
                        let is_load_more = self.search_loading_more;
                        self.search_loading_more = false;

                        // Store the new results to load thumbnails for
                        let new_results = search_results.results;

                        // Store continuation token for pagination
                        self.search_continuation = search_results.continuation;

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

                            // Keep fetching if we don't have enough displayable results and have continuation
                            if displayable_count < MIN_DISPLAYABLE_RESULTS
                                && self.search_continuation.is_some()
                            {
                                let token = self.search_continuation.as_ref().unwrap().clone();
                                let (hl, gl) = self.search_locale.clone();

                                // Start loading thumbnails for current batch while fetching next page
                                let thumb_tasks = helpers::create_thumbnail_tasks(&new_results);

                                // Fetch next page with stored locale
                                let next_page_task = Task::perform(
                                    async move {
                                        let client =
                                            InnerTube::new().await.map_err(|e| e.to_string())?;
                                        client
                                            .search_continuation(&token, &hl, &gl)
                                            .await
                                            .map_err(|e| e.to_string())
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
                        trace!("Search error: {:?}", e);
                        self.search_preloading = false;
                        self.searching = false;
                        self.search_loading_more = false;
                        Task::none()
                    }
                }
            }
            Message::ThumbLoaded(id, res) => {
                if let Ok(bytes) = res {
                    // Batch thumbnail updates instead of updating immediately
                    self.pending_thumb_updates.push((id, bytes));

                    let now = std::time::Instant::now();
                    let should_flush = match self.last_thumb_update {
                        None => true,
                        Some(last) => {
                            // Flush if we have 10+ pending or 100ms has passed
                            self.pending_thumb_updates.len() >= 10
                                || now.duration_since(last).as_millis() >= 100
                        }
                    };

                    if should_flush {
                        // Process all pending updates at once
                        for (thumb_id, thumb_bytes) in self.pending_thumb_updates.drain(..) {
                            self.thumbs.insert(
                                thumb_id,
                                iced::widget::image::Handle::from_bytes(thumb_bytes),
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
            Message::ViewChannel(channel_id) => {
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

                let id = channel_id.clone();

                // Determine channel language:
                // 1. Use per-channel saved language if set
                // 2. Otherwise use global default from config
                // 3. Otherwise auto-detect
                let channel_language = self
                    .config
                    .channels
                    .iter()
                    .find(|c| c.channel_id == channel_id)
                    .and_then(|c| c.language.clone());

                if let Some((hl, gl)) = channel_language {
                    // This channel has a specific language set
                    self.channel_locale = (hl.clone(), gl.clone());
                    self.selected_language = ytrs_lib::get_language_by_locale(&hl, &gl).cloned();
                } else if let Some(ref lang_config) = self.config.default_language {
                    // Use global default language
                    self.channel_locale = (lang_config.hl.clone(), lang_config.gl.clone());
                    self.selected_language = lang_config.to_language_option();
                } else {
                    // No language set - will auto-detect
                    self.selected_language = None;
                }

                // First load channel info, then use channel name for locale detection when loading videos
                Task::perform(
                    async move {
                        let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                        client.get_channel(&id).await.map_err(|e| e.to_string())
                    },
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

                        // Load channel avatar
                        let avatar_task = if let Some(thumb) = channel.thumbnails.first() {
                            let url = thumb.url.clone();
                            let id = channel.id.clone();
                            Task::perform(
                                async move {
                                    helpers::load_thumb(&url).await.map_err(|e| e.to_string())
                                },
                                move |r| Message::ThumbLoaded(id.clone(), r),
                            )
                        } else {
                            Task::none()
                        };

                        // Load channel videos - use manual language if selected, otherwise auto-detect
                        let channel_id = channel.id.clone();
                        let tab = self.current_tab;

                        let videos_task = if let Some(ref lang) = self.selected_language {
                            // Use manually selected language
                            let hl = lang.hl.to_string();
                            let gl = lang.gl.to_string();
                            Task::perform(
                                async move {
                                    let client =
                                        InnerTube::new().await.map_err(|e| e.to_string())?;
                                    client
                                        .get_channel_videos_with_explicit_locale(
                                            &channel_id,
                                            tab,
                                            &hl,
                                            &gl,
                                        )
                                        .await
                                        .map_err(|e| e.to_string())
                                },
                                Message::ChannelVideosLoaded,
                            )
                        } else {
                            // Auto-detect locale from channel description/name
                            let locale_hint =
                                channel.description.clone().or(Some(channel.name.clone()));
                            Task::perform(
                                async move {
                                    let client =
                                        InnerTube::new().await.map_err(|e| e.to_string())?;
                                    client
                                        .get_channel_videos_with_locale(
                                            &channel_id,
                                            tab,
                                            locale_hint.as_deref(),
                                        )
                                        .await
                                        .map_err(|e| e.to_string())
                                },
                                Message::ChannelVideosLoaded,
                            )
                        };

                        self.current_channel = Some(channel);
                        Task::batch(vec![banner_task, avatar_task, videos_task])
                    }
                    Err(e) => {
                        trace!("Channel load error: {:?}", e);
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

                                // Start loading thumbnails for current batch while fetching next page
                                let thumb_tasks = helpers::create_thumbnail_tasks(&new_videos);

                                // Fetch next page with stored locale
                                let next_page_task = Task::perform(
                                    async move {
                                        let client =
                                            InnerTube::new().await.map_err(|e| e.to_string())?;
                                        client
                                            .get_channel_videos_continuation_with_locale(
                                                &token, &hl, &gl,
                                            )
                                            .await
                                            .map_err(|e| e.to_string())
                                    },
                                    Message::ChannelVideosLoaded,
                                );

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
                        trace!("Channel videos load error: {:?}", e);
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

                    let channel_id = channel.id.clone();
                    // Use stored locale for consistent results across tabs
                    let (hl, gl) = self.channel_locale.clone();
                    Task::perform(
                        async move {
                            let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                            client
                                .get_channel_videos_with_explicit_locale(&channel_id, tab, &hl, &gl)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        Message::ChannelVideosLoaded,
                    )
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
                {
                    self.selected_sort_label = Some(label);
                    self.channel_results.clear();
                    self.channel_continuation = None; // Will be updated with new continuation
                    self.channel_preload_count = 0;
                    self.channel_preloading = true;
                    self.loading_channel = true;

                    let token = token.clone();
                    let (hl, gl) = self.channel_locale.clone();
                    return Task::perform(
                        async move {
                            let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                            client
                                .get_channel_videos_continuation_with_locale(&token, &hl, &gl)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        Message::ChannelVideosLoaded,
                    );
                }
                Task::none()
            }

            Message::LoadMoreVideos => {
                if let Some(ref token) = self.channel_continuation
                    && !self.channel_loading_more
                {
                    self.channel_loading_more = true;
                    // Enable preloading to fetch 3 more pages
                    self.channel_preload_count = 0;
                    self.channel_preloading = true;

                    let token = token.clone();
                    // Use stored locale for consistent results
                    let (hl, gl) = self.channel_locale.clone();
                    return Task::perform(
                        async move {
                            let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                            client
                                .get_channel_videos_continuation_with_locale(&token, &hl, &gl)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        Message::ChannelVideosLoaded,
                    );
                }
                Task::none()
            }
            Message::LoadMoreSearchResults => {
                if let Some(ref token) = self.search_continuation
                    && !self.search_loading_more
                {
                    self.search_loading_more = true;
                    // Enable preloading to fetch 3 more pages
                    self.search_preload_count = 0;
                    self.search_preloading = true;

                    let token = token.clone();
                    // Use stored locale for consistent results
                    let (hl, gl) = self.search_locale.clone();
                    return Task::perform(
                        async move {
                            let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                            client
                                .search_continuation(&token, &hl, &gl)
                                .await
                                .map_err(|e| e.to_string())
                        },
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

                // Load thumbnails for any newly subscribed channels
                let tasks: Vec<Task<Message>> = self
                    .config
                    .channels
                    .iter()
                    .filter(|c| c.subscribed)
                    .filter(|c| !self.subscription_thumbs.contains_key(&c.channel_id))
                    .map(|c| {
                        let channel_id = c.channel_id.clone();
                        let url = c.thumbnail_url.clone();
                        Task::perform(
                            async move {
                                helpers::load_circular_thumb(&url, 80)
                                    .await
                                    .map_err(|e| e.to_string())
                            },
                            move |res| {
                                Message::SubscriptionChannelThumbLoaded(channel_id.clone(), res)
                            },
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
                            if let Some(channel_config) = self
                                .config
                                .channels
                                .iter_mut()
                                .find(|c| c.channel_id == channel.id)
                            {
                                // Update existing channel config
                                channel_config.language = Some(language_tuple);
                            } else {
                                // Create new channel config with just language
                                let thumbnail_url = channel
                                    .thumbnails
                                    .last()
                                    .map(|t| t.url.clone())
                                    .unwrap_or_default();

                                self.config.channels.push(ytrs_lib::ChannelConfig {
                                    channel_id: channel.id.clone(),
                                    channel_name: channel.name.clone(),
                                    channel_handle: channel.handle.clone(),
                                    thumbnail_url,
                                    subscribed: false,
                                    subscribed_at: None,
                                    language: Some((hl.clone(), gl.clone())),
                                });
                            }

                            // Save config
                            let new_config = YtrsConfig {
                                config: self.config.clone(),
                                ..Default::default()
                            };
                            let save_task = Task::perform(
                                async move { new_config.save().await.map_err(|e| e.to_string()) },
                                Message::ConfigSaved,
                            );

                            self.channel_results.clear();
                            self.channel_continuation = None;
                            self.channel_preload_count = 0;
                            self.channel_preloading = true;
                            self.loading_channel = true;

                            let channel_id = channel.id.clone();

                            // Fetch channel info first
                            let fetch_task = Task::perform(
                                async move {
                                    let client =
                                        InnerTube::new().await.map_err(|e| e.to_string())?;
                                    client
                                        .get_channel(&channel_id)
                                        .await
                                        .map_err(|e| e.to_string())
                                },
                                Message::ChannelLoaded,
                            );

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
                            self.search_continuation = None;
                            self.search_preload_count = 0;
                            self.search_preloading = true;
                            let q = self.query.clone();

                            Task::perform(
                                async move {
                                    let client =
                                        InnerTube::new().await.map_err(|e| e.to_string())?;
                                    client
                                        .search_with_locale(&q, &hl, &gl)
                                        .await
                                        .map_err(|e| e.to_string())
                                },
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

                        let config = self.config.clone();
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
                        trace!("Config load error: {:?}", e);
                    }
                }
                Task::none()
            }
            Message::ConfigSaved(result) => {
                if let Err(e) = result {
                    trace!("Config save error: {:?}", e);
                }
                Task::none()
            }
            Message::ThemeChanged(new_theme) => {
                self.current_theme = new_theme.to_iced_theme();
                self.config.theme = new_theme;

                let config = self.config.clone();
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
            Message::Resized(width, height) => {
                self.window_width = width;
                self.window_height = height;
                Task::none()
            }
            Message::SubscribeToChannel => {
                if let Some(ref channel) = self.current_channel {
                    // Check if channel config already exists
                    let existing_config = self
                        .config
                        .channels
                        .iter_mut()
                        .find(|c| c.channel_id == channel.id);

                    if let Some(channel_config) = existing_config {
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
                        let channel_config = ytrs_lib::ChannelConfig {
                            channel_id: channel.id.clone(),
                            channel_name: channel.name.clone(),
                            channel_handle: channel.handle.clone(),
                            thumbnail_url,
                            subscribed: true,
                            subscribed_at: Some(chrono::Utc::now().to_rfc3339()),
                            language: None,
                        };

                        // Add to config
                        self.config.channels.push(channel_config);
                    }

                    // Save config
                    let new_config = YtrsConfig {
                        config: self.config.clone(),
                        ..Default::default()
                    };

                    return Task::perform(
                        async move { new_config.save().await.map_err(|e| e.to_string()) },
                        Message::ConfigSaved,
                    );
                }
                Task::none()
            }
            Message::UnsubscribeFromChannel(channel_id) => {
                // Find the channel config
                if let Some(channel_config) = self
                    .config
                    .channels
                    .iter_mut()
                    .find(|c| c.channel_id == channel_id)
                {
                    channel_config.subscribed = false;
                    channel_config.subscribed_at = None;

                    // If no language override, remove the entry entirely
                    if channel_config.language.is_none() {
                        self.config.channels.retain(|c| c.channel_id != channel_id);
                    }
                }

                // Remove thumbnail from cache
                self.subscription_thumbs.remove(&channel_id);

                // Save config
                let new_config = YtrsConfig {
                    config: self.config.clone(),
                    ..Default::default()
                };

                Task::perform(
                    async move { new_config.save().await.map_err(|e| e.to_string()) },
                    Message::ConfigSaved,
                )
            }
            Message::SubscriptionChannelThumbLoaded(channel_id, res) => {
                if let Ok(bytes) = res {
                    self.subscription_thumbs
                        .insert(channel_id, iced::widget::image::Handle::from_bytes(bytes));
                }
                Task::none()
            }
            Message::TabSelected(tab_id) => {
                self.active_tab = tab_id;
                let task = match tab_id {
                    TabId::Search => {
                        self.current_view = View::Search;
                        Task::none()
                    }
                    TabId::Channels => {
                        self.current_view = View::Channels;
                        // Load circular thumbnails for all subscribed channels that aren't already loaded
                        let thumb_tasks: Vec<Task<Message>> = self
                            .config
                            .channels
                            .iter()
                            .filter(|c| c.subscribed)
                            .filter(|c| !self.subscription_thumbs.contains_key(&c.channel_id))
                            .map(|c| {
                                let channel_id = c.channel_id.clone();
                                let url = c.thumbnail_url.clone();
                                Task::perform(
                                    async move {
                                        helpers::load_circular_thumb(&url, 80)
                                            .await
                                            .map_err(|e| e.to_string())
                                    },
                                    move |res| {
                                        Message::SubscriptionChannelThumbLoaded(
                                            channel_id.clone(),
                                            res,
                                        )
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

                        Task::batch(thumb_tasks).chain(cache_task)
                    }
                    TabId::Settings => {
                        self.current_view = View::Config;
                        Task::none()
                    }
                };

                task
            }
            Message::NoOp => Task::none(),
            Message::SubscriptionVideosCacheLoaded(result) => {
                match result {
                    Ok(cache) => {
                        // Populate subscription_videos from cache and collect videos for thumbnail loading
                        let mut all_videos: Vec<SearchResult> = Vec::new();
                        for (channel_id, cached) in &cache.channels {
                            // Look up channel name from config
                            let channel_name = self
                                .config
                                .channels
                                .iter()
                                .find(|c| &c.channel_id == channel_id)
                                .map(|c| c.channel_name.clone());
                            // Populate channel info on videos if missing
                            let videos: Vec<SearchResult> = cached
                                .videos
                                .iter()
                                .cloned()
                                .map(|mut v| {
                                    if v.channel.is_none() {
                                        if let Some(ref name) = channel_name {
                                            v.channel = Some(ytrs_lib::Channel {
                                                id: Some(channel_id.clone()),
                                                name: name.clone(),
                                                url: None,
                                                thumbnail: None,
                                            });
                                        }
                                    }
                                    v
                                })
                                .collect();
                            all_videos.extend(videos.clone());
                            self.subscription_videos.insert(channel_id.clone(), videos);
                        }
                        self.subscription_videos_cache = cache;

                        // Load thumbnails for cached videos
                        let thumb_tasks = helpers::create_thumbnail_tasks(&all_videos);

                        // Now fetch stale videos
                        let fetch_task = self.fetch_stale_subscription_videos();
                        Task::batch(thumb_tasks).chain(fetch_task)
                    }
                    Err(e) => {
                        trace!("Subscription video cache load error: {:?}", e);
                        self.fetch_stale_subscription_videos()
                    }
                }
            }
            Message::SubscriptionVideosLoaded(channel_id, channel_name, result) => {
                self.subscription_videos_loading.remove(&channel_id);
                match result {
                    Ok(channel_videos) => {
                        // Store full first page of videos with channel info populated
                        let videos: Vec<SearchResult> = channel_videos
                            .videos
                            .into_iter()
                            .map(|mut v| {
                                // Populate channel info if not present
                                if v.channel.is_none() {
                                    v.channel = Some(ytrs_lib::Channel {
                                        id: Some(channel_id.clone()),
                                        name: channel_name.clone(),
                                        url: None,
                                        thumbnail: None,
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
                            channel_id.clone(),
                            config::CachedChannelVideos {
                                videos: videos.clone(),
                                fetched_at: now,
                            },
                        );

                        self.subscription_videos.insert(channel_id, videos);

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
                        trace!("Subscription videos load error: {:?}", e);
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
                    content.push_str("=== YTRS Search Results Export ===\n\n");

                    for (idx, result) in self.search_results.iter().enumerate() {
                        content.push_str(&format!("{}. {}\n", idx + 1, result.title));
                        if let Some(ref channel) = result.channel {
                            content.push_str(&format!("   Channel: {}\n", channel.name));
                        }
                        if let Some(views) = result.view_count {
                            content.push_str(&format!("   Views: {}\n", views));
                        }
                        if let Some(ref duration) = result.duration {
                            content.push_str(&format!("   Duration: {}\n", duration));
                        }
                        if let Some(ref video_id) = result.video_id {
                            content.push_str(&format!("   Video ID: {}\n", video_id));
                        }
                        content.push_str("\n");
                    }

                    let filename = format!(
                        "ytrs-search-export-{}.txt",
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
                            Err(e) => {
                                trace!("Subscription save error: {}", e);
                                Message::NoOp
                            }
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::PlayVideo(video_id) => {
                // Find the video title from search results, channel results, or subscription videos
                let title = self
                    .search_results
                    .iter()
                    .find(|r| r.video_id.as_ref() == Some(&video_id))
                    .map(|r| r.title.clone())
                    .or_else(|| {
                        self.channel_results
                            .iter()
                            .find(|r| r.video_id.as_ref() == Some(&video_id))
                            .map(|r| r.title.clone())
                    })
                    .or_else(|| {
                        // Search in subscription videos
                        self.subscription_videos
                            .values()
                            .flatten()
                            .find(|r| r.video_id.as_ref() == Some(&video_id))
                            .map(|r| r.title.clone())
                    });

                self.playing_video_title = title;
                self.previous_view = self.current_view;
                self.current_view = View::Video;

                // Use yt-dlp to get direct URL with headers for seekable playback
                let url = format!("https://www.youtube.com/watch?v={}", video_id);
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            tracing::info!("Starting yt-dlp for URL: {}", url);

                            // Get video URL and headers from yt-dlp
                            let output = std::process::Command::new("yt-dlp")
                                .args(["--dump-single-json", &url])
                                .output()
                                .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

                            if !output.status.success() {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                tracing::error!("yt-dlp failed: {}", stderr);
                                return Err(format!("yt-dlp failed: {}", stderr));
                            }

                            tracing::info!("yt-dlp completed, parsing JSON...");

                            let json: serde_json::Value = serde_json::from_slice(&output.stdout)
                                .map_err(|e| format!("Failed to parse yt-dlp JSON: {}", e))?;

                            // Check if this is a live stream
                            let is_live = json["is_live"].as_bool().unwrap_or(false);

                            if is_live {
                                // Live streams use HLS with muxed video+audio
                                tracing::info!("Detected live stream, using HLS playback");

                                // Get the best format URL (yt-dlp puts best in "url" field)
                                let hls_url = json["url"]
                                    .as_str()
                                    .ok_or("No URL in yt-dlp output for live stream")?
                                    .to_string();

                                tracing::info!(
                                    "Got HLS URL: {}...",
                                    &hls_url[..100.min(hls_url.len())]
                                );

                                // Use playbin for HLS streams (it handles hlsdemux internally)
                                let uri = url::Url::parse(&hls_url)
                                    .map_err(|e| format!("Invalid HLS URL: {}", e))?;

                                // Retry logic for transient Caps errors
                                let mut last_error = None;
                                for attempt in 1..=3 {
                                    tracing::trace!("Live video creation attempt {}/3", attempt);
                                    match Video::new(&uri) {
                                        Ok(video) => {
                                            tracing::info!(
                                                "Live video created successfully on attempt {}",
                                                attempt
                                            );
                                            return Ok(std::sync::Arc::new(video));
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "Live video creation attempt {} failed: {:?}",
                                                attempt,
                                                e
                                            );
                                            last_error = Some(e);
                                            if attempt < 3 {
                                                std::thread::sleep(
                                                    std::time::Duration::from_millis(500),
                                                );
                                            }
                                        }
                                    }
                                }
                                tracing::error!(
                                    "Live video creation failed after 3 attempts: {:?}",
                                    last_error
                                );
                                Err(format!(
                                    "Failed to start live video after 3 attempts: {:?}",
                                    last_error.unwrap()
                                ))
                            } else {
                                // VOD: yt-dlp returns separate video/audio in requested_formats
                                let requested_formats = json["requested_formats"]
                                    .as_array()
                                    .ok_or("No requested_formats in yt-dlp output")?;

                                // Find video and audio streams
                                let video_format = requested_formats
                                    .iter()
                                    .find(|f| {
                                        f["vcodec"].as_str().map(|v| v != "none").unwrap_or(false)
                                    })
                                    .ok_or("No video format found")?;
                                let audio_format = requested_formats
                                    .iter()
                                    .find(|f| {
                                        f["acodec"].as_str().map(|a| a != "none").unwrap_or(false)
                                    })
                                    .ok_or("No audio format found")?;

                                let video_url = video_format["url"]
                                    .as_str()
                                    .ok_or("No video URL")?
                                    .to_string();
                                let audio_url = audio_format["url"]
                                    .as_str()
                                    .ok_or("No audio URL")?
                                    .to_string();

                                tracing::info!(
                                    "Got video URL: {}...",
                                    &video_url[..100.min(video_url.len())]
                                );
                                tracing::info!(
                                    "Got audio URL: {}...",
                                    &audio_url[..100.min(audio_url.len())]
                                );

                                // Get headers from video format
                                let headers_obj = video_format["http_headers"].as_object();
                                let headers: Vec<(String, String)> = headers_obj
                                    .map(|h| {
                                        h.iter()
                                            .filter_map(|(k, v)| {
                                                v.as_str().map(|s| (k.clone(), s.to_string()))
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                tracing::info!("Got {} headers", headers.len());

                                // Wait for available_at timestamp if present (YouTube throttle protection)
                                if let Some(available_at) = video_format["available_at"].as_i64() {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs() as i64;
                                    let wait_secs = available_at - now;
                                    if wait_secs > 0 {
                                        tracing::info!(
                                            "Waiting {} seconds for YouTube throttle (available_at: {})",
                                            wait_secs,
                                            available_at
                                        );
                                        std::thread::sleep(std::time::Duration::from_secs(
                                            wait_secs as u64,
                                        ));
                                    }
                                }

                                let header_refs: Vec<(&str, &str)> = headers
                                    .iter()
                                    .map(|(k, v)| (k.as_str(), v.as_str()))
                                    .collect();

                                tracing::info!("Creating video with from_url_with_headers...");

                                // Retry logic for transient Caps errors
                                let mut last_error = None;
                                for attempt in 1..=3 {
                                    tracing::trace!("VOD video creation attempt {}/3", attempt);
                                    match Video::from_url_with_headers(
                                        &video_url,
                                        &audio_url,
                                        &header_refs,
                                    ) {
                                        Ok(video) => {
                                            tracing::info!(
                                                "VOD video created successfully on attempt {}",
                                                attempt
                                            );
                                            return Ok(std::sync::Arc::new(video));
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "VOD video creation attempt {} failed: {:?}",
                                                attempt,
                                                e
                                            );
                                            last_error = Some(e);
                                            if attempt < 3 {
                                                std::thread::sleep(
                                                    std::time::Duration::from_millis(500),
                                                );
                                            }
                                        }
                                    }
                                }
                                tracing::error!(
                                    "VOD video creation failed after 3 attempts: {:?}",
                                    last_error
                                );
                                Err(format!(
                                    "Failed to start video after 3 attempts: {:?}",
                                    last_error.unwrap()
                                ))
                            }
                        })
                        .await
                        .map_err(|e| format!("Task join error: {}", e))?
                    },
                    Message::VideoLoaded,
                )
            }
            Message::VideoLoaded(result) => {
                match result {
                    Ok(video) => {
                        // Arc::into_inner only works if this is the only reference
                        // Since we just created it, this should always succeed
                        match std::sync::Arc::try_unwrap(video) {
                            Ok(v) => self.video = Some(v),
                            Err(_) => {
                                return Task::perform(
                                    async { "Failed to unwrap video Arc".to_string() },
                                    Message::VideoError,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        return Task::perform(async move { e }, Message::VideoError);
                    }
                }
                Task::none()
            }
            Message::VideoEnded => Task::none(),
            Message::TogglePlayPause => {
                if let Some(ref mut video) = self.video {
                    video.set_paused(!video.paused());
                }
                Task::none()
            }
            Message::ToggleFullscreen => {
                self.video_fullscreen = !self.video_fullscreen;
                let mode = if self.video_fullscreen {
                    iced::window::Mode::Fullscreen
                } else {
                    iced::window::Mode::Windowed
                };
                iced::window::latest().and_then(move |id| iced::window::set_mode(id, mode))
            }
            Message::BackFromVideo => {
                if let Some(ref mut video) = self.video {
                    video.set_paused(true);
                }
                self.video = None;
                self.playing_video_title = None;
                self.current_view = self.previous_view;
                // Exit fullscreen if we were in it
                if self.video_fullscreen {
                    self.video_fullscreen = false;
                    return iced::window::latest()
                        .and_then(|id| iced::window::set_mode(id, iced::window::Mode::Windowed));
                }
                Task::none()
            }
            Message::VideoError(err) => {
                trace!("Video error: {}", err);
                self.current_view = self.previous_view;
                Task::none()
            }
            Message::VideoMouseMoved => {
                // Only process in fullscreen video mode
                if self.current_view != View::Video || !self.video_fullscreen {
                    return Task::none();
                }
                self.video_controls_visible = true;
                self.video_last_mouse_move = Some(std::time::Instant::now());
                // Start a timer to hide controls after 3 seconds
                Task::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    },
                    |_| Message::VideoControlsTimeout,
                )
            }
            Message::VideoControlsTimeout => {
                // Only hide if we're in fullscreen and no recent mouse movement
                if self.video_fullscreen {
                    if let Some(last_move) = self.video_last_mouse_move {
                        if last_move.elapsed() >= std::time::Duration::from_secs(3) {
                            self.video_controls_visible = false;
                        }
                    }
                }
                Task::none()
            }
            Message::SeekVideoPreview(percent) => {
                self.video_seek_preview = Some(percent);
                Task::none()
            }
            Message::SeekVideoRelease => {
                if let Some(percent) = self.video_seek_preview.take() {
                    if let Some(ref mut video) = self.video {
                        let duration = video.duration();
                        let target_nanos = (duration.as_secs_f64() * percent) * 1_000_000_000.0;
                        let target = std::time::Duration::from_nanos(target_nanos as u64);
                        if let Err(e) = video.seek(target, false) {
                            tracing::error!("Failed to seek video: {:?}", e);
                        }
                        // Show notification for longer videos
                        if duration.as_secs() > 600 {
                            self.notification =
                                Some("Seeking may take a moment on longer videos...".to_string());
                            return Task::perform(
                                async {
                                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                                },
                                |_| Message::DismissNotification,
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::DismissNotification => {
                self.notification = None;
                Task::none()
            }
            Message::VideoTick => {
                // Just triggers a re-render to update the progress bar
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        use iced::Length;
        use iced::widget::{container, stack};

        let content = match self.current_view {
            View::Search => views::search::view(self),
            View::Channel => views::channel::view(self, get_language_by_locale),
            View::Config => views::config::view(self),
            View::Channels => views::subscriptions::view(self),
            View::Video => views::video::view(self),
        };

        // In video view, skip the tab bar entirely
        if self.current_view == View::Video {
            return container(content)
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
                .align_y(iced::alignment::Vertical::Bottom)
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
            // Track all mouse movement - filter in handler based on view state
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                Some(Message::VideoMouseMoved)
            }
            _ => None,
        });

        // Add a timer tick while video is playing to update progress bar
        let video_tick = if self.video.is_some()
            && self.current_view == View::Video
            && !self.video.as_ref().map(|v| v.paused()).unwrap_or(true)
        {
            iced::time::every(std::time::Duration::from_millis(250)).map(|_| Message::VideoTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([events, video_tick])
    }
}
