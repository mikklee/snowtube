//! Helper functions for the ytrs-client UI

use iced::{
    Alignment, Color, Element, Task, Theme,
    widget::{Image, column, container, stack, text},
};
use ytrs_lib::SearchResult;

use crate::messages::Message;

/// Load thumbnail from URL
pub async fn load_thumb(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let r = reqwest::get(url).await?;
    let b = r.bytes().await?;
    Ok(b.to_vec())
}

/// Helper function to truncate title text with ellipsis
pub fn truncate_title(title: &str, max_chars: usize) -> String {
    if title.chars().count() > max_chars {
        format!(
            "{}...",
            title.chars().take(max_chars - 3).collect::<String>()
        )
    } else {
        title.to_string()
    }
}

/// Helper function to create a thumbnail element.
/// If a video has been clicked, displays a 5-second countdown overlay
/// with a gray background and "Waiting for required preload time" message.
/// YouTube requires a 5-second preload time before MPV can start playing the video.
pub fn create_thumbnail(
    thumb: Image<iced::widget::image::Handle>,
    is_playing: bool,
    countdown: u8,
) -> Element<'static, Message> {
    if is_playing {
        stack![
            thumb,
            // Gray overlay
            container(iced::widget::space())
                .width(240)
                .height(135)
                .style(|_theme: &Theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        0.0, 0.0, 0.0, 0.6
                    ))),
                    ..Default::default()
                }),
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
    }
}

/// Helper function to create thumbnail loading tasks for search results
pub fn create_thumbnail_tasks(results: &[SearchResult]) -> Vec<Task<Message>> {
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

/// Format large numbers with K/M/B suffixes
pub fn fmt_num(n: u64) -> String {
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

/// Channel info for video tiles
pub struct ChannelInfo {
    pub name: &'static str,
    pub on_press: Option<Message>,
}

/// Create a video tile with thumbnail, title, channel, and metadata
pub fn create_video_tile<'a>(
    thumbnail: Element<'a, Message>,
    title_text: &str,
    channel: Option<ChannelInfo>,
    metadata_text: Option<String>,
    on_press: Message,
) -> Element<'a, Message> {
    use iced::{
        Length,
        widget::{button, column, container, text, tooltip},
    };

    // Create title with tooltip
    let full_title = title_text.to_string();
    let display_title = truncate_title(title_text, 25);

    let title_widget = tooltip(
        text(display_title).size(14),
        container(text(full_title))
            .style(container::dark)
            .padding(10),
        tooltip::Position::FollowCursor,
    );

    let mut info_col = column![title_widget];

    // Add channel if provided
    if let Some(ch) = channel {
        if let Some(msg) = ch.on_press {
            info_col = info_col.push(
                button(ch.name)
                    .style(|theme: &Theme, status| match status {
                        button::Status::Active => match theme {
                            // For some of the themes the text ends up blending with the background.
                            // So, we have to override the text_color.
                            Theme::SolarizedDark
                            | Theme::SolarizedLight
                            | Theme::TokyoNightStorm
                            | Theme::TokyoNight => button::Style {
                                text_color: Color::WHITE,
                                ..Default::default()
                            },
                            _other => button::Style {
                                text_color: theme.palette().text,
                                ..Default::default()
                            },
                        },
                        button::Status::Hovered => button::Style {
                            text_color: theme.palette().success,
                            ..Default::default()
                        },
                        button::Status::Pressed => button::Style {
                            text_color: theme.palette().text,
                            ..Default::default()
                        },
                        button::Status::Disabled => button::Style {
                            text_color: theme.palette().background,
                            ..Default::default()
                        },
                    })
                    .padding(0)
                    .on_press(msg),
            );
        } else {
            info_col = info_col.push(text(ch.name));
        }
    }

    // Add metadata if provided
    if let Some(meta) = metadata_text {
        info_col = info_col.push(text(meta).size(12));
    }

    let card = column![
        thumbnail,
        container(info_col.spacing(4))
            .padding(8)
            .width(240)
            .height(Length::Fixed(100.0))
    ]
    .spacing(0)
    .width(240);

    button(card).on_press(on_press).padding(0).into()
}
