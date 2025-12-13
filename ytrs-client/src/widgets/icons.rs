//! Icon elements using Font Awesome via iced_font_awesome

use iced::Theme;
use iced::widget::text::Style;
use iced_font_awesome::{Icon, fa_icon_solid};

/// Get the text color from the theme for icons
pub fn text_style(theme: &Theme) -> Style {
    Style {
        color: Some(theme.palette().text.inverse()),
    }
}

// Icon functions using Font Awesome with theme colors
// Returns Icon<'a, Theme> to allow further customization before converting to Element
pub fn icon_copy(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("copy").size(size).style(text_style)
}

pub fn icon_play(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("play").size(size).style(text_style)
}

pub fn icon_video(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("video").size(size).style(text_style)
}

pub fn icon_search(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("magnifying-glass")
        .size(size)
        .style(text_style)
}

pub fn icon_star(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("star").size(size).style(text_style)
}

pub fn icon_cog(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("gear").size(size).style(text_style)
}

pub fn icon_star_outline(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("star").size(size).style(text_style)
}

pub fn icon_headphones(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("headphones").size(size).style(text_style)
}

pub fn icon_refresh(size: f32) -> Icon<'static, Theme> {
    fa_icon_solid("arrows-rotate").size(size).style(text_style)
}
