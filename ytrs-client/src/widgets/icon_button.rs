//! Reusable circular icon button with tooltip

use crate::messages::Message;
use crate::theme::circular_button_style;
use iced::widget::{button, container, text, tooltip};
use iced::{Element, Length, Theme};

use super::icons::NERD_FONT;

// Star icons for subscribe button
const STAR_OUTLINE: char = '\u{2606}'; // ☆
const STAR_FILLED: char = '\u{2605}'; // ★

/// Creates a circular icon button with a tooltip
///
/// - `icon`: The icon character to display
/// - `size`: Button size (width and height)
/// - `tooltip_text`: Text to show on hover
/// - `use_nerd_font`: Whether to use Nerd Font (false uses default font for Unicode symbols)
/// - `on_press`: Message to emit when clicked
pub fn icon_button<'a, Message: Clone + 'a>(
    icon: char,
    size: f32,
    tooltip_text: &'a str,
    use_nerd_font: bool,
    on_press: Message,
) -> Element<'a, Message, Theme> {
    let icon_size = size * 0.5;

    let icon_text = if use_nerd_font {
        text(icon.to_string())
            .size(icon_size)
            .font(NERD_FONT)
            .center()
    } else {
        text(icon.to_string()).size(icon_size).center()
    };

    tooltip(
        button(
            container(icon_text)
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill),
        )
        .width(size)
        .height(size)
        .on_press(on_press)
        .style(circular_button_style),
        container(text(tooltip_text))
            .style(container::dark)
            .padding(6),
        tooltip::Position::Top,
    )
    .into()
}

/// Creates a subscribe/unsubscribe button with star icon
///
/// - `is_subscribed`: Whether the channel is currently subscribed
/// - `channel_id`: The channel ID
/// - `size`: Button size (width and height)
pub fn subscribe_button(
    is_subscribed: bool,
    channel_id: String,
    size: f32,
) -> Element<'static, Message, Theme> {
    let (icon, tip, msg) = if is_subscribed {
        (
            STAR_FILLED,
            "Unsubscribe",
            Message::UnsubscribeFromChannel(channel_id),
        )
    } else {
        (STAR_OUTLINE, "Subscribe", Message::SubscribeToChannel)
    };
    icon_button(icon, size, tip, false, msg)
}
