//! Glass container style for floating UI elements

use iced::widget::container;
use iced::{Color, Theme};

/// Style for the glass container effect (frosted glass simulation)
/// Note: True backdrop blur would require modifying iced's wgpu rendering pipeline
pub fn glass_container_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    // iOS-style frosted glass - use background color with transparency
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
