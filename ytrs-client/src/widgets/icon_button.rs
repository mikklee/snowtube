//! Reusable circular icon button with tooltip

use crate::messages::Message;
use crate::theme::circular_button_style;
use crate::widgets::icons::{icon_refresh, icon_star, icon_star_outline};
use iced::widget::{button, container, text, tooltip};
use iced::{Element, Length, Theme};
use iced_font_awesome::Icon;

/// Creates a circular icon button with a tooltip
///
/// - `icon`: The icon element to display
/// - `size`: Button size (width and height)
/// - `tooltip_text`: Text to show on hover
/// - `on_press`: Message to emit when clicked
pub fn icon_button<'a, Message: Clone + 'a>(
    icon: Element<'a, Message, Theme>,
    size: f32,
    tooltip_text: &'a str,
    on_press: Message,
) -> Element<'a, Message, Theme> {
    let button_content = container(icon)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill);

    tooltip(
        button(button_content)
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

pub fn gen_icon_button(
    size: f32,
    icon_fn: fn(f32) -> Icon<'static, Theme>,
    tooltip: &'static str,
    message: Message,
) -> Element<'static, Message, Theme> {
    let icon_size = size * 0.5;
    icon_button(icon_fn(icon_size).into(), size, tooltip, message)
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
    if is_subscribed {
        gen_icon_button(
            size,
            icon_star,
            "Unsubscribe",
            Message::UnsubscribeFromChannel(channel_id),
        )
    } else {
        gen_icon_button(
            size,
            icon_star_outline,
            "Subscribe",
            Message::SubscribeToChannel,
        )
    }
}

pub fn refresh_subs_button(size: f32) -> Element<'static, Message, Theme> {
    gen_icon_button(
        size,
        icon_refresh,
        "Refresh subscriptions",
        Message::RefreshSubscriptionVideos,
    )
}
