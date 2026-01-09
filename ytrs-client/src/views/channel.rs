//! Channel view for the ytrs-client application

use common::{ChannelTab, LanguageOption, format_relative_time, parse_relative_time};
use iced::{
    Alignment,
    Alignment::Center,
    Element, Length, Theme,
    widget::{Image, button, column, combo_box, container, pick_list, row, text},
};

use crate::App;
use crate::helpers::{centered_grid_padding, create_thumbnail, create_video_tile, fmt_num};
use crate::messages::Message;
use crate::theme::{rounded_button_style, rounded_combo_box_style, rounded_pick_list_style};
use crate::widgets::{Wrap, bounceable_scrollable, subscribe_button};

/// Render the channel view
pub fn view(
    app: &App,
    get_language_by_locale: fn(&str, &str) -> Option<&'static LanguageOption>,
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
        let avatar: Element<Message> = if let Some(h) = app.subscription_thumbs.get(&channel.id) {
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
            .get(&channel.key())
            .is_some_and(|c| c.subscribed);

        let sub_button = subscribe_button(is_subscribed, channel.key(), 40.0);

        let header = row![
            button(text("← Back"))
                .on_press(Message::BackToChannels)
                .padding(10)
                .style(rounded_button_style),
            avatar,
            info_column.padding(10),
            sub_button,
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

        // Language and Sort controls
        // Find the auto-detected language name to display in placeholder (O(1) HashMap lookup)
        let auto_detected_name =
            get_language_by_locale(&app.channel_locale.0, &app.channel_locale.1)
                .map(|lang| lang.name)
                .unwrap_or("Unknown");

        let placeholder = format!("Auto-detected: {}", auto_detected_name);

        let language_control = row![
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

        // Build sort dropdown if we have sort filters available
        let sort_control: Option<Element<Message>> = if !app.available_sort_filters.is_empty() {
            let filter_labels: Vec<String> = app
                .available_sort_filters
                .iter()
                .map(|f| f.label.clone())
                .collect();

            Some(
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
                .align_y(Alignment::Center)
                .into(),
            )
        } else {
            None
        };

        // Controls row
        let mut controls_row = row![language_control].spacing(10);
        if let Some(sort) = sort_control {
            controls_row = controls_row.push(sort);
        }

        // Responsive layout: if window is narrow, put controls in a new row with wrap
        let is_narrow = app.window_width < 800.0;

        let controls_section: Element<Message> = if is_narrow {
            column![header, tabs, controls_row.wrap()]
                .spacing(10)
                .width(Length::Fill)
                .into()
        } else {
            let tabs_and_controls = row![
                tabs,
                iced::widget::space::horizontal().width(Length::Fill),
                controls_row
            ]
            .align_y(Alignment::Center);
            column![header, tabs_and_controls]
                .spacing(10)
                .width(Length::Fill)
                .into()
        };

        // Add controls section with background and 2px bottom border
        let controls_with_border = column![
            container(controls_section)
                .padding(10)
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
                if r.id.is_empty() {
                    return None;
                }
                let h = app.thumbs.get(&r.id)?;

                let thumb = Image::new(h.clone()).width(240).height(135);
                let thumb_with_overlay = create_thumbnail(thumb, false, 0);

                let mut meta = vec![];
                if let Some(v) = r.view_count {
                    meta.push(format!("{} views", fmt_num(v)));
                }
                if let Some(d) = &r.duration_string {
                    meta.push(d.clone());
                }
                let seconds = parse_relative_time(r.published_text.as_deref());
                meta.push(format_relative_time(seconds));

                Some(create_video_tile(
                    thumb_with_overlay,
                    &r.title,
                    None,
                    Some(meta.join(" • ")),
                    Message::PlayVideo(Box::new(r.clone())),
                    &r.platform_icon,
                ))
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
                .visible_scrollbar(app.config.show_scrollbar)
                .into()
        };

        content = content.push(videos_section);

        content.into()
    } else {
        container(text("Loading channel...")).padding(40).into()
    }
}
