//! Channels (subscriptions) view for the ytrs-client application

use crate::App;

/// Parse relative time text like "2 days ago" into minutes for sorting.
/// Returns u64::MAX for unparseable strings (sorts to end).
fn parse_published_text(text: Option<&str>) -> u64 {
    let text = match text {
        Some(t) => t.to_lowercase(),
        None => return u64::MAX,
    };

    // Extract number and unit
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        return u64::MAX;
    }

    let num: u64 = match parts[0].parse() {
        Ok(n) => n,
        Err(_) => return u64::MAX,
    };

    let unit = parts[1];

    // Convert to minutes
    if unit.starts_with("second") {
        num / 60 // round down to 0 for < 60 seconds
    } else if unit.starts_with("minute") {
        num
    } else if unit.starts_with("hour") {
        num * 60
    } else if unit.starts_with("day") {
        num * 60 * 24
    } else if unit.starts_with("week") {
        num * 60 * 24 * 7
    } else if unit.starts_with("month") {
        num * 60 * 24 * 30
    } else if unit.starts_with("year") {
        num * 60 * 24 * 365
    } else {
        u64::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_published_text() {
        assert_eq!(parse_published_text(Some("30 seconds ago")), 0);
        assert_eq!(parse_published_text(Some("5 minutes ago")), 5);
        assert_eq!(parse_published_text(Some("1 minute ago")), 1);
        assert_eq!(parse_published_text(Some("2 hours ago")), 120);
        assert_eq!(parse_published_text(Some("1 hour ago")), 60);
        assert_eq!(parse_published_text(Some("3 days ago")), 3 * 60 * 24);
        assert_eq!(parse_published_text(Some("1 day ago")), 60 * 24);
        assert_eq!(parse_published_text(Some("2 weeks ago")), 2 * 60 * 24 * 7);
        assert_eq!(parse_published_text(Some("1 week ago")), 60 * 24 * 7);
        assert_eq!(parse_published_text(Some("6 months ago")), 6 * 60 * 24 * 30);
        assert_eq!(parse_published_text(Some("1 month ago")), 60 * 24 * 30);
        assert_eq!(parse_published_text(Some("2 years ago")), 2 * 60 * 24 * 365);
        assert_eq!(parse_published_text(Some("1 year ago")), 60 * 24 * 365);
    }

    #[test]
    fn test_parse_published_text_invalid() {
        assert_eq!(parse_published_text(None), u64::MAX);
        assert_eq!(parse_published_text(Some("")), u64::MAX);
        assert_eq!(parse_published_text(Some("invalid")), u64::MAX);
        assert_eq!(parse_published_text(Some("abc days ago")), u64::MAX);
    }

    #[test]
    fn test_parse_published_text_sorting() {
        let mut times = vec![
            parse_published_text(Some("1 year ago")),
            parse_published_text(Some("5 minutes ago")),
            parse_published_text(Some("2 days ago")),
            parse_published_text(Some("1 hour ago")),
        ];
        times.sort();
        assert_eq!(times[0], 5); // 5 minutes
        assert_eq!(times[1], 60); // 1 hour
        assert_eq!(times[2], 2 * 60 * 24); // 2 days
        assert_eq!(times[3], 60 * 24 * 365); // 1 year
    }
}
use crate::helpers::{centered_grid_padding, create_thumbnail, truncate_title};
use crate::messages::Message;
use crate::theme::rounded_button_style;
use crate::widgets::{Wrap, bounceable_scrollable};
use iced::{
    Alignment::Center,
    Element, Length,
    widget::text::Shaping,
    widget::{Image, button, column, container, row, text},
};

/// Render the channels (subscriptions) view
pub fn view(app: &App) -> Element<'_, Message> {
    let _start = std::time::Instant::now();

    // Header with refresh button
    let header = container(
        row![
            text("Channels").size(24).shaping(Shaping::Advanced),
            button(text("Refresh"))
                .on_press(Message::RefreshSubscriptionVideos)
                .padding(8)
                .style(rounded_button_style)
        ]
        .spacing(20)
        .align_y(Center),
    )
    .padding(20)
    .width(Length::Fill);

    let mut subscribed_channels: Vec<_> = app
        .config
        .channels
        .iter()
        .filter(|c| c.subscribed)
        .collect();
    subscribed_channels.sort_by(|a, b| {
        a.channel_name
            .to_lowercase()
            .cmp(&b.channel_name.to_lowercase())
    });

    let body: Element<Message> = if subscribed_channels.is_empty() {
        container(
            column![
                text("No channels yet").size(20).shaping(Shaping::Advanced),
                text("Subscribe to channels from search to see them here")
                    .size(14)
                    .shaping(Shaping::Advanced)
            ]
            .spacing(10)
            .align_x(Center),
        )
        .padding(60)
        .center_x(Length::Fill)
        .into()
    } else {
        // LEFT COLUMN: Channel cards (one per row)
        let channel_cards: Vec<Element<Message>> = subscribed_channels
            .iter()
            .filter_map(|channel_config| {
                let channel_id = &channel_config.channel_id;
                let avatar_handle = app.subscription_thumbs.get(channel_id)?.clone();
                let name = channel_config.channel_name.clone();

                let avatar = Image::new(avatar_handle).width(80).height(80);
                let channel_name_text = text(name)
                    .size(14)
                    .shaping(Shaping::Advanced)
                    .align_x(Center)
                    .width(120);

                let channel_card = button(
                    column![avatar, channel_name_text]
                        .align_x(Center)
                        .spacing(10)
                        .width(120),
                )
                .on_press(Message::ViewChannel(channel_id.clone()))
                .padding(10)
                .style(|theme: &iced::Theme, status| {
                    let palette = theme.palette();
                    let border_color = match status {
                        button::Status::Hovered | button::Status::Pressed => palette.primary,
                        _ => iced::Color::TRANSPARENT,
                    };
                    button::Style {
                        text_color: palette.text,
                        background: None,
                        border: iced::Border {
                            radius: 12.0.into(),
                            width: 2.0,
                            color: border_color,
                        },
                        ..Default::default()
                    }
                });

                Some(container(channel_card).center_x(Length::Fill).into())
            })
            .collect();

        let left_column = bounceable_scrollable(
            container(column(channel_cards).spacing(15.0)).padding(iced::Padding {
                top: 20.0,
                bottom: 100.0,
                left: 10.0,
                right: 10.0,
            }),
        )
        .id("subscriptions-channels")
        .width(Length::Fixed(160.0));

        // RIGHT COLUMN: Video grid (like search/channel view)
        // Collect all videos and sort by publish date (newest first)
        let mut all_videos: Vec<_> = subscribed_channels
            .iter()
            .flat_map(|channel_config| {
                let channel_id = &channel_config.channel_id;
                app.subscription_videos
                    .get(channel_id)
                    .map(|videos| videos.iter().collect::<Vec<_>>())
                    .unwrap_or_default()
            })
            .collect();

        // Sort by published_text (parse relative time)
        all_videos.sort_by(|a, b| {
            let a_mins = parse_published_text(a.published_text.as_deref());
            let b_mins = parse_published_text(b.published_text.as_deref());
            a_mins.cmp(&b_mins) // smaller = more recent
        });

        let video_cards: Vec<Element<Message>> = all_videos
            .into_iter()
            .filter_map(|video| {
                let vid = video.video_id.as_ref()?;
                let thumb_handle = app.thumbs.get(vid)?;

                let thumb = Image::new(thumb_handle.clone()).width(240).height(135);
                let is_playing = app.playing_video.as_ref() == Some(vid);
                let thumb_with_overlay = create_thumbnail(thumb, is_playing, app.countdown_value);

                let display_title = truncate_title(&video.title, 28);

                let card = column![
                    thumb_with_overlay,
                    container(text(display_title).size(12))
                        .padding(4)
                        .width(240)
                ]
                .spacing(4)
                .width(240);

                Some(
                    button(card)
                        .on_press(Message::Play(vid.clone()))
                        .padding(0)
                        .into(),
                )
            })
            .collect();

        // Calculate padding for video grid
        const CARD_WIDTH: f32 = 240.0;
        const CARD_SPACING: f32 = 15.0;
        let videos_width = app.window_width - 160.0; // subtract left column width

        let grid_padding =
            centered_grid_padding(videos_width, CARD_WIDTH, CARD_SPACING, 20.0, 20.0, 100.0);

        let right_column: Element<Message> = if video_cards.is_empty() {
            if subscribed_channels
                .iter()
                .any(|c| app.subscription_videos_loading.contains(&c.channel_id))
            {
                container(text("Loading videos...").size(14))
                    .padding(40)
                    .center_x(Length::Fill)
                    .into()
            } else {
                container(text("No videos yet").size(14))
                    .padding(40)
                    .center_x(Length::Fill)
                    .into()
            }
        } else {
            bounceable_scrollable(
                container(
                    Wrap::with_elements(video_cards)
                        .spacing(CARD_SPACING)
                        .line_spacing(CARD_SPACING),
                )
                .padding(grid_padding),
            )
            .id("subscriptions-videos")
            .into()
        };

        row![left_column, right_column].into()
    };

    eprintln!("  Subscriptions view TOTAL: {:?}", _start.elapsed());
    eprintln!("    - Total subscriptions: {}", subscribed_channels.len());

    column![header, body].height(Length::Fill).into()
}
