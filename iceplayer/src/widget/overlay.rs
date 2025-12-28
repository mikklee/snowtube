//! Overlay components for the video player.
//!
//! Includes loading placeholder, seeking overlay, title overlay, centered play button, etc.

use super::controls::glass_container_style;
use super::spinner::spinner;
use iced::widget::{button, column, container, stack, text};
use iced::{Color, Element, Length, Padding, Renderer, Theme};
use iced_font_awesome::fa_icon_solid;

/// Dark semi-transparent overlay background.
/// Used for loading and seeking states.
pub fn dark_overlay<'a, Message: 'a>(
    content: Element<'a, Message, Theme, Renderer>,
) -> Element<'a, Message, Theme, Renderer> {
    container(content)
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

/// Spinner with optional status text, centered.
/// Used as building block for loading states.
pub fn spinner_with_text<'a, Message: 'static>(
    status: Option<&'a str>,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let spinner_widget: Element<'a, Message, Theme, Renderer> = spinner(48.0, theme);

    let mut content = column![spinner_widget]
        .spacing(16)
        .align_x(iced::Alignment::Center);

    if let Some(s) = status {
        content = content.push(text(s).size(14).color(Color::WHITE));
    }

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Loading overlay with spinner, status text, and dark background.
pub fn loading_overlay<'a, Message: 'static>(
    status: Option<&'a str>,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    dark_overlay(spinner_with_text(status, theme))
}

/// Seeking overlay with spinner and status text.
pub fn seeking_overlay<Message: 'static>(
    theme: &Theme,
) -> Element<'static, Message, Theme, Renderer> {
    let spinner_widget: Element<'static, Message, Theme, Renderer> = spinner(48.0, theme);

    let seeking_content = column![
        spinner_widget,
        text("Seeking. This may take a minute.")
            .size(16)
            .color(Color::WHITE),
        text("If it takes longer, try reloading the video.")
            .size(14)
            .color(Color::WHITE)
    ]
    .spacing(16)
    .align_x(iced::Alignment::Center);

    dark_overlay(seeking_content.into())
}

/// Title overlay at the top of the video with text shadow.
pub fn title_overlay<'a, Message: 'a>(title: &'a str) -> Element<'a, Message, Theme, Renderer> {
    let shadow_text = text(title).size(18).color(Color::BLACK);
    let main_text = text(title).size(18).color(Color::WHITE);
    let title_with_shadow = stack![
        container(shadow_text).padding(Padding {
            top: 1.0,
            bottom: 0.0,
            left: 1.0,
            right: 0.0
        }),
        main_text,
    ];

    container(title_with_shadow)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Top)
        .align_x(iced::alignment::Horizontal::Left)
        .padding(Padding::from([15, 20]))
        .into()
}

/// Notification overlay in the center of the video.
pub fn notification_overlay<'a, Message: 'a>(
    message: &'a str,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    container(
        container(text(message).size(14).color(Color::WHITE))
            .padding(Padding::new(12.0))
            .style(glass_container_style),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

/// Centered play button overlay shown when video is loaded but paused.
/// This is the initial state after loading completes.
pub fn centered_play_button<'a, Message: Clone + 'a>(
    on_play: Message,
    theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let palette = theme.palette();

    // Button size (diameter of the circle)
    const BUTTON_SIZE: f32 = 80.0;

    let play_icon = fa_icon_solid("play")
        .size(32.0)
        .style(|_| iced::widget::text::Style {
            color: Some(Color::WHITE),
        });

    let play_button = button(
        container(play_icon)
            .width(Length::Fixed(BUTTON_SIZE))
            .height(Length::Fixed(BUTTON_SIZE))
            .center_x(Length::Fixed(BUTTON_SIZE))
            .center_y(Length::Fixed(BUTTON_SIZE)),
    )
    .on_press(on_play)
    .padding(0)
    .style(move |_theme, status| {
        use iced::widget::button;

        let bg_alpha = match status {
            button::Status::Hovered => 0.9,
            button::Status::Pressed => 1.0,
            _ => 0.7,
        };

        button::Style {
            background: Some(iced::Background::Color(Color {
                r: palette.primary.r,
                g: palette.primary.g,
                b: palette.primary.b,
                a: bg_alpha,
            })),
            text_color: Color::WHITE,
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: (BUTTON_SIZE / 2.0).into(),
            },
            shadow: iced::Shadow {
                color: Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.3,
                },
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            snap: false,
        }
    });

    container(play_button)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

/// Loading placeholder with spinner and status text.
/// Shows a black background with centered spinner.
/// Uses slightly transparent white text (0.7 alpha) for status.
pub fn loading_placeholder<'a, Message: 'static>(
    status: Option<&'a str>,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    // Reuse spinner_with_text - the slight color difference (0.7 vs 1.0 alpha) is acceptable
    spinner_with_text(status, theme)
}

/// Error overlay with error message.
pub fn error_overlay<'a, Message: 'a>(
    error: &'a str,
    _theme: &Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let error_content = column![
        fa_icon_solid("triangle-exclamation")
            .size(48.0)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb(1.0, 0.4, 0.4)),
            }),
        text("Error loading video").size(18).color(Color::WHITE),
        text(error).size(14).color(Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.7,
        }),
    ]
    .spacing(12)
    .align_x(iced::Alignment::Center);

    container(error_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}
