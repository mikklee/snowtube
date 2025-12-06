//! Video player widget with integrated controls

use crate::messages::Message;
use crate::widgets::glass::glass_container_style;
use crate::widgets::spinner::shader_spinner;

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
    disabled: bool,
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

    let btn = button(
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
    .style(move |theme, status| control_button_style(theme, status, is_active, disabled));

    if disabled {
        btn.into()
    } else {
        btn.on_press(message).into()
    }
}

/// Custom button style for control bar items
fn control_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_active: bool,
    disabled: bool,
) -> iced::widget::button::Style {
    use iced::widget::button;
    let palette = theme.palette();

    let (background, text_color) = if disabled {
        (
            None,
            Color {
                r: palette.text.r,
                g: palette.text.g,
                b: palette.text.b,
                a: 0.2,
            },
        )
    } else if is_active {
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

/// Seeking overlay with spinner and status text
/// Reusable across windowed and fullscreen modes
pub fn seeking_overlay(theme: &Theme) -> Element<'static, Message> {
    let spinner: Element<'static, Message> = shader_spinner(48.0, theme);

    let seeking_content = column![
        spinner,
        text("Seeking. This may take a minute.")
            .size(16)
            .color(Color::WHITE),
        text("If it takes longer, try reloading the video.")
            .size(14)
            .color(Color::WHITE)
    ]
    .spacing(16)
    .align_x(iced::Alignment::Center);

    container(seeking_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
            ..Default::default()
        })
        .into()
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
            width: 2.0,
            border: iced::Border {
                radius: 1.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 5.0 },
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
    seek_preview: Option<f64>,
) -> Element<'static, Message> {
    let play_pause_icon = if is_paused { '\u{f04b}' } else { '\u{f04c}' };
    let play_pause_label = if is_paused { "Play" } else { "Pause" };

    let buttons: Vec<Element<'static, Message>> = vec![
        control_button('\u{f053}', "Back", Message::BackFromVideo, false, false),
        control_button(
            play_pause_icon,
            play_pause_label,
            Message::TogglePlayPause,
            false,
            false,
        ),
    ];

    let buttons_row = row(buttons).spacing(0).width(Length::Fill);

    // Progress bar with time display
    // Show preview position if dragging, otherwise show current position
    let (display_percent, display_position) = if let Some(preview) = seek_preview {
        let preview_duration = Duration::from_secs_f64(duration.as_secs_f64() * preview);
        (preview, preview_duration)
    } else {
        let progress_percent = if duration.as_secs_f64() > 0.0 {
            (position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (progress_percent, position)
    };

    // Only enable seeking if video has started playing (position > 0 or has been playing)
    // Otherwise the A/V is desynced if the user seeks.
    let can_seek = duration.as_secs_f64() > 0.0 && position.as_millis() > 0;

    let progress_slider: Element<'static, Message> = if can_seek {
        slider(0.0..=1.0, display_percent, Message::SeekVideoPreview)
            .step(0.001)
            .on_release(Message::SeekVideoRelease)
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    } else {
        slider(0.0..=1.0, 0.0, |_| Message::NoOp)
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    };

    let progress_row = row![
        text(format_duration(display_position))
            .size(12)
            .width(Length::Shrink),
        progress_slider,
        text(format_duration(duration))
            .size(12)
            .width(Length::Shrink),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .padding(Padding {
        left: 20.0,
        right: 20.0,
        ..Padding::ZERO
    })
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

/// Create the video control bar for loading state (play/pause disabled)
fn loading_control_bar() -> Element<'static, Message> {
    let buttons: Vec<Element<'static, Message>> = vec![
        control_button('\u{f053}', "Back", Message::BackFromVideo, false, false),
        control_button(
            '\u{f04b}', // Play icon
            "Play",
            Message::TogglePlayPause,
            false,
            true, // disabled
        ),
    ];

    let buttons_row = row(buttons).spacing(0).width(Length::Fill);

    // Disabled progress slider showing 0:00
    let progress_slider: Element<'static, Message> = slider(0.0..=1.0, 0.0, |_| Message::NoOp)
        .width(Length::Fill)
        .style(progress_slider_style)
        .into();

    let progress_row = row![
        text("00:00").size(12).width(Length::Shrink),
        progress_slider,
        text("00:00").size(12).width(Length::Shrink),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .padding(Padding {
        left: 20.0,
        right: 20.0,
        ..Padding::ZERO
    })
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
/// For windowed mode - scales to fit available space while maintaining aspect ratio
#[allow(clippy::too_many_arguments)]
pub fn video_with_controls<'a>(
    video: &'a Video,
    title: Option<&'a str>,
    is_paused: bool,
    show_controls: bool,
    position: Duration,
    duration: Duration,
    available_width: f32,
    available_height: f32,
    seek_preview: Option<f64>,
    notification: Option<&'a str>,
    is_seeking: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let (video_width, video_height) = video.size();

    // Calculate scaled dimensions to fit within available space while maintaining aspect ratio
    let video_aspect = video_width as f32 / video_height as f32;
    let available_aspect = available_width / available_height;

    let (scaled_width, scaled_height) = if video_aspect > available_aspect {
        // Video is wider than available space - constrain by width
        (available_width, available_width / video_aspect)
    } else {
        // Video is taller than available space - constrain by height
        (available_height * video_aspect, available_height)
    };

    let video_widget: Element<'a, Message> = VideoPlayer::new(video)
        .width(scaled_width)
        .height(scaled_height)
        .content_fit(iced::ContentFit::Contain)
        .on_end_of_stream(Message::VideoEnded)
        .on_double_click(Message::ToggleFullscreen)
        .into();

    let mut layers: Vec<Element<'a, Message>> = vec![video_widget];

    // Title overlay at top with text shadow
    if let Some(title_text) = title {
        let shadow_text = text(title_text).size(18).color(Color::BLACK);
        let main_text = text(title_text).size(18).color(Color::WHITE);
        let title_with_shadow = stack![
            container(shadow_text).padding(Padding {
                top: 1.0,
                bottom: 0.0,
                left: 1.0,
                right: 0.0
            }),
            main_text,
        ];
        layers.push(
            container(title_with_shadow)
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
            container(video_control_bar(
                is_paused,
                position,
                duration,
                seek_preview,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom)
            .into(),
        );
    }

    // Notification overlay at center
    if let Some(msg) = notification {
        layers.push(
            container(
                container(text(msg).size(14).color(Color::WHITE))
                    .padding(Padding::new(12.0))
                    .style(glass_container_style),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        );
    }

    // Seeking overlay with spinner
    if is_seeking {
        layers.push(seeking_overlay(theme));
    }

    // Stack sized to scaled video dimensions
    stack(layers)
        .width(Length::Fixed(scaled_width))
        .height(Length::Fixed(scaled_height))
        .into()
}

/// Video player with controls overlaid - fullscreen mode (fills available space)
#[allow(clippy::too_many_arguments)]
pub fn video_with_controls_fullscreen<'a>(
    video: &'a Video,
    title: Option<&'a str>,
    is_paused: bool,
    show_controls: bool,
    position: Duration,
    duration: Duration,
    seek_preview: Option<f64>,
    notification: Option<&'a str>,
    is_seeking: bool,
    theme: &Theme,
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

    // Title overlay at top with text shadow (same visibility as controls)
    if show_controls && let Some(title_text) = title {
        let shadow_text = text(title_text).size(18).color(Color::BLACK);
        let main_text = text(title_text).size(18).color(Color::WHITE);
        let title_with_shadow = stack![
            container(shadow_text).padding(Padding {
                top: 1.0,
                bottom: 0.0,
                left: 1.0,
                right: 0.0
            }),
            main_text,
        ];
        layers.push(
            container(title_with_shadow)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Top)
                .align_x(iced::alignment::Horizontal::Left)
                .padding(Padding::from([15, 20]))
                .into(),
        );
    }

    if show_controls {
        layers.push(
            container(video_control_bar(
                is_paused,
                position,
                duration,
                seek_preview,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom)
            .into(),
        );
    }

    // Notification overlay at center
    if let Some(msg) = notification {
        layers.push(
            container(
                container(text(msg).size(14).color(Color::WHITE))
                    .padding(Padding::new(12.0))
                    .style(glass_container_style),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        );
    }

    // Seeking overlay with spinner
    if is_seeking {
        layers.push(seeking_overlay(theme));
    }

    stack(layers)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Video loading placeholder with black background, spinner, and status text
/// Used when video is loading to show progress to the user
/// Uses 16:9 aspect ratio to calculate dimensions
pub fn video_loading_placeholder<'a>(
    title: Option<&'a str>,
    status: Option<&'a str>,
    available_width: f32,
    available_height: f32,
    theme: &Theme,
) -> Element<'a, Message> {
    // Standard 16:9 aspect ratio
    const ASPECT_RATIO: f32 = 16.0 / 9.0;

    // Calculate dimensions to fit within available space while maintaining 16:9
    let available_aspect = available_width / available_height;
    let (width, height) = if ASPECT_RATIO > available_aspect {
        // Constrain by width
        (available_width, available_width / ASPECT_RATIO)
    } else {
        // Constrain by height
        (available_height * ASPECT_RATIO, available_height)
    };

    // Shader spinner that animates itself
    let spinner: Element<'a, Message> = shader_spinner(48.0, theme);

    let status_text = status.map(|s| {
        text(s).size(14).color(Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.7,
        })
    });

    let mut center_content = column![spinner]
        .spacing(16)
        .align_x(iced::Alignment::Center);

    if let Some(status_widget) = status_text {
        center_content = center_content.push(status_widget);
    }

    let mut layers: Vec<Element<'a, Message>> = vec![
        // Centered spinner and status
        container(center_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
    ];

    // Title overlay at top left (same style as video player)
    if let Some(title_text) = title {
        let shadow_text = text(title_text).size(18).color(Color::BLACK);
        let main_text = text(title_text).size(18).color(Color::WHITE);
        let title_with_shadow = stack![
            container(shadow_text).padding(Padding {
                top: 1.0,
                bottom: 0.0,
                left: 1.0,
                right: 0.0
            }),
            main_text,
        ];
        layers.push(
            container(title_with_shadow)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Top)
                .align_x(iced::alignment::Horizontal::Left)
                .padding(Padding::from([15, 20]))
                .into(),
        );
    }

    // Controls at bottom (with disabled play/pause)
    layers.push(
        container(loading_control_bar())
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom)
            .into(),
    );

    // Container with black background, sized to match video dimensions
    container(stack(layers))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        })
        .into()
}
