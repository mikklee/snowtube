//! Video player view

use crate::App;
use crate::messages::Message;
use crate::widgets::{
    video_loading_placeholder, video_with_controls, video_with_controls_fullscreen,
};
use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length};

/// Fixed video height for windowed mode (leaves room for options below)
const VIDEO_HEIGHT: f32 = 800.0;

pub fn view(app: &App) -> Element<'_, Message> {
    // In fullscreen mode: video with controls fills the screen
    if app.video_fullscreen {
        if let Some(ref video) = app.video {
            let is_paused = video.paused();
            let title = app.playing_video_title.as_deref();
            let position = video.position();
            let duration = video.duration();
            return container(video_with_controls_fullscreen(
                video,
                title,
                is_paused,
                app.video_controls_visible,
                position,
                duration,
                app.video_seek_preview,
                app.notification.as_deref(),
                app.video_seeking,
                &app.current_theme,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::BLACK)),
                ..Default::default()
            })
            .into();
        } else {
            // Fullscreen but no video - show loading placeholder
            return container(video_loading_placeholder(
                app.playing_video_title.as_deref(),
                app.video_loading_status.as_deref(),
                app.window_width,
                app.window_height,
                &app.current_theme,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into();
        }
    }

    // Non-fullscreen: title and controls overlaid on video
    let title = app.playing_video_title.as_deref();

    // Action buttons for non-fullscreen mode
    let action_buttons = if let Some(ref video_id) = app.playing_video_id {
        row![
            button(text("Copy URL").size(12))
                .on_press(Message::CopyVideoUrl(video_id.clone()))
                .padding(8),
            button(text("Open in MPV").size(12))
                .on_press(Message::LaunchInMpv(video_id.clone()))
                .padding(8),
        ]
        .spacing(10)
    } else {
        row![]
    };

    if let Some(ref video) = app.video {
        let is_paused = video.paused();
        let position = video.position();
        let duration = video.duration();
        let video_player = video_with_controls(
            video,
            title,
            is_paused,
            true,
            position,
            duration,
            app.window_width,
            VIDEO_HEIGHT,
            app.video_seek_preview,
            app.notification.as_deref(),
            app.video_seeking,
            &app.current_theme,
        );

        container(
            column![video_player, action_buttons]
                .spacing(10)
                .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    } else if let Some(ref error) = app.video_error {
        // Show error with action buttons
        let error_content = column![
            text("Failed to load video").size(20).color(Color::WHITE),
            text(error).size(14).color(Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 0.7,
            }),
            action_buttons,
            button(text("Back").size(14))
                .on_press(Message::BackFromVideo)
                .padding(10),
        ]
        .spacing(16)
        .align_x(iced::Alignment::Center);

        container(error_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::BLACK)),
                ..Default::default()
            })
            .into()
    } else if app.video_loading {
        // Show loading placeholder with status and action buttons
        let loading_placeholder = video_loading_placeholder(
            app.playing_video_title.as_deref(),
            app.video_loading_status.as_deref(),
            app.window_width,
            VIDEO_HEIGHT,
            &app.current_theme,
        );

        container(
            column![loading_placeholder, action_buttons]
                .spacing(10)
                .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    } else {
        // No video and not loading - show empty placeholder (same positioning as video)
        container(video_loading_placeholder(
            None,
            None,
            app.window_width,
            VIDEO_HEIGHT,
            &app.current_theme,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    }
}
