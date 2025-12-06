//! Video player view

use crate::App;
use crate::messages::Message;
use crate::widgets::{
    video_loading_placeholder, video_with_controls, video_with_controls_fullscreen,
};
use iced::widget::container;
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
                app.video_loading_status.as_deref(),
                app.window_width,
                app.window_height,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into();
        }
    }

    // Non-fullscreen: title and controls overlaid on video
    let title = app.playing_video_title.as_deref();

    if let Some(ref video) = app.video {
        let is_paused = video.paused();
        let position = video.position();
        let duration = video.duration();
        container(video_with_controls(
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
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    } else if app.video_loading {
        // Show loading placeholder with status (same positioning as video)
        container(video_loading_placeholder(
            app.video_loading_status.as_deref(),
            app.window_width,
            VIDEO_HEIGHT,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    } else {
        // No video and not loading - show empty placeholder (same positioning as video)
        container(video_loading_placeholder(
            None,
            app.window_width,
            VIDEO_HEIGHT,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    }
}
