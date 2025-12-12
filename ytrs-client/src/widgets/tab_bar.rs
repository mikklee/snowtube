//! TabBar widget with rounded corners, transparency, and icons

use iced::widget::{button, column, container, row, text};
use iced::{Color, Element, Length, Padding, Theme};

use super::glass::glass_container_style;
use super::icons::NERD_FONT;
use crate::messages::{Message, TabId};

/// Configuration for a tab bar item
pub struct TabItem {
    pub id: TabId,
    pub label: &'static str,
    pub icon: char,
    pub icon_size: f32,
}

/// iOS-style tab bar with rounded corners and transparency
pub fn tab_bar<'a>(active_tab: TabId, items: &[TabItem]) -> Element<'a, Message> {
    let tab_buttons: Vec<Element<'a, Message>> = items
        .iter()
        .map(|item| {
            let is_active = active_tab == item.id;
            tab_button(item.id, item.label, item.icon, item.icon_size, is_active)
        })
        .collect();

    // Create the tab bar container with glass effect styling
    let tabs_row = row(tab_buttons).spacing(0).width(Length::Fill);

    // Inner glass container with max width
    let glass_bar = container(tabs_row)
        .padding(Padding::new(8.0))
        .max_width(800.0)
        .width(Length::Fill)
        .style(glass_container_style);

    // Outer container centers the tab bar and is transparent
    container(glass_bar)
        .padding(Padding {
            top: 8.0,
            bottom: 16.0, // Extra padding at bottom for safe area
            left: 12.0,
            right: 12.0,
        })
        .width(Length::Fill)
        .center_x(Length::Fill)
        .style(|_| container::Style {
            background: None, // Transparent outer container
            ..Default::default()
        })
        .into()
}

/// Create a single tab button with icon and label
fn tab_button(
    id: TabId,
    label: &'static str,
    icon: char,
    icon_size: f32,
    is_active: bool,
) -> Element<'static, Message> {
    let content = column![
        text(icon.to_string())
            .size(icon_size)
            .font(NERD_FONT)
            .width(Length::Fill)
            .center(),
        text(format!("  {}", label))
            .size(14)
            .width(Length::Fill)
            .center(),
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
    .style(move |theme, status| tab_button_style(theme, status, is_active))
    .on_press(Message::TabSelected(id))
    .into()
}

/// Custom button style for tab items
fn tab_button_style(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let palette = theme.palette();

    let (background, text_color) = if is_active {
        // Active tab: subtle highlight
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
        // Inactive tab: transparent
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

/// Default tab items for the app
pub fn default_tab_items() -> [TabItem; 3] {
    [
        TabItem {
            id: TabId::Search,
            label: "Search",
            icon: '\u{f002}', // nf-fa-search
            icon_size: 24.0,
        },
        TabItem {
            id: TabId::Channels,
            label: "Channels",
            icon: '\u{f005}', // nf-fa-star
            icon_size: 24.0,
        },
        TabItem {
            id: TabId::Settings,
            label: "Settings",
            icon: '\u{f013}', // nf-fa-cog
            icon_size: 24.0,
        },
    ]
}
