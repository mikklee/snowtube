//! Video control bar with play/pause, progress slider, time display, and fullscreen toggle.

use common::Subtitle;
use iced::widget::{button, container, pick_list, row, slider, text};
use iced::{Border, Color, Element, Length, Padding, Renderer, Theme};
use iced_font_awesome::fa_icon_solid;
use std::time::Duration;

/// Parameters for rendering a control bar
pub struct ControlBarParams<'a, Message: Clone + 'a> {
    pub is_paused: bool,
    pub position: Duration,
    pub duration: Duration,
    pub seek_preview: Option<f64>,
    pub on_toggle_play: Message,
    pub on_seek_preview: Box<dyn Fn(f64) -> Message + 'a>,
    pub on_seek_release: Message,
    pub on_toggle_fullscreen: Message,
    /// Available subtitles
    pub subtitles: &'a [Subtitle],
    /// Currently selected subtitle index (None = off)
    pub selected_subtitle: Option<usize>,
    /// Callback when subtitle is selected
    pub on_select_subtitle: Box<dyn Fn(Option<usize>) -> Message + 'a>,
}

/// Subtitle option for the pick_list
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleOption {
    Off,
    Track { index: usize, name: String },
}

impl std::fmt::Display for SubtitleOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubtitleOption::Off => write!(f, "Off"),
            SubtitleOption::Track { name, .. } => write!(f, "{}", name),
        }
    }
}

pub fn play_button<'a, Message: Clone + 'a>(
    on_press: Message,
    disabled: bool,
) -> Element<'a, Message, Theme, Renderer> {
    icon_button("play", on_press, disabled)
}

pub fn pause_button<'a, Message: Clone + 'a>(
    on_press: Message,
    disabled: bool,
) -> Element<'a, Message, Theme, Renderer> {
    icon_button("pause", on_press, disabled)
}

pub fn fullscreen_button<'a, Message: Clone + 'a>(
    on_press: Message,
    disabled: bool,
) -> Element<'a, Message, Theme, Renderer> {
    icon_button("expand", on_press, disabled)
}

pub fn exit_fullscreen_button<'a, Message: Clone + 'a>(
    on_press: Message,
    disabled: bool,
) -> Element<'a, Message, Theme, Renderer> {
    icon_button("compress", on_press, disabled)
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

/// Style for subtitle pick_list in control bar
fn subtitle_pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let palette = theme.palette();
    let is_hovered = matches!(
        status,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. }
    );

    pick_list::Style {
        text_color: if is_hovered {
            palette.primary
        } else {
            Color {
                a: 0.9,
                ..palette.text
            }
        },
        placeholder_color: Color {
            a: 0.5,
            ..palette.text
        },
        handle_color: if is_hovered {
            palette.primary
        } else {
            Color {
                a: 0.7,
                ..palette.text
            }
        },
        background: iced::Background::Color(Color::TRANSPARENT),
        border: Border {
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}

/// Create subtitle picker element if subtitles are available
fn subtitle_picker<'a, Message: Clone + 'a>(
    subtitles: &[Subtitle],
    selected: Option<usize>,
    on_select: impl Fn(Option<usize>) -> Message + 'a,
) -> Option<Element<'a, Message, Theme, Renderer>> {
    if subtitles.is_empty() {
        return None;
    }

    let options: Vec<SubtitleOption> = std::iter::once(SubtitleOption::Off)
        .chain(
            subtitles
                .iter()
                .enumerate()
                .map(|(i, s)| SubtitleOption::Track {
                    index: i,
                    name: s.language_name.clone(),
                }),
        )
        .collect();

    let current = match selected {
        None => SubtitleOption::Off,
        Some(idx) => subtitles
            .get(idx)
            .map(|s| SubtitleOption::Track {
                index: idx,
                name: s.language_name.clone(),
            })
            .unwrap_or(SubtitleOption::Off),
    };

    Some(
        pick_list(options, Some(current), move |opt| match opt {
            SubtitleOption::Off => on_select(None),
            SubtitleOption::Track { index, .. } => on_select(Some(index)),
        })
        .padding(Padding {
            top: 4.0,
            bottom: 4.0,
            left: 8.0,
            right: 8.0,
        })
        .style(subtitle_pick_list_style)
        .into(),
    )
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

fn icon_button<'a, Message: Clone + 'a>(
    icon_name: &'static str,
    message: Message,
    disabled: bool,
) -> Element<'a, Message, Theme, Renderer> {
    let icon = fa_icon_solid(icon_name).size(20.0);

    let btn = button(
        container(icon)
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
    params: ControlBarParams<'a, Message>,
) -> Element<'a, Message, Theme, Renderer> {
    // Progress calculation
    let (display_percent, display_position) = if let Some(preview) = params.seek_preview {
        let preview_duration = Duration::from_secs_f64(params.duration.as_secs_f64() * preview);
        (preview, preview_duration)
    } else {
        let progress_percent = if params.duration.as_secs_f64() > 0.0 {
            (params.position.as_secs_f64() / params.duration.as_secs_f64()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (progress_percent, params.position)
    };

    // Only enable seeking if video has started playing (position > 0)
    let can_seek = params.duration.as_secs_f64() > 0.0 && params.position.as_millis() > 0;
    let on_toggle_play = params.on_toggle_play.clone();

    let progress_slider: Element<'a, Message, Theme, Renderer> = if can_seek {
        slider(0.0..=1.0, display_percent, params.on_seek_preview)
            .step(0.001)
            .on_release(params.on_seek_release)
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    } else {
        slider(0.0..=1.0, display_percent, move |_| on_toggle_play.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    };

    // Build controls row with optional subtitle picker
    let play_pause = if params.is_paused {
        play_button(params.on_toggle_play.clone(), false)
    } else {
        pause_button(params.on_toggle_play.clone(), false)
    };

    let mut controls = row![
        play_pause,
        text(format_duration(display_position))
            .size(12)
            .width(Length::Shrink),
        progress_slider,
        text(format_duration(params.duration))
            .size(12)
            .width(Length::Shrink),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // Add subtitle picker if subtitles are available
    if let Some(picker) = subtitle_picker(
        params.subtitles,
        params.selected_subtitle,
        params.on_select_subtitle,
    ) {
        controls = controls.push(picker);
    }

    controls = controls.push(fullscreen_button(params.on_toggle_fullscreen, false));

    let controls_row = controls
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
    params: ControlBarParams<'a, Message>,
) -> Element<'a, Message, Theme, Renderer> {
    // Progress calculation
    let (display_percent, display_position) = if let Some(preview) = params.seek_preview {
        let preview_duration = Duration::from_secs_f64(params.duration.as_secs_f64() * preview);
        (preview, preview_duration)
    } else {
        let progress_percent = if params.duration.as_secs_f64() > 0.0 {
            (params.position.as_secs_f64() / params.duration.as_secs_f64()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (progress_percent, params.position)
    };

    let can_seek = params.duration.as_secs_f64() > 0.0 && params.position.as_millis() > 0;
    let on_toggle_play = params.on_toggle_play.clone();

    let progress_slider: Element<'a, Message, Theme, Renderer> = if can_seek {
        slider(0.0..=1.0, display_percent, params.on_seek_preview)
            .step(0.001)
            .on_release(params.on_seek_release)
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    } else {
        slider(0.0..=1.0, display_percent, move |_| on_toggle_play.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into()
    };

    // Build controls row with optional subtitle picker
    let play_pause = if params.is_paused {
        play_button(params.on_toggle_play.clone(), false)
    } else {
        pause_button(params.on_toggle_play.clone(), false)
    };

    let mut controls = row![
        play_pause,
        text(format_duration(display_position))
            .size(12)
            .color(Color::WHITE)
            .width(Length::Shrink),
        progress_slider,
        text(format_duration(params.duration))
            .size(12)
            .color(Color::WHITE)
            .width(Length::Shrink),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    // Add subtitle picker if subtitles are available
    if let Some(picker) = subtitle_picker(
        params.subtitles,
        params.selected_subtitle,
        params.on_select_subtitle,
    ) {
        controls = controls.push(picker);
    }

    controls = controls.push(exit_fullscreen_button(params.on_toggle_fullscreen, false));

    let controls_row = controls
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
    let play_btn = play_button(on_toggle_play.clone(), true);
    let progress_slider: Element<'a, Message, Theme, Renderer> =
        slider(0.0..=1.0, 0.0, move |_| on_toggle_play.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into();

    let controls_row = row![
        play_btn,
        text("00:00").size(12).width(Length::Shrink),
        progress_slider,
        text(format_duration(duration))
            .size(12)
            .width(Length::Shrink),
        fullscreen_button(on_toggle_fullscreen, true),
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
    let play_btn = play_button(on_start_playback.clone(), false);
    let progress_slider: Element<'a, Message, Theme, Renderer> =
        slider(0.0..=1.0, 0.0, move |_| on_start_playback.clone())
            .width(Length::Fill)
            .style(progress_slider_style)
            .into();

    let controls_row = row![
        play_btn,
        text("00:00").size(12).width(Length::Shrink),
        progress_slider,
        text(format_duration(duration))
            .size(12)
            .width(Length::Shrink),
        fullscreen_button(on_toggle_fullscreen, false),
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
