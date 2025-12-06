//! Video player view

use crate::App;
use crate::messages::Message;
use crate::widgets::{video_with_controls, video_with_controls_fullscreen};
use iced::widget::{container, text};
use iced::{Color, Element, Length};

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
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Color::BLACK)),
                ..Default::default()
            })
            .into();
        } else {
            return container(text("No video loaded"))
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
        // Account for tab bar height (~60px) when calculating available space
        let available_height = (app.window_height - 60.0).max(100.0);
        container(video_with_controls(
            video,
            title,
            is_paused,
            true,
            position,
            duration,
            app.window_width,
            available_height,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    } else {
        container(text("No video loaded"))
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
    }
}
