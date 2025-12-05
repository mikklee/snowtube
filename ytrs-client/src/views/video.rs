//! Video player view

use crate::App;
use crate::messages::Message;
use crate::widgets::{video_with_controls, video_with_controls_fullscreen};
use iced::widget::{container, text};
use iced::{Element, Length};

pub fn view(app: &App) -> Element<'_, Message> {
    // In fullscreen mode: video with controls fills the screen
    if app.video_fullscreen {
        if let Some(ref video) = app.video {
            let is_paused = video.paused();
            let title = app.playing_video_title.as_deref();
            return video_with_controls_fullscreen(
                video,
                title,
                is_paused,
                app.video_controls_visible,
            );
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
        container(video_with_controls(video, title, is_paused, true))
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
