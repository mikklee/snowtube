//! Video control bar with play/pause, progress slider, time display, and fullscreen toggle.

use iced::widget::{button, container, row, slider, text};
use iced::{Color, Element, Font, Length, Padding, Renderer, Theme};
use std::time::Duration;

/// Nerd Font for icons
const NERD_FONT: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
    ..Font::DEFAULT
};

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
            radius: 0.0.into(),
        },
        shadow: iced::Shadow {
            color: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.2,
            },
            offset: iced::Vector::new(0.0, -2.0),
            blur_radius: 8.0,
        },
        text_color: None,
        snap: false,
    }
}

/// Style for fullscreen overlay controls (semi-transparent)
fn fullscreen_control_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    container::Style {
        background: Some(iced::Background::Color(Color {
            r: palette.background.r * 0.1,
            g: palette.background.g * 0.1,
            b: palette.background.b * 0.1,
            a: 0.85,
        })),
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
        text_color: None,
        snap: false,
    }
}

/// Custom button style for control bar icon buttons
fn icon_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    disabled: bool,
) -> iced::widget::button::Style {
    use iced::widget::button;
    let palette = theme.palette();

    let text_color = if disabled {
        Color {
            r: palette.text.r,
            g: palette.text.g,
            b: palette.text.b,
            a: 0.3,
        }
    } else {
        match status {
            button::Status::Hovered => palette.primary,
            button::Status::Pressed => Color {
                r: palette.primary.r,
                g: palette.primary.g,
                b: palette.primary.b,
                a: 0.8,
            },
            _ => Color {
                r: palette.text.r,
                g: palette.text.g,
                b: palette.text.b,
                a: 0.9,
            },
        }
    };

    button::Style {
        background: None,
        text_color,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
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

/// Create an icon button
fn icon_button<'a, Message: Clone + 'a>(
    icon: char,
    message: Message,
    disabled: bool,
) -> Element<'a, Message, Theme, Renderer> {
    let icon_text = text(icon.to_string()).size(20.0).font(NERD_FONT);

    let btn = button(
        container(icon_text)
            .padding(Padding::new(8.0))
            .center_x(Length::Shrink)
            .center_y(Length::Shrink),
    )
    .style(move |theme, status| icon_button_style(theme, status, disabled));

    if disabled {
        btn.into()
    } else {
        btn.on_press(message).into()
    }
}

/// Create the video control bar for windowed mode (below video).
///
/// Layout: |play/pause| 00:00 ----slider---- 00:00 |fullscreen|
pub fn video_control_bar<'a, Message: Clone + 'a>(
    is_paused: bool,
    position: Duration,
    duration: Duration,
    seek_preview: Option<f64>,
    on_toggle_play: Message,
    on_seek_preview: impl Fn(f64) -> Message + 'a,
    on_seek_release: Message,
    on_toggle_fullscreen: Message,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let play_pause_icon = if is_paused { '\u{f04b}' } else { '\u{f04c}' }; // play/pause icons

    let play_button = icon_button(play_pause_icon, on_toggle_play.clone(), false);

    // Progress calculation
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
    let can_seek = duration.as_secs_f64() > 0.0 && position.as_millis() > 0;

    let progress_slider: Element<'a, Message, Theme, Renderer> = if can_seek {
        slider(0.0..=1.0, display_percent, on_seek_preview)
            .step(0.001)
            .on_release(on_seek_release)
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    } else {
        // Disabled slider
        slider(0.0..=1.0, display_percent, move |_| on_toggle_play.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    };

    let fullscreen_icon = '\u{eb4c}'; // screen_full (codicon)
    let fullscreen_button = icon_button(fullscreen_icon, on_toggle_fullscreen, false);

    let controls_row = row![
        play_button,
        text(format_duration(display_position))
            .size(12)
            .width(Length::Shrink),
        progress_slider,
        text(format_duration(duration))
            .size(12)
            .width(Length::Shrink),
        fullscreen_button,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .padding(Padding {
        top: 4.0,
        bottom: 4.0,
        left: 8.0,
        right: 8.0,
    })
    .width(Length::Fill);

    container(controls_row)
        .width(Length::Fill)
        .style(glass_container_style)
        .into()
}

/// Create the video control bar for fullscreen mode (overlay at bottom).
///
/// Layout: |play/pause| 00:00 ----slider---- 00:00 |exit fullscreen|
pub fn fullscreen_control_bar<'a, Message: Clone + 'a>(
    is_paused: bool,
    position: Duration,
    duration: Duration,
    seek_preview: Option<f64>,
    on_toggle_play: Message,
    on_seek_preview: impl Fn(f64) -> Message + 'a,
    on_seek_release: Message,
    on_toggle_fullscreen: Message,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let play_pause_icon = if is_paused { '\u{f04b}' } else { '\u{f04c}' };

    let play_button = icon_button(play_pause_icon, on_toggle_play.clone(), false);

    // Progress calculation
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

    let can_seek = duration.as_secs_f64() > 0.0 && position.as_millis() > 0;

    let progress_slider: Element<'a, Message, Theme, Renderer> = if can_seek {
        slider(0.0..=1.0, display_percent, on_seek_preview)
            .step(0.001)
            .on_release(on_seek_release)
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    } else {
        slider(0.0..=1.0, display_percent, move |_| on_toggle_play.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    };

    let exit_fullscreen_icon = '\u{eb4d}'; // screen_normal (codicon)
    let fullscreen_button = icon_button(exit_fullscreen_icon, on_toggle_fullscreen, false);

    let controls_row = row![
        play_button,
        text(format_duration(display_position))
            .size(12)
            .color(Color::WHITE)
            .width(Length::Shrink),
        progress_slider,
        text(format_duration(duration))
            .size(12)
            .color(Color::WHITE)
            .width(Length::Shrink),
        fullscreen_button,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .padding(Padding {
        top: 8.0,
        bottom: 8.0,
        left: 12.0,
        right: 12.0,
    })
    .width(Length::Fill);

    container(controls_row)
        .width(Length::Fill)
        .style(fullscreen_control_style)
        .into()
}

/// Create a disabled control bar for loading state.
pub fn loading_control_bar<'a, Message: Clone + 'a>(
    duration: Duration,
    on_toggle_play: Message,
    on_toggle_fullscreen: Message,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let play_button = icon_button('\u{f04b}', on_toggle_play.clone(), true);

    let progress_slider: Element<'a, Message, Theme, Renderer> =
        slider(0.0..=1.0, 0.0, move |_| on_toggle_play.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into();

    let fullscreen_button = icon_button('\u{eb4c}', on_toggle_fullscreen, true); // screen_full (codicon)

    let controls_row = row![
        play_button,
        text("00:00").size(12).width(Length::Shrink),
        progress_slider,
        text(format_duration(duration))
            .size(12)
            .width(Length::Shrink),
        fullscreen_button,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .padding(Padding {
        top: 4.0,
        bottom: 4.0,
        left: 8.0,
        right: 8.0,
    })
    .width(Length::Fill);

    container(controls_row)
        .width(Length::Fill)
        .style(glass_container_style)
        .into()
}

/// Create control bar for ready-to-play state (before video loads).
/// Play button is enabled, but seeking is disabled.
pub fn ready_control_bar<'a, Message: Clone + 'a>(
    duration: Duration,
    on_start_playback: Message,
    on_toggle_fullscreen: Message,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let play_button = icon_button('\u{f04b}', on_start_playback.clone(), false); // play enabled

    // Disabled slider at position 0
    let progress_slider: Element<'a, Message, Theme, Renderer> =
        slider(0.0..=1.0, 0.0, move |_| on_start_playback.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into();

    let fullscreen_button = icon_button('\u{eb4c}', on_toggle_fullscreen, false); // screen_full (codicon)

    let controls_row = row![
        play_button,
        text("00:00").size(12).width(Length::Shrink),
        progress_slider,
        text(format_duration(duration))
            .size(12)
            .width(Length::Shrink),
        fullscreen_button,
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .padding(Padding {
        top: 4.0,
        bottom: 4.0,
        left: 8.0,
        right: 8.0,
    })
    .width(Length::Fill);

    container(controls_row)
        .width(Length::Fill)
        .style(glass_container_style)
        .into()
}
