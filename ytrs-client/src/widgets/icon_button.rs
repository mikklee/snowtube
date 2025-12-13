//! Reusable circular icon button with tooltip

use crate::messages::Message;
use crate::theme::circular_button_style;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use iced::widget::{button, container, text, tooltip};
use iced::{Element, Length, Padding, Theme};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::icons::{ICON_HEADPHONES, ICON_VIDEO, NERD_FONT};

// Star icons for subscribe button
const STAR_OUTLINE: char = '\u{2606}'; // ☆
const STAR_FILLED: char = '\u{2605}'; // ★

// Nerd Font bytes embedded at compile time
const NERD_FONT_BYTES: &[u8] = include_bytes!("../../fonts/JetBrainsMonoNerdFont-Regular.ttf");

// Cache for glyph offset calculations
static OFFSET_CACHE: LazyLock<Mutex<HashMap<(char, u32), (f32, f32)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// Icons that need X-axis correction (mdi icons that render off-center)
const ICONS_NEEDING_X_CORRECTION: &[char] = &[ICON_HEADPHONES, ICON_VIDEO];

/// Calculate the offset needed to center a nerd font glyph
fn get_glyph_offset(icon: char, size: f32) -> (f32, f32) {
    // Only apply correction to icons that need it
    if !ICONS_NEEDING_X_CORRECTION.contains(&icon) {
        return (0.0, 0.0);
    }

    let size_key = size as u32;

    // Check cache first
    if let Ok(cache) = OFFSET_CACHE.lock() {
        if let Some(offset) = cache.get(&(icon, size_key)) {
            return *offset;
        }
    }

    let font = FontRef::try_from_slice(NERD_FONT_BYTES).expect("Failed to load nerd font");
    let scaled_font = font.as_scaled(PxScale::from(size));

    let glyph_id = font.glyph_id(icon);
    let glyph = glyph_id.with_scale(PxScale::from(size));

    let offset = if let Some(outlined) = font.outline_glyph(glyph) {
        let bounds = outlined.px_bounds();

        // Calculate how much the glyph is off-center horizontally
        let glyph_width = bounds.max.x - bounds.min.x;
        let glyph_center_x = bounds.min.x + glyph_width / 2.0;

        // The expected center based on advance width
        let advance = scaled_font.h_advance(glyph_id);
        let expected_center_x = advance / 2.0;

        // For Y: add the full glyph height to the top padding (bounds.min.y)
        let glyph_height = bounds.max.y - bounds.min.y;
        let offset_y = bounds.min.y + glyph_height;

        // Offset to apply
        let offset_x = glyph_center_x - expected_center_x;

        (offset_x, offset_y)
    } else {
        (0.0, 0.0)
    };

    // Cache the result
    if let Ok(mut cache) = OFFSET_CACHE.lock() {
        cache.insert((icon, size_key), offset);
    }

    offset
}

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

    let button_content: Element<'a, Message, Theme> = if use_nerd_font {
        // Get offset to center the glyph properly
        let (offset_x, offset_y) = get_glyph_offset(icon, icon_size);

        let icon_text = text(icon.to_string())
            .size(icon_size)
            .font(NERD_FONT)
            .center();

        // Apply padding to counteract the glyph's off-center position
        let padding = Padding {
            top: (-offset_y).max(0.0),
            bottom: offset_y.max(0.0),
            left: (-offset_x).max(0.0),
            right: offset_x.max(0.0),
        };

        container(icon_text)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(padding)
            .center(Length::Fill)
            .into()
    } else {
        let icon_text = text(icon.to_string()).size(icon_size).center();
        container(icon_text)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .into()
    };

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
