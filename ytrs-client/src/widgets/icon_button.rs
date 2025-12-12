//! Reusable circular icon button with tooltip

use crate::theme::circular_button_style;
use iced::widget::{button, container, text, tooltip};
use iced::{Element, Length, Theme};

use super::icons::NERD_FONT;

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
    let inner_size = size * 0.5;

    let icon_text = if use_nerd_font {
        text(icon.to_string()).size(icon_size).font(NERD_FONT)
    } else {
        text(icon.to_string()).size(icon_size)
    };

    tooltip(
        button(
            container(icon_text)
                .width(inner_size)
                .height(inner_size)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
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
