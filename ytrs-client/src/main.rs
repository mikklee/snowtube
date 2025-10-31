use iced::widget::{Image, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Task, Theme};
use iced_aw::Wrap;
use std::collections::HashMap;
use std::process::Command;
use ytrs::{InnerTube, SearchResult};

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
enum Message {
    InputChanged(String),
    Search,
    SearchDone(Result<Vec<SearchResult>, String>),
    ThumbLoaded(String, Result<Vec<u8>, String>),
    Play(String),
}

struct App {
    query: String,
    results: Vec<SearchResult>,
    thumbs: HashMap<String, iced::widget::image::Handle>,
    searching: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                query: String::new(),
                results: Vec::new(),
                thumbs: HashMap::new(),
                searching: false,
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
                let q = self.query.clone();
                Task::perform(
                    async move {
                        let client = InnerTube::new().await.map_err(|e| e.to_string())?;
                        client.search(&q).await.map_err(|e| e.to_string())
                    },
                    Message::SearchDone,
                )
            }
            Message::SearchDone(res) => {
                self.searching = false;
                match res {
                    Ok(mut r) => {
                        // Create a channel result from the first video's channel info
                        if let Some(first_video) = r.first() {
                            if let Some(channel) = &first_video.channel {
                                // Create a channel "result" to display first
                                let channel_result = SearchResult {
                                    video_id: None,
                                    title: channel.name.clone(),
                                    description: None,
                                    channel: Some(channel.clone()),
                                    view_count: None,
                                    duration: None,
                                    published_text: None,
                                    thumbnails: channel.thumbnail.clone().unwrap_or_default(),
                                };
                                // Insert channel at the beginning
                                r.insert(0, channel_result);
                            }
                        }
                        self.results = r;
                        self.thumbs.clear();
                        let tasks: Vec<_> = self
                            .results
                            .iter()
                            .filter_map(|r| {
                                // Load video thumbnails
                                if let Some(vid) = r.video_id.as_ref() {
                                    r.thumbnails.first().map(|t| {
                                        let id = vid.clone();
                                        let url = t.url.clone();
                                        Task::perform(
                                            async move {
                                                load_thumb(&url).await.map_err(|e| e.to_string())
                                            },
                                            move |r| Message::ThumbLoaded(id.clone(), r),
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
                                                async move {
                                                    load_thumb(&url).await.map_err(|e| e.to_string())
                                                },
                                                move |r| Message::ThumbLoaded(id.clone(), r),
                                            )
                                        })
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect();
                        Task::batch(tasks)
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
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
            Message::Play(id) => {
                let url = format!("https://www.youtube.com/watch?v={}", id);
                std::thread::spawn(move || {
                    let _ = Command::new("mpv")
                        .arg(&url)
                        .arg("--ytdl=yes")
                        .arg("--script-opts=ytdl_hook-ytdl_path=yt-dlp")
                        .spawn();
                });
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let search = row![
            text_input("Search YouTube...", &self.query)
                .on_input(Message::InputChanged)
                .on_submit(Message::Search)
                .padding(10)
                .width(Length::FillPortion(8)),
            button(text("Search")).on_press(Message::Search).padding(10)
        ]
        .spacing(10)
        .padding(20);

        let body: Element<Message> = if self.results.is_empty() {
            if self.searching {
                container(text("Searching...")).padding(40).into()
            } else {
                container(
                    column![
                        text("ytrs").size(40),
                        text("YouTube search without API keys").size(14)
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
                    // Handle channel results (no video_id)
                    if r.video_id.is_none() {
                        // This is a channel result - check if we have the thumbnail loaded
                        let channel_id = r.channel.as_ref().and_then(|c| c.id.as_ref());

                        let thumb_widget: Element<Message> = if let Some(cid) = channel_id {
                            if let Some(h) = self.thumbs.get(cid) {
                                Image::new(h.clone()).width(240).height(135).into()
                            } else {
                                // Thumbnail not loaded yet, show placeholder
                                container(text(&r.title).size(16))
                                    .padding(20)
                                    .width(240)
                                    .height(135)
                                    .into()
                            }
                        } else {
                            // No channel id, show text
                            container(text(&r.title).size(16))
                                .padding(20)
                                .width(240)
                                .height(135)
                                .into()
                        };

                        let card = column![
                            thumb_widget,
                            container(
                                text("Channel")
                                    .size(12)
                            )
                            .padding(8)
                            .width(240)
                            .height(Length::Fixed(80.0))
                        ]
                        .spacing(0)
                        .width(240);

                        return Some(container(card).padding(0).into());
                    }

                    let vid = r.video_id.as_ref()?;

                    // Only render videos if thumbnail is loaded
                    let h = self.thumbs.get(vid)?;

                    let thumb: Element<Message> =
                        Image::new(h.clone()).width(240).height(135).into();

                    let mut meta = vec![];
                    if let Some(ref ch) = r.channel {
                        meta.push(ch.name.clone());
                    }
                    if let Some(v) = r.view_count {
                        meta.push(format!("{} views", fmt_num(v)));
                    }
                    if let Some(ref d) = r.duration {
                        meta.push(d.clone());
                    }

                    let card = column![
                        thumb,
                        container(
                            column![
                                text(&r.title).size(14).shaping(text::Shaping::Advanced),
                                text(meta.join(" • "))
                                    .size(12)
                                    .shaping(text::Shaping::Advanced),
                            ]
                            .spacing(4)
                        )
                        .padding(8)
                        .width(240)
                        .height(Length::Fixed(80.0))
                    ]
                    .spacing(0)
                    .width(240);

                    let v = vid.clone();
                    Some(button(card).on_press(Message::Play(v)).padding(0).into())
                })
                .collect();

            scrollable(
                container(Wrap::with_elements(cards).spacing(15.0).line_spacing(15.0))
                    .padding(20)
                    .width(Length::Fill),
            )
            .into()
        };

        column![search, body].into()
    }
}

async fn load_thumb(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let r = reqwest::get(url).await?;
    let b = r.bytes().await?;
    Ok(b.to_vec())
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
