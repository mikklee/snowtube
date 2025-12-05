//! Video player widget with integrated controls

use crate::messages::Message;
use crate::widgets::glass::glass_container_style;
use iced::widget::{button, column, container, row, slider, stack, text};
use iced::{Color, Element, Font, Length, Padding, Theme};
use iced_video_player::{Video, VideoPlayer};
use std::time::Duration;

/// Nerd Font for icons
const NERD_FONT: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
    ..Font::DEFAULT
};

/// Create a video control bar button
fn control_button<'a>(
    icon: char,
    label: &'static str,
    message: Message,
    is_active: bool,
) -> Element<'a, Message> {
    let content = column![
        text(icon.to_string())
            .size(24.0)
            .font(NERD_FONT)
            .width(Length::Fill)
            .center(),
        text(label).size(14).width(Length::Fill).center(),
    ]
    .spacing(4)
    .align_x(iced::Alignment::Center)
    .width(Length::Fill);

    button(
        container(content)
            .padding(Padding {
                top: 6.0,
                bottom: 6.0,
                left: 12.0,
                right: 12.0,
            })
            .center_x(Length::Fill)
            .center_y(Length::Shrink),
    )
    .width(Length::FillPortion(1))
    .style(move |theme, status| control_button_style(theme, status, is_active))
    .on_press(message)
    .into()
}

/// Custom button style for control bar items
fn control_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_active: bool,
) -> iced::widget::button::Style {
    use iced::widget::button;
    let palette = theme.palette();

    let (background, text_color) = if is_active {
        (
            Some(iced::Background::Color(Color {
                r: palette.primary.r,
                g: palette.primary.g,
                b: palette.primary.b,
                a: 0.15,
            })),
            palette.primary,
        )
    } else {
        let text_alpha = match status {
            button::Status::Hovered => 0.8,
            button::Status::Pressed => 0.9,
            _ => 0.5,
        };
        (
            None,
            Color {
                r: palette.text.r,
                g: palette.text.g,
                b: palette.text.b,
                a: text_alpha,
            },
        )
    };

    button::Style {
        background,
        text_color,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 48.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

/// Format duration as MM:SS or HH:MM:SS
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Custom slider style for progress bar
fn progress_slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let palette = theme.palette();

    let handle_color = match status {
        slider::Status::Hovered | slider::Status::Dragged => palette.primary,
        _ => Color {
            r: palette.text.r,
            g: palette.text.g,
            b: palette.text.b,
            a: 0.9,
        },
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                iced::Background::Color(palette.primary),
                iced::Background::Color(Color {
                    r: palette.text.r,
                    g: palette.text.g,
                    b: palette.text.b,
                    a: 0.2,
                }),
            ),
            width: 4.0,
            border: iced::Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            background: iced::Background::Color(handle_color),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

/// Create the video control bar with progress slider
fn video_control_bar(
    is_paused: bool,
    position: Duration,
    duration: Duration,
) -> Element<'static, Message> {
    let play_pause_icon = if is_paused { '\u{f04b}' } else { '\u{f04c}' };
    let play_pause_label = if is_paused { "Play" } else { "Pause" };

    let buttons: Vec<Element<'static, Message>> = vec![
        control_button('\u{f053}', "Back", Message::BackFromVideo, false),
        control_button(
            play_pause_icon,
            play_pause_label,
            Message::TogglePlayPause,
            false,
        ),
    ];

    let buttons_row = row(buttons).spacing(0).width(Length::Fill);

    // Progress bar with time display
    let progress_percent = if duration.as_secs_f64() > 0.0 {
        (position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let progress_row = row![
        text(format_duration(position))
            .size(12)
            .width(Length::Shrink),
        slider(0.0..=1.0, progress_percent, Message::SeekVideo)
            .step(0.001)
            .width(Length::Fill)
            .style(progress_slider_style),
        text(format_duration(duration))
            .size(12)
            .width(Length::Shrink),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    let controls_content = column![buttons_row, progress_row]
        .spacing(8)
        .width(Length::Fill);

    let glass_bar = container(controls_content)
        .padding(Padding::new(12.0))
        .max_width(500.0)
        .width(Length::Fill)
        .style(glass_container_style);

    container(glass_bar)
        .padding(Padding {
            top: 8.0,
            bottom: 16.0,
            left: 12.0,
            right: 12.0,
        })
        .width(Length::Fill)
        .center_x(Length::Fill)
        .style(|_| container::Style {
            background: None,
            ..Default::default()
        })
        .into()
}

/// Video player with title at top and controls at bottom, all overlaid on video
/// For windowed mode - constrains height to video's actual dimensions, full width
pub fn video_with_controls<'a>(
    video: &'a Video,
    title: Option<&'a str>,
    is_paused: bool,
    show_controls: bool,
    position: Duration,
    duration: Duration,
) -> Element<'a, Message> {
    let (video_width, video_height) = video.size();

    let video_widget: Element<'a, Message> = VideoPlayer::new(video)
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(iced::ContentFit::Contain)
        .on_end_of_stream(Message::VideoEnded)
        .on_double_click(Message::ToggleFullscreen)
        .into();

    let mut layers: Vec<Element<'a, Message>> = vec![video_widget];

    // Title overlay at top
    if let Some(title_text) = title {
        layers.push(
            container(text(title_text).size(18))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Top)
                .align_x(iced::alignment::Horizontal::Left)
                .padding(Padding::from([15, 20]))
                .into(),
        );
    }

    // Controls overlay at bottom
    if show_controls {
        layers.push(
            container(video_control_bar(is_paused, position, duration))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom)
                .into(),
        );
    }

    // Stack video and overlays, fixed dimensions
    stack(layers)
        .width(Length::Fixed(video_width as f32))
        .height(Length::Fixed(video_height as f32))
        .into()
}

/// Video player with controls overlaid - fullscreen mode (fills available space)
pub fn video_with_controls_fullscreen<'a>(
    video: &'a Video,
    title: Option<&'a str>,
    is_paused: bool,
    show_controls: bool,
    position: Duration,
    duration: Duration,
) -> Element<'a, Message> {
    let video_widget: Element<'a, Message> = VideoPlayer::new(video)
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(iced::ContentFit::Contain)
        .on_end_of_stream(Message::VideoEnded)
        .on_double_click(Message::ToggleFullscreen)
        .into();

    let mut layers: Vec<Element<'a, Message>> = vec![
        container(video_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    ];

    // Title overlay at top (same visibility as controls)
    if show_controls {
        if let Some(title_text) = title {
            layers.push(
                container(text(title_text).size(18))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Top)
                    .align_x(iced::alignment::Horizontal::Left)
                    .padding(Padding::from([15, 20]))
                    .into(),
            );
        }
    }

    if show_controls {
        layers.push(
            container(video_control_bar(is_paused, position, duration))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom)
                .into(),
        );
    }

    stack(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
