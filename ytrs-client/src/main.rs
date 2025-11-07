use iced::Alignment::Center;
use iced::widget::button::Style;
use iced::widget::{
    Image, button, column, combo_box, container, lazy, pick_list, row, scrollable, space, stack,
    text, text_input,
};
use iced::{Alignment, Element, Length, Task, Theme};
use iced_aw::Wrap;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, OnceLock};
use ytrs::{
    ChannelInfo, ChannelTab, ChannelVideos, InnerTube, LanguageOption, SearchResult, SearchResults,
    SortFilter, get_all_languages,
};

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
        .theme(cosmic_theme)
        .font(include_bytes!("../fonts/NotoSansCJK-VF.otf.ttc"))
        .default_font(iced::Font {
            family: iced::font::Family::Name("Noto Sans CJK JP"),
            ..iced::Font::DEFAULT
        })
        .run()
}

fn cosmic_title(_: &App) -> String {
    "ytrs".to_string()
}

fn cosmic_theme(_: &App) -> Theme {
    Theme::custom("Cosmic".to_string(), cosmic_palette())
}

fn cosmic_palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: iced::Color::from_rgb(0.08, 0.08, 0.12),
        text: iced::Color::from_rgb(0.95, 0.95, 0.98),
        primary: iced::Color::from_rgb(0.5, 0.4, 0.9),
        success: iced::Color::from_rgb(0.3, 0.8, 0.6),
        danger: iced::Color::from_rgb(0.9, 0.3, 0.4),
        warning: iced::Color::from_rgb(0.9, 0.7, 0.3),
    }
}

#[derive(Debug, Clone)]
enum View {
    Search,
    Channel,
}

#[derive(Debug, Clone)]
enum Message {
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
    LanguageSelected(LanguageOption),
}

struct App {
    query: String,
    results: Vec<SearchResult>, // Shared: search results or channel videos
    thumbs: HashMap<String, iced::widget::image::Handle>,
    searching: bool,
    continuation: Option<String>,     // Shared: continuation token
    current_locale: (String, String), // Shared: detected locale
    loading_more: bool,               // Shared: loading more results
    preload_count: usize,             // Shared: track preloading (3 pages)
    preloading: bool,                 // Shared: whether preloading
    current_view: View,
    current_channel: Option<ChannelInfo>,
    current_tab: ChannelTab,
    banner: Option<iced::widget::image::Handle>,
    loading_channel: bool,
    available_sort_filters: Vec<SortFilter>,
    selected_sort_label: Option<String>,
    language_combo_state: combo_box::State<LanguageOption>,
    selected_language: Option<LanguageOption>,
    playing_video: Option<String>, // Currently playing video ID
    countdown_value: u8,           // Current countdown value (5, 4, 3, 2, 1, 0)
    mpv_process: Arc<tokio::sync::Mutex<Option<std::process::Child>>>, // MPV process handle
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                query: String::new(),
                results: Vec::new(),
                thumbs: HashMap::new(),
                searching: false,
                continuation: None,
                current_locale: ("en".to_string(), "GB".to_string()),
                loading_more: false,
                preload_count: 0,
                preloading: false,
                current_view: View::Search,
                current_channel: None,
                current_tab: ChannelTab::Videos,
                banner: None,
                loading_channel: false,
                available_sort_filters: Vec::new(),
                selected_sort_label: None,
                language_combo_state: combo_box::State::new(get_all_languages().to_vec()),
                selected_language: None,
                playing_video: None,
                countdown_value: 0,
                mpv_process: Arc::new(tokio::sync::Mutex::new(None)),
            },
            Task::none(),
        )
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
                self.results.clear();
                self.continuation = None;
                self.preload_count = 0;
                self.preloading = true;
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
                        let is_load_more = self.loading_more;
                        self.loading_more = false;

                        // Store the new results to load thumbnails for
                        let new_results = search_results.results;

                        // Store continuation token for pagination
                        self.continuation = search_results.continuation;

                        // Store detected locale only if no manual language is selected
                        if self.selected_language.is_none() {
                            if let Some(locale) = search_results.detected_locale {
                                self.current_locale = locale;
                            }
                        }

                        // Update results (replace on first search, append on continuation/preload)
                        if !is_load_more && self.results.is_empty() {
                            self.results = new_results.clone();
                            self.thumbs.clear();
                        } else {
                            // Appending results (load more or preloading)
                            self.results.extend(new_results.clone());
                        }

                        // Auto-preload: fetch 3 pages (90 results) before showing content
                        const TARGET_PRELOAD_PAGES: usize = 3;

                        if self.preloading {
                            self.preload_count += 1;

                            // If we haven't reached target and have continuation, auto-load more
                            if self.preload_count < TARGET_PRELOAD_PAGES
                                && self.continuation.is_some()
                            {
                                let token = self.continuation.as_ref().unwrap().clone();
                                let (hl, gl) = self.current_locale.clone();

                                // Start loading thumbnails for current batch while fetching next page
                                let thumb_tasks = create_thumbnail_tasks(&new_results);

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
                                self.preloading = false;
                                self.searching = false;
                                self.loading_more = false;
                            }
                        }

                        // Load thumbnails ONLY for the new results (not all results)
                        Task::batch(create_thumbnail_tasks(&new_results))
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        self.preloading = false;
                        self.searching = false;
                        self.loading_more = false;
                        Task::none()
                    }
                }
            }
            Message::ThumbLoaded(id, res) => {
                if let Ok(bytes) = res {
                    self.thumbs
                        .insert(id, iced::widget::image::Handle::from_bytes(bytes));
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
                self.banner = None;
                self.results.clear();
                self.current_tab = ChannelTab::Videos;
                self.available_sort_filters.clear();
                self.selected_sort_label = None;
                self.continuation = None;
                self.preload_count = 0;
                self.preloading = true;
                // Keep selected_language if it exists (persist from search view)

                let id = channel_id.clone();

                // Use manual locale if selected, otherwise let channel load detect it
                if let Some(ref language) = self.selected_language {
                    let hl = language.hl.to_string();
                    let gl = language.gl.to_string();
                    self.current_locale = (hl, gl);
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
                                    async move { load_thumb(&url).await.map_err(|e| e.to_string()) },
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
                                async move { load_thumb(&url).await.map_err(|e| e.to_string()) },
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
                        let is_load_more = self.loading_more;
                        self.loading_more = false;

                        // Store the new videos to load thumbnails for
                        let new_videos = videos.videos;

                        // Update videos (append if load more, replace otherwise)
                        if is_load_more || self.preloading {
                            self.results.extend(new_videos.clone());
                        } else {
                            self.results = new_videos.clone();
                        }

                        // Store continuation token for pagination
                        self.continuation = videos.continuation;

                        // Store detected locale only if no manual language is selected
                        if self.selected_language.is_none() {
                            if let Some(locale) = videos.detected_locale {
                                self.current_locale = locale;
                            }
                        }

                        // Update sort filters if available
                        if let Some(filters) = videos.sort_filters {
                            self.selected_sort_label = filters
                                .iter()
                                .find(|f| f.is_selected)
                                .map(|f| f.label.clone());
                            self.available_sort_filters = filters;
                        }

                        // Auto-preload: fetch 3 pages (90 videos) before showing content
                        const TARGET_PRELOAD_PAGES: usize = 3;

                        if self.preloading {
                            self.preload_count += 1;

                            // If we haven't reached target and have continuation, auto-load more
                            if self.preload_count < TARGET_PRELOAD_PAGES
                                && self.continuation.is_some()
                            {
                                let token = self.continuation.as_ref().unwrap().clone();
                                let (hl, gl) = self.current_locale.clone();

                                // Start loading thumbnails for current batch while fetching next page
                                let thumb_tasks = create_thumbnail_tasks(&new_videos);

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
                                self.preloading = false;
                                self.loading_channel = false;
                                self.loading_more = false;
                            }
                        }

                        // Load thumbnails ONLY for the new videos (not all videos)
                        Task::batch(create_thumbnail_tasks(&new_videos))
                    }
                    Err(e) => {
                        eprintln!("Error loading channel videos: {}", e);
                        self.preloading = false;
                        self.loading_channel = false;
                        self.loading_more = false;
                        Task::none()
                    }
                }
            }
            Message::ChangeChannelTab(tab) => {
                if let Some(ref channel) = self.current_channel {
                    self.current_tab = tab;
                    self.results.clear();
                    self.available_sort_filters.clear();
                    self.selected_sort_label = None;
                    self.continuation = None;
                    self.preload_count = 0;
                    self.preloading = true;
                    self.loading_channel = true;

                    let channel_id = channel.id.clone();
                    // Use stored locale for consistent results across tabs
                    let (hl, gl) = self.current_locale.clone();
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
                    self.results.clear();
                    self.continuation = None; // Will be updated with new continuation
                    self.preload_count = 0;
                    self.preloading = true;
                    self.loading_channel = true;

                    let token = token.clone();
                    let (hl, gl) = self.current_locale.clone();
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
                if let Some(ref token) = self.continuation
                    && !self.loading_more
                {
                    self.loading_more = true;
                    // Enable preloading to fetch 3 more pages
                    self.preload_count = 0;
                    self.preloading = true;

                    let token = token.clone();
                    // Use stored locale for consistent results
                    let (hl, gl) = self.current_locale.clone();
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
                if let Some(ref token) = self.continuation
                    && !self.loading_more
                {
                    self.loading_more = true;
                    // Enable preloading to fetch 3 more pages
                    self.preload_count = 0;
                    self.preloading = true;

                    let token = token.clone();
                    // Use stored locale for consistent results
                    let (hl, gl) = self.current_locale.clone();
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
            Message::BackToSearch => {
                self.current_view = View::Search;
                self.current_channel = None;
                self.results.clear();
                self.banner = None;
                self.available_sort_filters.clear();
                self.selected_sort_label = None;
                self.continuation = None;
                Task::none()
            }
            Message::LanguageSelected(language) => {
                self.selected_language = Some(language.clone());
                let hl = language.hl.to_string();
                let gl = language.gl.to_string();

                // Update current_locale to the manually selected language
                self.current_locale = (hl.clone(), gl.clone());

                match self.current_view {
                    View::Channel => {
                        // Re-fetch channel with this locale
                        if let Some(ref channel) = self.current_channel {
                            self.results.clear();
                            self.continuation = None;
                            self.preload_count = 0;
                            self.preloading = true;
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
                    View::Search => {
                        // Re-run search with new locale if there's an active query
                        if !self.query.is_empty() && !self.searching {
                            self.searching = true;
                            self.results.clear();
                            self.continuation = None;
                            self.preload_count = 0;
                            self.preloading = true;
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
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self.current_view {
            View::Search => self.view_search(),
            View::Channel => self.view_channel(),
        }
    }

    fn view_search(&self) -> Element<'_, Message> {
        let search_row = row![
            text_input("Search YouTube...", &self.query)
                .on_input(Message::InputChanged)
                .on_submit(Message::Search)
                .padding(10)
                .width(Length::Fill),
            button(text("Search")).on_press(Message::Search).padding(10)
        ]
        .spacing(10);

        let language_row = row![
            text("Language:").size(14),
            combo_box(
                &self.language_combo_state,
                "Auto-detect",
                self.selected_language.as_ref(),
                Message::LanguageSelected,
            )
            .width(250)
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let search = container(column![
            row![
                column![
                    iced::widget::space::vertical(),
                    search_row,
                    iced::widget::space::vertical()
                ]
                .spacing(10),
                column![
                    iced::widget::space::vertical(),
                    language_row,
                    iced::widget::space::vertical()
                ],
            ]
            .spacing(20)
            .padding(20),
        ])
        .height(100)
        .width(Length::Fill);

        let body: Element<Message> = if self.results.is_empty() {
            if self.searching {
                container(text("Searching...")).padding(40).into()
            } else {
                container(
                    column![
                        text("ytrs").size(40),
                        text("YouTube for polyglots").size(14)
                    ]
                    .spacing(10)
                    .align_x(Alignment::Center),
                )
                .padding(60)
                .center_x(Length::FillPortion(1))
                .into()
            }
        } else {
            let cards: Vec<Element<Message>> = self
                .results
                .iter()
                .filter_map(|r| {
                    let vid = r.video_id.clone()?;

                    // Only render videos if thumbnail is loaded
                    let h = self.thumbs.get(&vid)?.clone();

                    // Clone all data for lazy closure (must be owned)
                    let view_count = r.view_count;
                    let duration = r.duration.clone();
                    let title = r.title.clone();
                    let channel = r.channel.clone();
                    let is_playing = self.playing_video.as_ref() == Some(&vid);
                    let countdown = self.countdown_value;

                    // Lazy widget caches rendering - only rebuilds when (vid, is_playing, countdown) changes
                    Some(
                        lazy((vid.clone(), is_playing, countdown), move |_| {
                            let thumb = Image::new(h.clone()).width(240).height(135);

                            // Create thumbnail with optional countdown overlay
                            let thumb_with_overlay: Element<Message> = if is_playing {
                                stack![
                                    thumb,
                                    // Gray overlay
                                    container(space()).width(240).height(135).style(
                                        |_theme: &Theme| container::Style {
                                            background: Some(iced::Background::Color(
                                                iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6)
                                            )),
                                            ..Default::default()
                                        }
                                    ),
                                    // Countdown text
                                    container(
                                        column![
                                            text("Waiting for required preload time")
                                                .size(12)
                                                .color(iced::Color::WHITE),
                                            text(countdown.to_string())
                                                .size(48)
                                                .color(iced::Color::WHITE)
                                        ]
                                        .align_x(Alignment::Center)
                                        .spacing(5)
                                    )
                                    .width(240)
                                    .height(135)
                                    .center_x(240)
                                    .center_y(135)
                                ]
                                .into()
                            } else {
                                thumb.into()
                            };

                            // Build metadata line
                            let mut meta_parts = vec![];
                            if let Some(v) = view_count {
                                meta_parts.push(format!("{} views", fmt_num(v)));
                            }
                            if let Some(ref d) = duration {
                                meta_parts.push(d.clone());
                            }

                            // Create info section with title and metadata
                            let full_title = title.clone();
                            let display_title = truncate_title(&title, 25);

                            let title_widget = iced::widget::tooltip(
                                text(display_title).size(14),
                                container(text(full_title))
                                    .style(container::dark)
                                    .padding(10),
                                iced::widget::tooltip::Position::FollowCursor,
                            );

                            let mut info_col = column![title_widget];

                            // Add clickable channel name if available
                            if let Some(ref ch) = channel {
                                if let Some(ref cid) = ch.id {
                                    info_col = info_col.push(
                                        // users.rust-lang.org/t/how-to-make-a-static-str-from-a-variable/53718/15
                                        // Leaking memory here is done to make the channel name have a 'static lifetime
                                        // This allows us to 'cache' the video tiles, improving performance drastically.
                                        // However, the downside being that memory is not regained before exiting the application.
                                        // This is probably acceptable for normal use, but there may be a better way of doing this.
                                        button(&*Box::leak(ch.name.clone().into_boxed_str()))
                                            .style(|theme: &Theme, status| match status {
                                                button::Status::Active => Style {
                                                    text_color: theme.palette().text,
                                                    ..Default::default()
                                                },
                                                button::Status::Hovered => Style {
                                                    text_color: theme.palette().success,
                                                    ..Default::default()
                                                },
                                                button::Status::Pressed => Style {
                                                    text_color: theme.palette().text,
                                                    ..Default::default()
                                                },
                                                button::Status::Disabled => Style {
                                                    text_color: theme.palette().background,
                                                    ..Default::default()
                                                },
                                            })
                                            .padding(0)
                                            .on_press(Message::ViewChannel(cid.clone())),
                                    );
                                } else {
                                    info_col = info_col
                                        .push(text(&*Box::leak(ch.name.clone().into_boxed_str())));
                                }
                            }

                            // Add metadata line if we have any
                            if !meta_parts.is_empty() {
                                info_col = info_col.push(text(meta_parts.join(" • ")).size(12));
                            }

                            let card = column![
                                thumb_with_overlay,
                                container(info_col.spacing(4))
                                    .padding(8)
                                    .width(240)
                                    .height(Length::Fixed(100.0))
                            ]
                            .spacing(0)
                            .width(240);

                            button(card).on_press(Message::Play(vid.clone())).padding(0)
                        })
                        .into(),
                    )
                })
                .collect();

            let mut search_content = column![
                container(Wrap::with_elements(cards).spacing(15.0).line_spacing(15.0))
                    .center_x(Length::Fill)
            ]
            .align_x(Alignment::Center);

            // Show "Load More" button or loading indicator
            if self.loading_more {
                let loading_indicator = container(text("Loading more...").size(14))
                    .padding(20)
                    .center_x(Length::Fill);
                search_content = search_content.push(loading_indicator);
            } else if self.continuation.is_some() {
                // Show "Load More" button if we have more results to load
                let load_more_btn = container(
                    button(text("Load More Results"))
                        .on_press(Message::LoadMoreSearchResults)
                        .padding(10),
                )
                .padding(20)
                .center_x(Length::Fill);
                search_content = search_content.push(load_more_btn);
            }

            scrollable(container(search_content).padding(20).width(Length::Fill)).into()
        };

        column![search, body].into()
    }

    fn view_channel(&self) -> Element<'_, Message> {
        if let Some(ref channel) = self.current_channel {
            let mut content = column![].spacing(0);

            // Banner with header overlay
            let banner_image: Element<Message> = if let Some(ref banner_handle) = self.banner {
                Image::new(banner_handle.clone())
                    .width(Length::Fill)
                    .height(200)
                    .into()
            } else {
                // Placeholder banner
                container(iced::widget::space::horizontal())
                    .width(Length::Fill)
                    .height(200)
                    .style(|theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(theme.palette().primary)),
                        ..Default::default()
                    })
                    .into()
            };

            content = content.push(banner_image);

            // Back button, avatar, and channel info on same row
            let avatar: Element<Message> = if let Some(h) = self.thumbs.get(&channel.id) {
                Image::new(h.clone()).width(80).height(80).into()
            } else {
                container(space()).width(80).height(80).into()
            };

            let mut info_column = column![text(&channel.name).size(24),].spacing(5);

            if let Some(ref subs) = channel.subscriber_count {
                info_column = info_column.push(text(subs).size(14));
            }

            let header = row![
                button(text("← Back"))
                    .on_press(Message::BackToSearch)
                    .padding(10),
                avatar,
                info_column.padding(10),
            ]
            .spacing(10)
            .align_y(Alignment::Center);

            // Tabs
            let tabs = row![
                button(text("VIDEOS"))
                    .on_press(Message::ChangeChannelTab(ChannelTab::Videos))
                    .padding(10),
                button(text("SHORTS"))
                    .on_press(Message::ChangeChannelTab(ChannelTab::Shorts))
                    .padding(10),
                button(text("LIVE"))
                    .on_press(Message::ChangeChannelTab(ChannelTab::Streams))
                    .padding(10),
            ]
            .spacing(10);

            // Language and Sort controls on the same row
            // Find the auto-detected language name to display in placeholder (O(1) HashMap lookup)
            let auto_detected_name =
                get_language_by_locale(&self.current_locale.0, &self.current_locale.1)
                    .map(|lang| lang.name)
                    .unwrap_or("Unknown");

            let placeholder = format!("Auto-detected: {}", auto_detected_name);

            let mut controls_row = row![
                text("Language:").size(14),
                combo_box(
                    &self.language_combo_state,
                    &placeholder,
                    self.selected_language.as_ref(),
                    Message::LanguageSelected,
                )
                .width(250)
            ]
            .align_y(Center)
            .spacing(10);

            // Add sort dropdown if we have sort filters available
            if !self.available_sort_filters.is_empty() {
                let filter_labels: Vec<String> = self
                    .available_sort_filters
                    .iter()
                    .map(|f| f.label.clone())
                    .collect();

                controls_row = controls_row.push(
                    row![
                        text("Sort by:").size(14),
                        pick_list(
                            filter_labels,
                            self.selected_sort_label.clone(),
                            Message::ChangeSortFilter,
                        )
                        .padding(5)
                    ]
                    .spacing(10)
                    .padding(10)
                    .align_y(Alignment::Center),
                );
            }

            // Add controls section with background and 2px bottom border
            let controls_with_border = column![
                container(row![
                    column![header, tabs].spacing(10).width(Length::Fill),
                    column![iced::widget::space::vertical(), controls_row]
                ])
                .padding(10)
                .height(150)
                .width(Length::Fill)
                .style(|theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(theme.palette().background)),
                    ..Default::default()
                }),
                // 2px bottom border line
                container(space())
                    .width(Length::Fill)
                    .height(2)
                    .style(|theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(theme.palette().primary)),
                        ..Default::default()
                    })
            ]
            .spacing(0);

            content = content.push(controls_with_border);

            // Videos grid
            let video_cards: Vec<Element<Message>> =
                self.results
                    .iter()
                    .filter_map(|r| {
                        let vid = r.video_id.as_ref()?;
                        let h = self.thumbs.get(vid)?;

                        let thumb = Image::new(h.clone()).width(240).height(135);

                        // Check if this video is currently playing
                        let is_playing = self.playing_video.as_ref() == Some(vid);
                        let countdown = self.countdown_value;

                        // Create thumbnail with optional countdown overlay
                        let thumb_with_overlay: Element<Message> = if is_playing {
                            stack![
                                thumb,
                                // Gray overlay
                                container(space()).width(240).height(135).style(
                                    |_theme: &Theme| container::Style {
                                        background: Some(iced::Background::Color(
                                            iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6)
                                        )),
                                        ..Default::default()
                                    }
                                ),
                                // Countdown text
                                container(
                                    column![
                                        text("Waiting for required preload time")
                                            .size(12)
                                            .color(iced::Color::WHITE),
                                        text(countdown.to_string())
                                            .size(48)
                                            .color(iced::Color::WHITE)
                                    ]
                                    .align_x(Alignment::Center)
                                    .spacing(5)
                                )
                                .width(240)
                                .height(135)
                                .center_x(240)
                                .center_y(135)
                            ]
                            .into()
                        } else {
                            thumb.into()
                        };

                        let mut meta = vec![];
                        if let Some(v) = r.view_count {
                            meta.push(format!("{} views", fmt_num(v)));
                        }
                        if let Some(ref d) = r.duration {
                            meta.push(d.clone());
                        }
                        if let Some(ref p) = r.published_text {
                            meta.push(p.clone());
                        }

                        let full_title = r.title.clone();
                        let display_title = truncate_title(&r.title, 25);

                        let title_widget = iced::widget::tooltip(
                            text(display_title).size(14),
                            container(text(full_title))
                                .style(container::dark)
                                .padding(10),
                            iced::widget::tooltip::Position::FollowCursor,
                        );

                        let card = column![
                            thumb_with_overlay,
                            container(
                                column![title_widget, text(meta.join(" • ")).size(12),].spacing(4)
                            )
                            .padding(8)
                            .width(240)
                            .height(Length::Fixed(100.0))
                        ]
                        .spacing(0)
                        .width(240);

                        let v = vid.clone();
                        Some(button(card).on_press(Message::Play(v)).padding(0).into())
                    })
                    .collect();

            let videos_section: Element<Message> = if video_cards.is_empty() {
                if self.loading_channel {
                    container(text("Loading..."))
                        .padding(40)
                        .center_x(Length::Fill)
                        .into()
                } else {
                    container(text("No videos found"))
                        .padding(40)
                        .center_x(Length::Fill)
                        .into()
                }
            } else {
                let mut video_content = column![
                    container(
                        Wrap::with_elements(video_cards)
                            .spacing(15.0)
                            .line_spacing(15.0),
                    )
                    .center_x(Length::Fill)
                    .align_x(Alignment::Center)
                ];

                // Show "Load More" button or loading indicator
                if self.loading_more {
                    let loading_indicator = container(text("Loading more...").size(14))
                        .padding(20)
                        .center_x(Length::Fill);
                    video_content = video_content.push(loading_indicator);
                } else if self.continuation.is_some() {
                    // Show "Load More" button if we have more videos to load
                    let load_more_btn = container(
                        button(text("Load More Videos"))
                            .on_press(Message::LoadMoreVideos)
                            .padding(10),
                    )
                    .padding(20)
                    .center_x(Length::Fill);
                    video_content = video_content.push(load_more_btn);
                }

                scrollable(container(video_content).padding(20)).into()
            };

            content = content.push(videos_section);

            content.into()
        } else {
            container(text("Loading channel...")).padding(40).into()
        }
    }
}

async fn load_thumb(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let r = reqwest::get(url).await?;
    let b = r.bytes().await?;
    Ok(b.to_vec())
}

/// Helper function to truncate title text with ellipsis
fn truncate_title(title: &str, max_chars: usize) -> String {
    if title.chars().count() > max_chars {
        format!(
            "{}...",
            title.chars().take(max_chars - 3).collect::<String>()
        )
    } else {
        title.to_string()
    }
}

/// Helper function to create thumbnail loading tasks for search results
fn create_thumbnail_tasks(results: &[SearchResult]) -> Vec<Task<Message>> {
    results
        .iter()
        .filter_map(|r| {
            // Load video thumbnails
            if let Some(vid) = r.video_id.as_ref() {
                r.thumbnails.first().map(|t| {
                    let id = vid.clone();
                    let url = t.url.clone();
                    Task::perform(
                        async move { load_thumb(&url).await.map_err(|e| e.to_string()) },
                        move |res| Message::ThumbLoaded(id.clone(), res),
                    )
                })
            }
            // Load channel thumbnails
            else if let Some(channel) = r.channel.as_ref() {
                if let Some(cid) = channel.id.as_ref() {
                    r.thumbnails.first().map(|t| {
                        let id = cid.clone();
                        let url = t.url.clone();
                        Task::perform(
                            async move { load_thumb(&url).await.map_err(|e| e.to_string()) },
                            move |res| Message::ThumbLoaded(id.clone(), res),
                        )
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

fn fmt_num(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}
