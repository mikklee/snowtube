//! Icon constants and font for Nerd Font icons

use iced::Font;

/// Nerd Font for icons
pub const NERD_FONT: Font = Font {
    family: iced::font::Family::Name("JetBrainsMono Nerd Font"),
    ..Font::DEFAULT
};

// Common icons
pub const ICON_COPY: char = '\u{f0c5}'; // nf-fa-copy
pub const ICON_PLAY: char = '\u{f04b}'; // nf-fa-play (for MPV)
pub const ICON_HEADPHONES: char = '\u{f025}'; // nf-fa-headphones (for audio-only)
