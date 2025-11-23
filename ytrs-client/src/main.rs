mod config;
mod helpers;
mod messages;
mod theme;
mod views;
mod widgets;

use iced::widget::combo_box;
use iced::{Element, Size, Subscription, Task, Theme, event};
use std::cell::Cell;
use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
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
    pub current_view: View,
    pub previous_view: View, // Track which view to return to from config
    pub active_tab: TabId,   // Current active tab in TabBar
    pub last_view_for_timing: Cell<Option<View>>, // Track last view to detect tab switches
    pub language_combo_state: combo_box::State<LanguageOption>,
    pub selected_language: Option<LanguageOption>, // User's manual language override (global)
    pub playing_video: Option<String>,             // Currently playing video ID
    pub countdown_value: u8,                       // Current countdown value (5, 4, 3, 2, 1, 0)
    pub mpv_process: Arc<tokio::sync::Mutex<Option<std::process::Child>>>, // MPV process handle
    pub config: AppConfig,                         // Persistent configuration
    pub window_width: f32,                         // Current window width for responsive layout
    pub current_theme: Theme,                      // Current theme
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
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                // Shared state
                query: String::new(),
                thumbs: HashMap::new(),
                subscription_thumbs: HashMap::new(),
                current_view: View::Search,
                previous_view: View::Search,
                active_tab: TabId::Search,
                last_view_for_timing: Cell::new(None),
                language_combo_state: combo_box::State::new(get_all_languages().to_vec()),
                selected_language: None,
                playing_video: None,
                countdown_value: 0,
                mpv_process: Arc::new(tokio::sync::Mutex::new(None)),
                config: AppConfig::default(),
                window_width: 800.0,
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

    fn update(&mut self, msg: Message) -> Task<Message> {
        let _update_start = std::time::Instant::now();
        let msg_name = format!("{:?}", msg)
            .split('(')
            .next()
            .unwrap_or("Unknown")
            .to_string();

        let result = match msg {
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
                            self.thumbs.clear();
                        } else {
                            // Appending results (load more or preloading)
                            self.search_results.extend(new_results.clone());
                        }

                        // Auto-preload: fetch 3 pages (90 results) before showing content
                        const TARGET_PRELOAD_PAGES: usize = 3;

                        if self.search_preloading {
                            self.search_preload_count += 1;

                            // If we haven't reached target and have continuation, auto-load more
                            if self.search_preload_count < TARGET_PRELOAD_PAGES
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
                        eprintln!("Error: {}", e);
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
            Message::Play(id) => {
                // Set up countdown state
                self.playing_video = Some(id.clone());
                self.countdown_value = 5;

                // Start MPV playback in tokio spawn_blocking
                let url = format!("https://www.youtube.com/watch?v={}", id);
                let mpv_process = self.mpv_process.clone();

                tokio::spawn(async move {
                    // Kill previous MPV process if it exists
                    let mut process_lock = mpv_process.lock().await;
                    if let Some(mut process) = process_lock.take() {
                        let _ = process.kill();
                    }
                    drop(process_lock);

                    // Spawn new MPV process
                    let result = tokio::task::spawn_blocking(move || {
                        Command::new("mpv")
                            .arg(&url)
                            .arg("--ytdl=yes")
                            .arg("--script-opts=ytdl_hook-ytdl_path=yt-dlp")
                            .spawn()
                    })
                    .await;

                    // Store the process handle
                    if let Ok(Ok(child)) = result {
                        let mut process_lock = mpv_process.lock().await;
                        *process_lock = Some(child);
                    }
                });

                // Start countdown timer
                let video_id = id.clone();
                Task::perform(
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        video_id
                    },
                    Message::CountdownTick,
                )
            }
            Message::CountdownTick(video_id) => {
                // Only process countdown if this is still the playing video
                if self.playing_video.as_ref() != Some(&video_id) {
                    return Task::none();
                }

                if self.countdown_value > 0 {
                    self.countdown_value -= 1;
                }

                if self.countdown_value > 0 {
                    // Continue countdown
                    Task::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            video_id
                        },
                        Message::CountdownTick,
                    )
                } else {
                    // Countdown complete, clear playing state
                    self.playing_video = None;
                    Task::none()
                }
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
                // Keep selected_language if it exists (persist from search view)

                let id = channel_id.clone();

                // Use manual locale if selected, otherwise let channel load detect it
                if let Some(ref language) = self.selected_language {
                    let hl = language.hl.to_string();
                    let gl = language.gl.to_string();
                    self.channel_locale = (hl, gl);
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
                        eprintln!("Error loading channel: {}", e);
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
                        eprintln!("Error loading channel videos: {}", e);
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
                Task::none()
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
                            self.channel_results.clear();
                            self.channel_continuation = None;
                            self.channel_preload_count = 0;
                            self.channel_preloading = true;
                            self.loading_channel = true;

                            let channel_id = channel.id.clone();

                            // Fetch channel info first
                            Task::perform(
                                async move {
                                    let client =
                                        InnerTube::new().await.map_err(|e| e.to_string())?;
                                    client
                                        .get_channel(&channel_id)
                                        .await
                                        .map_err(|e| e.to_string())
                                },
                                Message::ChannelLoaded,
                            )
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
                        eprintln!("Failed to load config: {}", e);
                    }
                }
                Task::none()
            }
            Message::ConfigSaved(result) => {
                if let Err(e) = result {
                    eprintln!("Failed to save config: {}", e);
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
            Message::Resized(width, _height) => {
                self.window_width = width;
                Task::none()
            }
            Message::SubscribeToChannel => {
                if let Some(ref channel) = self.current_channel {
                    // Check if already subscribed
                    let already_subscribed = self
                        .config
                        .subscriptions
                        .iter()
                        .any(|sub| sub.channel_id == channel.id);

                    if !already_subscribed {
                        // Get the best quality thumbnail
                        let thumbnail_url = channel
                            .thumbnails
                            .last()
                            .map(|t| t.url.clone())
                            .unwrap_or_default();

                        // Create subscription
                        let subscription = ytrs_lib::ChannelSubscription {
                            channel_id: channel.id.clone(),
                            channel_name: channel.name.clone(),
                            channel_handle: channel.handle.clone(),
                            thumbnail_url,
                            subscribed_at: chrono::Utc::now().to_rfc3339(),
                        };

                        // Add to config
                        self.config.subscriptions.push(subscription);

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
                }
                Task::none()
            }
            Message::UnsubscribeFromChannel(channel_id) => {
                // Remove from subscriptions
                self.config
                    .subscriptions
                    .retain(|sub| sub.channel_id != channel_id);

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
                let _tab_switch_start = std::time::Instant::now();
                eprintln!("\n╔═══════════════════════════════════════════════════");
                eprintln!(
                    "║ TabSelected: switching to {:?} at {:?}",
                    tab_id, _tab_switch_start
                );
                eprintln!("╚═══════════════════════════════════════════════════");

                self.active_tab = tab_id;
                let task = match tab_id {
                    TabId::Search => {
                        self.current_view = View::Search;
                        Task::none()
                    }
                    TabId::Channels => {
                        self.current_view = View::Channels;
                        // Load circular thumbnails for all subscriptions that aren't already loaded
                        let thumb_load_start = std::time::Instant::now();
                        let tasks: Vec<Task<Message>> = self
                            .config
                            .subscriptions
                            .iter()
                            .filter(|sub| !self.subscription_thumbs.contains_key(&sub.channel_id))
                            .map(|sub| {
                                let channel_id = sub.channel_id.clone();
                                let url = sub.thumbnail_url.clone();
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
                        eprintln!(
                            "    TabSelected: created {} thumbnail load tasks in {:?}",
                            tasks.len(),
                            thumb_load_start.elapsed()
                        );
                        Task::batch(tasks)
                    }
                    TabId::Settings => {
                        self.current_view = View::Config;
                        Task::none()
                    }
                };

                eprintln!(
                    "  TabSelected: update took {:?}",
                    _tab_switch_start.elapsed()
                );
                task
            }
            Message::NoOp => Task::none(),
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
                            Ok(filename) => {
                                eprintln!("Exported search results to: {}", filename);
                                Message::NoOp
                            }
                            Err(e) => {
                                eprintln!("Failed to export: {}", e);
                                Message::NoOp
                            }
                        },
                    )
                } else {
                    eprintln!("No search results to export");
                    Task::none()
                }
            }
        };

        let elapsed = _update_start.elapsed();
        if elapsed.as_micros() > 100 {
            eprintln!("  Update[{}]: took {:?}", msg_name, elapsed);
        }

        result
    }

    fn view(&self) -> Element<'_, Message> {
        // Track view call frequency
        static VIEW_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
        let call_num = VIEW_CALL_COUNT.fetch_add(1, Ordering::Relaxed);

        // Detect tab switches for profiling using interior mutability
        let last_view = self.last_view_for_timing.get();
        let is_tab_switch = last_view.as_ref() != Some(&self.current_view);

        eprintln!(
            "  VIEW: call #{}, current view: {:?}, {} search results, {} thumbs loaded",
            call_num,
            self.current_view,
            self.search_results.len(),
            self.thumbs.len()
        );
        let _view_start = if is_tab_switch {
            if let Some(last) = last_view {
                eprintln!(
                    "\n========== TAB SWITCH: {:?} -> {:?} ==========",
                    last, self.current_view
                );
            }
            self.last_view_for_timing
                .set(Some(self.current_view.clone()));
            Some(std::time::Instant::now())
        } else {
            None
        };

        use iced::Alignment;
        use iced::Length;
        use iced::widget::{button, column, container, row, text};

        // Create custom tab bar using buttons
        let search_tab = button(container(text("Search")).padding(10).center_x(Length::Fill))
            .width(Length::FillPortion(1))
            .style(if self.active_tab == TabId::Search {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::TabSelected(TabId::Search));

        let channels_tab = button(
            container(text("Channels"))
                .padding(10)
                .center_x(Length::Fill),
        )
        .width(Length::FillPortion(1))
        .style(if self.active_tab == TabId::Channels {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::TabSelected(TabId::Channels));

        let settings_tab = button(
            container(text("Settings"))
                .padding(10)
                .center_x(Length::Fill),
        )
        .width(Length::FillPortion(1))
        .style(if self.active_tab == TabId::Settings {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::TabSelected(TabId::Settings));

        let tab_bar = row![search_tab, channels_tab, settings_tab]
            .spacing(0)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        let content = match self.current_view {
            View::Search => views::search::view(self),
            View::Channel => views::channel::view(self, get_language_by_locale),
            View::Config => views::config::view(self),
            View::Channels => views::subscriptions::view(self),
        };

        let result = column![tab_bar, content].spacing(0).into();

        if let Some(start) = _view_start {
            eprintln!(
                "========== TAB SWITCH TOTAL: {:?} ==========\n",
                start.elapsed()
            );
        }

        result
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|ev, _status, _id| match ev {
            iced::Event::Window(iced::window::Event::Resized(Size { width, height })) => {
                Some(Message::Resized(width, height))
            }
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key: iced::keyboard::Key::Character(c),
                modifiers,
                ..
            }) if modifiers.control() && c.as_ref() == "e" => Some(Message::ExportSearchResults),
            _ => None,
        })
    }
}
