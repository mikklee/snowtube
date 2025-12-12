//! Video control bar with play/pause, progress slider, and time display.

use iced::widget::{button, column, container, row, slider, text};
use iced::{Color, Element, Font, Length, Padding, Renderer, Theme};
use std::time::Duration;

/// Nerd Font for icons
const NERD_FONT: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
    ..Font::DEFAULT
};

/// Internal messages for the control bar
#[derive(Debug, Clone)]
pub enum ControlMessage {
    TogglePlayPause,
    SeekPreview(f64),
    SeekRelease,
}

/// Style for the glass container effect (frosted glass simulation)
pub fn glass_container_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    let bg_color = Color {
        r: palette.background.r,
        g: palette.background.g,
        b: palette.background.b,
        a: 0.98,
    };

    container::Style {
        background: Some(iced::Background::Color(bg_color)),
        border: iced::Border {
            color: Color {
                r: palette.text.r,
                g: palette.text.g,
                b: palette.text.b,
                a: 0.08,
            },
            width: 0.5,
            radius: 48.0.into(),
        },
        shadow: iced::Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.2,
            },
            offset: iced::Vector::new(0.0, -3.0),
            blur_radius: 16.0,
        },
        text_color: None,
        snap: false,
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

/// Format duration as MM:SS or HH:MM:SS
pub fn format_duration(duration: Duration) -> String {
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

/// Create a control button with icon and label
fn control_button<'a, Message: Clone + 'a>(
    icon: char,
    label: &'static str,
    message: Message,
    is_active: bool,
    disabled: bool,
) -> Element<'a, Message, Theme, Renderer> {
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

/// Create the video control bar with progress slider.
///
/// This control bar contains:
/// - Play/pause button
/// - Progress slider
/// - Time display (current / duration)
///
/// Note: Back button is NOT included - that's app-specific navigation.
pub fn video_control_bar<'a, Message: Clone + 'a>(
    is_paused: bool,
    position: Duration,
    duration: Duration,
    seek_preview: Option<f64>,
    on_toggle_play: Message,
    on_seek_preview: impl Fn(f64) -> Message + 'a,
    on_seek_release: Message,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let play_pause_icon = if is_paused { '\u{f04b}' } else { '\u{f04c}' };
    let play_pause_label = if is_paused { "Play" } else { "Pause" };

    // Clone for use in disabled slider fallback
    let on_toggle_play_for_slider = on_toggle_play.clone();

    let play_button = control_button(
        play_pause_icon,
        play_pause_label,
        on_toggle_play,
        false,
        false,
    );

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

    // Only enable seeking if video has started playing (position > 0)
    // Otherwise the A/V is desynced if the user seeks.
    let can_seek = duration.as_secs_f64() > 0.0 && position.as_millis() > 0;

    let progress_slider: Element<'a, Message, Theme, Renderer> = if can_seek {
        slider(0.0..=1.0, display_percent, on_seek_preview)
            .step(0.001)
            .on_release(on_seek_release)
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    } else {
        // Disabled slider - show current position but don't allow interaction
        slider(0.0..=1.0, display_percent, move |_| {
            on_toggle_play_for_slider.clone()
        })
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

    let controls_content = column![
        container(play_button)
            .width(Length::Fill)
            .center_x(Length::Fill),
        progress_row
    ]
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

/// Create a disabled control bar for loading state.
pub fn loading_control_bar<'a, Message: Clone + 'a>(
    on_toggle_play: Message,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    // Clone for use in slider closure
    let on_toggle_play_for_slider = on_toggle_play.clone();

    let play_button = control_button(
        '\u{f04b}', // Play icon
        "Play",
        on_toggle_play,
        false,
        true, // disabled
    );

    // Disabled progress slider showing 0:00
    let progress_slider: Element<'a, Message, Theme, Renderer> =
        slider(0.0..=1.0, 0.0, move |_| on_toggle_play_for_slider.clone())
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

    let controls_content = column![
        container(play_button)
            .width(Length::Fill)
            .center_x(Length::Fill),
        progress_row
    ]
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
