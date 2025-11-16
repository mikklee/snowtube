//! Helper functions for the ytrs-client UI

use iced::{
    Alignment, Element, Task, Theme,
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
