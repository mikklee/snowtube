//! Helper functions for the ytrs-client UI

use iced::Padding;
use iced::{
    Alignment, Color, Element, Task, Theme,
    widget::{Image, column, container, stack, text},
};
use std::path::PathBuf;
use ytrs_lib::SearchResult;

use crate::messages::Message;

/// Get the cache directory for images
fn get_cache_dir() -> Result<PathBuf, String> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| "Could not determine cache directory".to_string())?
        .join("ytrs")
        .join("thumbnails");
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    Ok(cache_dir)
}

/// Generate a cache key from URL
fn url_to_cache_key(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Load thumbnail from URL with disk caching
pub async fn load_thumb(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Try to load from cache first
    if let Ok(cache_dir) = get_cache_dir() {
        let cache_key = url_to_cache_key(url);
        let cache_path = cache_dir.join(&cache_key);

        if cache_path.exists() {
            if let Ok(bytes) = tokio::fs::read(&cache_path).await {
                return Ok(bytes);
            }
        }
    }

    // Download from URL
    let r = reqwest::get(url).await?;
    let b = r.bytes().await?;
    let bytes = b.to_vec();

    // Save to cache
    if let Ok(cache_dir) = get_cache_dir() {
        let cache_key = url_to_cache_key(url);
        let cache_path = cache_dir.join(&cache_key);
        let _ = tokio::fs::write(&cache_path, &bytes).await;
    }

    Ok(bytes)
}

/// Load thumbnail and make it circular with disk caching
pub async fn load_circular_thumb(
    url: &str,
    size: u32,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

    // Try to load from cache first (circular version)
    if let Ok(cache_dir) = get_cache_dir() {
        let cache_key = format!("{}_circular_{}", url_to_cache_key(url), size);
        let cache_path = cache_dir.join(&cache_key);

        if cache_path.exists() {
            if let Ok(bytes) = tokio::fs::read(&cache_path).await {
                return Ok(bytes);
            }
        }
    }

    // Download from URL
    let r = reqwest::get(url).await?;
    let bytes = r.bytes().await?;

    // Load image
    let img = image::load_from_memory(&bytes)?;

    // Resize to square
    let img = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);

    // Create circular mask
    let mut output = ImageBuffer::new(size, size);
    let center = size as f32 / 2.0;
    let radius = center;

    for (x, y, pixel) in output.enumerate_pixels_mut() {
        let dx = x as f32 - center;
        let dy = y as f32 - center;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance <= radius {
            let img_pixel = img.get_pixel(x, y);
            *pixel = Rgba([img_pixel[0], img_pixel[1], img_pixel[2], img_pixel[3]]);
        } else {
            *pixel = Rgba([0, 0, 0, 0]); // Transparent
        }
    }

    // Encode back to PNG
    let mut buf = Vec::new();
    DynamicImage::ImageRgba8(output)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)?;

    // Save to cache
    if let Ok(cache_dir) = get_cache_dir() {
        let cache_key = format!("{}_circular_{}", url_to_cache_key(url), size);
        let cache_path = cache_dir.join(&cache_key);
        let _ = tokio::fs::write(&cache_path, &buf).await;
    }

    Ok(buf)
}

/// Calculate horizontal padding to center a grid of items.
///
/// Given the window width, item width, and spacing between items,
/// calculates the left/right padding needed to center the grid.
/// This achieves a CSS `margin: 0 auto` effect for wrapped grids.
pub fn centered_grid_padding(
    window_width: f32,
    item_width: f32,
    spacing: f32,
    min_padding: f32,
    top: f32,
    bottom: f32,
) -> Padding {
    let available_width = window_width - (min_padding * 2.0);
    let items_per_row = ((available_width + spacing) / (item_width + spacing)).floor() as u32;
    let items_per_row = items_per_row.max(1);
    let content_width =
        (items_per_row as f32 * item_width) + ((items_per_row - 1) as f32 * spacing);
    let side_padding = ((window_width - content_width) / 2.0).max(min_padding);

    Padding {
        top,
        bottom,
        left: side_padding,
        right: side_padding,
    }
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

/// Truncate title with different limits for CJK vs non-CJK text
pub fn truncate_title_smart(title: &str, cjk_limit: usize, non_cjk_limit: usize) -> String {
    let limit = if ytrs_lib::contains_asian_characters(title) {
        cjk_limit
    } else {
        non_cjk_limit
    };
    truncate_title(title, limit)
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
/// All thumbnails are loaded in parallel using tokio::spawn
pub fn create_thumbnail_tasks(results: &[SearchResult]) -> Vec<Task<Message>> {
    let thumb_data: Vec<(String, String)> = results
        .iter()
        .filter_map(|r| {
            // Load video thumbnails
            if let Some(vid) = r.video_id.as_ref() {
                r.thumbnails.first().map(|t| (vid.clone(), t.url.clone()))
            }
            // Load channel thumbnails
            else if let Some(channel) = r.channel.as_ref() {
                if let Some(cid) = channel.id.as_ref() {
                    r.thumbnails.first().map(|t| (cid.clone(), t.url.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Spawn ALL downloads in parallel
    thumb_data
        .into_iter()
        .map(|(id, url)| {
            Task::perform(
                async move {
                    let id_clone = id.clone();
                    // Spawn on tokio runtime for true parallelism
                    tokio::spawn(async move {
                        (id_clone, load_thumb(&url).await.map_err(|e| e.to_string()))
                    })
                    .await
                    .unwrap_or_else(|_| (id, Err("Task panicked".to_string())))
                },
                move |(id, res)| Message::ThumbLoaded(id, res),
            )
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
        widget::text::Shaping,
        widget::{button, column, container, text, tooltip},
    };

    // Create title with tooltip
    let full_title = title_text.to_string();
    let display_title = truncate_title_smart(title_text, 25, 50);

    let title_widget = tooltip(
        text(display_title).size(14).shaping(Shaping::Advanced),
        container(text(full_title).shaping(Shaping::Advanced))
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
            info_col = info_col.push(text(ch.name).shaping(Shaping::Advanced));
        }
    }

    // Add metadata if provided
    if let Some(meta) = metadata_text {
        info_col = info_col.push(text(meta).size(12).shaping(Shaping::Advanced));
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
