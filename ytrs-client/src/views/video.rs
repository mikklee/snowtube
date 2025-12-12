//! Video player view

use crate::App;
use crate::messages::Message;
use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length};

/// Fixed video height for windowed mode (leaves room for options below)
const VIDEO_HEIGHT: f32 = 800.0;

pub fn view(app: &App) -> Element<'_, Message> {
    // Action buttons (Copy URL, Open in MPV)
    let action_buttons = if let Some(ref video_id) = app.playing_video_id {
        row![
            button(text("Copy URL").size(12))
                .on_press(Message::CopyVideoUrl(video_id.clone()))
                .padding(8),
            button(text("Open in MPV").size(12))
                .on_press(Message::LaunchInMpv(video_id.clone()))
                .padding(8),
            button(text("Back").size(12))
                .on_press(Message::BackFromVideo)
                .padding(8),
        ]
        .spacing(10)
    } else {
        row![
            button(text("Back").size(12))
                .on_press(Message::BackFromVideo)
                .padding(8),
        ]
    };

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
            // Windowed: video player with action buttons below
            container(
                column![video_player, action_buttons]
                    .spacing(10)
                    .align_x(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
        }
    } else {
        // No video player state - show back button
        container(
            column![
                text("No video loaded").size(16).color(Color::WHITE),
                action_buttons,
            ]
            .spacing(16)
            .align_x(iced::Alignment::Center),
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
