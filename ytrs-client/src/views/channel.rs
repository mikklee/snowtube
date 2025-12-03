//! Channel view for the ytrs-client application

use iced::{
    Alignment,
    Alignment::Center,
    Element, Length, Theme,
    widget::{Image, button, column, combo_box, container, pick_list, row, text},
};
use ytrs_lib::ChannelTab;

use crate::App;
use crate::helpers::{centered_grid_padding, create_thumbnail, fmt_num, truncate_title_smart};
use crate::messages::Message;
use crate::theme::{rounded_button_style, rounded_combo_box_style, rounded_pick_list_style};
use crate::widgets::{Wrap, bounceable_scrollable};

/// Render the channel view
pub fn view(
    app: &App,
    get_language_by_locale: fn(&str, &str) -> Option<&'static ytrs_lib::LanguageOption>,
) -> Element<'_, Message> {
    if let Some(ref channel) = app.current_channel {
        let mut content = column![].spacing(0);

        // Banner with header overlay
        let banner_image: Element<Message> = if let Some(ref banner_handle) = app.banner {
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
        let avatar: Element<Message> = if let Some(h) = app.thumbs.get(&channel.id) {
            Image::new(h.clone()).width(80).height(80).into()
        } else {
            container(iced::widget::space()).width(80).height(80).into()
        };

        let mut info_column = column![text(&channel.name).size(24),].spacing(5);

        if let Some(ref subs) = channel.subscriber_count {
            info_column = info_column.push(text(subs).size(14));
        }

        // Check if subscribed
        let is_subscribed = app
            .config
            .channels
            .iter()
            .any(|c| c.channel_id == channel.id && c.subscribed);

        let subscribe_button = if is_subscribed {
            button(text("Unsubscribe"))
                .on_press(Message::UnsubscribeFromChannel(channel.id.clone()))
                .padding(10)
                .style(rounded_button_style)
        } else {
            button(text("Subscribe"))
                .on_press(Message::SubscribeToChannel)
                .padding(10)
                .style(rounded_button_style)
        };

        let header = row![
            button(text("← Back"))
                .on_press(Message::BackToChannels)
                .padding(10)
                .style(rounded_button_style),
            avatar,
            info_column.padding(10),
            subscribe_button,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // Tabs
        let tabs = row![
            button(text("VIDEOS"))
                .on_press(Message::ChangeChannelTab(ChannelTab::Videos))
                .padding(10)
                .style(rounded_button_style),
            button(text("SHORTS"))
                .on_press(Message::ChangeChannelTab(ChannelTab::Shorts))
                .padding(10)
                .style(rounded_button_style),
            button(text("LIVE"))
                .on_press(Message::ChangeChannelTab(ChannelTab::Streams))
                .padding(10)
                .style(rounded_button_style),
        ]
        .spacing(10);

        // Language and Sort controls on the same row
        // Find the auto-detected language name to display in placeholder (O(1) HashMap lookup)
        let auto_detected_name =
            get_language_by_locale(&app.channel_locale.0, &app.channel_locale.1)
                .map(|lang| lang.name)
                .unwrap_or("Unknown");

        let placeholder = format!("Auto-detected: {}", auto_detected_name);

        let mut controls_row = row![
            text("Language:").size(14),
            combo_box(
                &app.language_combo_state,
                &placeholder,
                app.selected_language.as_ref(),
                Message::LanguageSelected,
            )
            .width(250)
            .input_style(rounded_combo_box_style)
        ]
        .align_y(Center)
        .spacing(10);

        // Add sort dropdown if we have sort filters available
        if !app.available_sort_filters.is_empty() {
            let filter_labels: Vec<String> = app
                .available_sort_filters
                .iter()
                .map(|f| f.label.clone())
                .collect();

            controls_row = controls_row.push(
                row![
                    text("Sort by:").size(14),
                    pick_list(
                        filter_labels,
                        app.selected_sort_label.clone(),
                        Message::ChangeSortFilter,
                    )
                    .padding(5)
                    .style(rounded_pick_list_style)
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
            container(iced::widget::space())
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
        let video_cards: Vec<Element<Message>> = app
            .channel_results
            .iter()
            .filter(|r| {
                // Filter out premium/members-only videos (keep videos where is_premium is NOT true)
                r.is_premium != Some(true)
            })
            .filter_map(|r| {
                let vid = r.video_id.as_ref()?;
                let h = app.thumbs.get(vid)?;

                let thumb = Image::new(h.clone()).width(240).height(135);

                // Check if this video is currently playing
                let is_playing = app.playing_video.as_ref() == Some(vid);
                let countdown = app.countdown_value;

                // Create thumbnail with optional countdown overlay
                let thumb_with_overlay = create_thumbnail(thumb, is_playing, countdown);

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
                let display_title = truncate_title_smart(&r.title, 25, 50);

                let title_widget = iced::widget::tooltip(
                    text(display_title).size(14),
                    container(text(full_title))
                        .style(container::dark)
                        .padding(10),
                    iced::widget::tooltip::Position::FollowCursor,
                );

                let card = column![
                    thumb_with_overlay,
                    container(column![title_widget, text(meta.join(" • ")).size(12),].spacing(4))
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
            if app.loading_channel {
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
            const CARD_WIDTH: f32 = 240.0;
            const CARD_SPACING: f32 = 15.0;

            let grid_padding = centered_grid_padding(
                app.window_width,
                CARD_WIDTH,
                CARD_SPACING,
                20.0,  // min_padding
                20.0,  // top
                100.0, // bottom - extra space for tab bar overlay
            );

            let mut video_content = column![
                Wrap::with_elements(video_cards)
                    .spacing(CARD_SPACING)
                    .line_spacing(CARD_SPACING)
            ];

            // Show "Load More" button or loading indicator
            if app.channel_preloading {
                // Still preloading initial videos
                let loading_indicator =
                    container(text("Still requesting videos from YouTube...").size(14))
                        .padding(20)
                        .center_x(Length::Fill);
                video_content = video_content.push(loading_indicator);
            } else if app.channel_loading_more {
                let loading_indicator = container(text("Loading more...").size(14))
                    .padding(20)
                    .center_x(Length::Fill);
                video_content = video_content.push(loading_indicator);
            } else if app.channel_continuation.is_some() {
                // Show "Load More" button if we have more videos to load
                let load_more_btn = container(
                    button(text("Load More Videos"))
                        .on_press(Message::LoadMoreVideos)
                        .padding(10)
                        .style(rounded_button_style),
                )
                .padding(20)
                .center_x(Length::Fill);
                video_content = video_content.push(load_more_btn);
            }

            bounceable_scrollable(container(video_content).padding(grid_padding))
                .id("channel")
                .into()
        };

        content = content.push(videos_section);

        content.into()
    } else {
        container(text("Loading channel...")).padding(40).into()
    }
}
