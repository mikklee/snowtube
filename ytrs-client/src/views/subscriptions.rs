//! Channels (subscriptions) view for the ytrs-client application

use crate::App;
use crate::helpers::{centered_grid_padding, create_thumbnail, fmt_num, truncate_title_smart};
use crate::messages::Message;
use crate::theme::rounded_button_style;
use crate::widgets::{Wrap, bounceable_scrollable};
use iced::{
    Alignment::Center,
    Element, Length,
    widget::text::Shaping,
    widget::{Image, button, column, container, lazy, row, text, tooltip},
};
use ytrs_lib::{format_relative_time, parse_relative_time};

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
            let a_mins = parse_relative_time(a.published_text.as_deref());
            let b_mins = parse_relative_time(b.published_text.as_deref());
            a_mins.cmp(&b_mins) // smaller = more recent
        });

        let video_cards: Vec<Element<Message>> = all_videos
            .into_iter()
            .filter(|video| video.is_premium != Some(true))
            .filter_map(|video| {
                let vid = video.video_id.clone()?;

                // Only render videos if thumbnail is loaded
                let thumb_handle = app.thumbs.get(&vid)?.clone();

                // Clone all data for lazy closure (must be owned)
                let view_count = video.view_count;
                let duration = video.duration.clone();
                let published_text = video.published_text.clone();
                let title = video.title.clone();
                let is_playing = app.playing_video.as_ref() == Some(&vid);
                let countdown = app.countdown_value;

                // Lazy widget caches rendering - only rebuilds when (vid, is_playing, countdown) changes
                Some(
                    lazy((vid.clone(), is_playing, countdown), move |_| {
                        let thumb = Image::new(thumb_handle.clone()).width(240).height(135);
                        let thumb_with_overlay = create_thumbnail(thumb, is_playing, countdown);

                        let display_title = truncate_title_smart(&title, 70, 110);

                        // Build metadata line same as channel view
                        let mut meta = vec![];
                        if let Some(v) = view_count {
                            meta.push(format!("{} views", fmt_num(v)));
                        }
                        if let Some(ref d) = duration {
                            meta.push(d.clone());
                        }
                        let seconds = parse_relative_time(published_text.as_deref());
                        let time_ago = format_relative_time(seconds);
                        meta.push(time_ago);

                        // Leak title for tooltip (same pattern as search view)
                        let title_static: &'static str = Box::leak(title.clone().into_boxed_str());

                        let title_widget = tooltip(
                            text(display_title).size(14).shaping(Shaping::Advanced),
                            container(text(title_static).shaping(Shaping::Advanced))
                                .style(container::dark)
                                .padding(10),
                            tooltip::Position::FollowCursor,
                        );

                        let info = column![
                            title_widget,
                            text(meta.join(" • ")).size(12).shaping(Shaping::Advanced)
                        ]
                        .spacing(4);

                        let card = column![
                            thumb_with_overlay,
                            container(info)
                                .padding(8)
                                .width(240)
                                .height(Length::Fixed(120.0))
                        ]
                        .spacing(0)
                        .width(240);

                        let btn: Element<'static, Message> = button(card)
                            .on_press(Message::Play(vid.clone()))
                            .padding(0)
                            .into();
                        btn
                    })
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
