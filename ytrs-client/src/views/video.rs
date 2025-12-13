//! Video player view

use crate::App;
use crate::helpers::channel_name_button;
use crate::messages::Message;
use crate::theme::rounded_button_style;
use crate::widgets::{ICON_COPY, ICON_PLAY, bounceable_scrollable, icon_button, subscribe_button};
use iced::widget::{Image, button, column, container, row, text};
use iced::{Alignment, Border, Color, Element, Length, Theme};

/// Fixed video height for windowed mode (leaves room for info box below)
const VIDEO_HEIGHT: f32 = 600.0;
/// Control bar height (must match iceplayer)
const CONTROL_BAR_HEIGHT: f32 = 44.0;
/// Standard 16:9 aspect ratio
const ASPECT_RATIO: f32 = 16.0 / 9.0;

/// Rounded container style for the info box
fn info_box_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Circular avatar container style
fn avatar_style(_theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            radius: 24.0.into(), // Half of 48px for circular
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Calculate video dimensions maintaining 16:9 aspect ratio
fn calculate_video_dimensions(available_width: f32, available_height: f32) -> (f32, f32) {
    let video_available_height = available_height - CONTROL_BAR_HEIGHT;
    let available_aspect = available_width / video_available_height;

    if ASPECT_RATIO > available_aspect {
        // Width-constrained
        (
            available_width,
            available_width / ASPECT_RATIO + CONTROL_BAR_HEIGHT,
        )
    } else {
        // Height-constrained
        (video_available_height * ASPECT_RATIO, available_height)
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    if let Some(ref state) = app.video_player {
        // Determine available dimensions
        let (available_width, available_height) = if state.fullscreen {
            (app.window_width, app.window_height)
        } else {
            (app.window_width, VIDEO_HEIGHT)
        };

        // Render the video player widget
        let video_player = iceplayer::widget::view(
            state,
            Message::VideoPlayer,
            available_width,
            available_height,
            &app.current_theme,
        );

        if state.fullscreen {
            // Fullscreen: just the video player
            container(video_player)
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(Color::BLACK)),
                    ..Default::default()
                })
                .into()
        } else {
            // Calculate actual video width for constraining the info box
            let (video_width, _) = calculate_video_dimensions(available_width, available_height);

            // Back button above video (left-aligned, not constrained to video width)
            let back_button = button(text("← Back").size(14))
                .on_press(Message::BackFromVideo)
                .padding(10)
                .style(rounded_button_style);

            let back_row = container(back_button)
                .width(Length::Fill)
                .padding(iced::Padding {
                    top: 20.0,
                    right: 20.0,
                    bottom: 10.0,
                    left: 20.0,
                });

            // Windowed: back button, video player, info box below
            let info_box = build_info_box(app, video_width);

            let content = column![
                back_row,
                container(
                    column![video_player, info_box]
                        .spacing(0)
                        .align_x(Alignment::Center)
                        .width(Length::Fill),
                )
                .padding(iced::Padding {
                    top: 100.0,
                    right: 0.0,
                    bottom: 100.0,
                    left: 0.0,
                })
            ]
            .spacing(0)
            .align_x(Alignment::Center)
            .width(Length::Fill);

            bounceable_scrollable(content)
                .id("video")
                .visible_scrollbar(app.config.show_scrollbar)
                .into()
        }
    } else {
        // No video player state - show back button
        let back_button = button(text("Back").size(14))
            .on_press(Message::BackFromVideo)
            .padding(10)
            .style(rounded_button_style);

        container(
            column![
                text("No video loaded").size(16).color(Color::WHITE),
                back_button,
            ]
            .spacing(16)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        })
        .into()
    }
}

/// Build the info box with channel avatar, title, subscribe button, and action buttons
fn build_info_box(app: &App, video_width: f32) -> Element<'_, Message> {
    let video_id = app.playing_video_id.clone().unwrap_or_default();

    // Get title from video info
    let title = app
        .playing_video_info
        .as_ref()
        .map(|i| i.title.clone())
        .unwrap_or_default();

    // Get channel name and id from the values passed by PlayVideo message
    let channel_name = app.playing_channel_name.clone().unwrap_or_default();
    let channel_id = app.playing_channel_id.clone();

    // Channel avatar (rounded) - look up by channel_id in thumbs or subscription_thumbs
    let avatar_handle = channel_id.as_ref().and_then(|cid| {
        app.thumbs
            .get(cid)
            .or_else(|| app.subscription_thumbs.get(cid))
    });

    let avatar: Element<Message> = if let Some(handle) = avatar_handle {
        container(Image::new(handle.clone()).width(48).height(48)).style(avatar_style)
    } else {
        // Placeholder avatar
        container(iced::widget::space::Space::new().width(48).height(48)).style(|theme: &Theme| {
            container::Style {
                background: Some(iced::Background::Color(theme.palette().primary)),
                border: Border {
                    radius: 24.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
    }
    .into();

    let title_text = text(title).size(18);
    let channel_element = channel_name_button(channel_name, channel_id.clone());

    let title_column = column![title_text, channel_element]
        .spacing(4)
        .width(Length::Fill);

    // Subscribe button - check if subscribed
    let sub_button: Element<Message> = if let Some(cid) = channel_id {
        let is_subscribed = app
            .config
            .channels
            .iter()
            .any(|c| c.channel_id == cid && c.subscribed);
        subscribe_button(is_subscribed, cid, 40.0)
    } else {
        iced::widget::space::Space::new().into()
    };

    // Action buttons (Copy URL, Open in MPV)
    let copy_button = icon_button(
        ICON_COPY,
        40.0,
        "Copy URL",
        true,
        Message::CopyVideoUrl(video_id.clone()),
    );
    let mpv_button = icon_button(
        ICON_PLAY,
        40.0,
        "Open in MPV",
        true,
        Message::LaunchInMpv(video_id),
    );

    let action_buttons = row![
        iced::widget::space::Space::new().width(Length::Fill),
        sub_button,
        copy_button,
        mpv_button
    ]
    .spacing(8)
    .width(Length::Fixed(200.0));

    // Always two rows: title on top, buttons below (right-aligned)
    let info_content = row![avatar, title_column, action_buttons]
        .spacing(12)
        .align_y(Alignment::Center);

    container(info_content)
        .width(Length::Fixed(video_width))
        .padding(16)
        .style(info_box_style)
        .into()
}
