//! Theme definitions and utilities for the ytrs-client application

use iced::widget::{button, pick_list, text_input};
use iced::{Border, Theme};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

/// Default border radius for UI elements
pub const BORDER_RADIUS: f32 = 48.0;

/// Available application themes
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Display,
    EnumIter,
    EnumString,
    Default,
)]
pub enum AppTheme {
    #[default]
    Cyberpunk,
    Light,
    Dark,
    Dracula,
    Nord,
    SolarizedLight,
    SolarizedDark,
    GruvboxLight,
    GruvboxDark,
    TokyoNight,
    TokyoNightStorm,
    KanagawaWave,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
}

impl AppTheme {
    /// Convert AppTheme to iced Theme
    pub fn to_iced_theme(self) -> Theme {
        match self {
            AppTheme::Cyberpunk => Theme::custom("Cyberpunk".to_string(), cyberpunk_palette()),
            AppTheme::Light => Theme::Light,
            AppTheme::Dark => Theme::Dark,
            AppTheme::Dracula => Theme::Dracula,
            AppTheme::Nord => Theme::Nord,
            AppTheme::SolarizedLight => Theme::SolarizedLight,
            AppTheme::SolarizedDark => Theme::SolarizedDark,
            AppTheme::GruvboxLight => Theme::GruvboxLight,
            AppTheme::GruvboxDark => Theme::GruvboxDark,
            AppTheme::TokyoNight => Theme::TokyoNight,
            AppTheme::TokyoNightStorm => Theme::TokyoNightStorm,
            AppTheme::KanagawaWave => Theme::KanagawaWave,
            AppTheme::CatppuccinLatte => Theme::CatppuccinLatte,
            AppTheme::CatppuccinFrappe => Theme::CatppuccinFrappe,
            AppTheme::CatppuccinMacchiato => Theme::CatppuccinMacchiato,
            AppTheme::CatppuccinMocha => Theme::CatppuccinMocha,
        }
    }
}

/// Custom Cyberpunk theme palette
fn cyberpunk_palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: iced::Color::from_rgb(0.08, 0.08, 0.12),
        text: iced::Color::from_rgb(0.95, 0.95, 0.98),
        primary: iced::Color::from_rgb(0.5, 0.4, 0.9),
        success: iced::Color::from_rgb(0.3, 0.8, 0.6),
        danger: iced::Color::from_rgb(0.9, 0.3, 0.4),
        warning: iced::Color::from_rgb(0.9, 0.7, 0.3),
    }
}

/// Rounded button style
pub fn rounded_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = button::primary(theme, status);
    style.border = Border {
        radius: BORDER_RADIUS.into(),
        ..style.border
    };
    style
}

/// Rounded text input style with transparent background
pub fn rounded_text_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.border = Border {
        radius: BORDER_RADIUS.into(),
        ..style.border
    };
    style.background = iced::Background::Color(iced::Color::TRANSPARENT);
    style
}

/// Rounded combo box style (combo_box uses text_input style)
pub fn rounded_combo_box_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.border = Border {
        radius: BORDER_RADIUS.into(),
        ..style.border
    };
    style
}

/// Rounded pick list style
pub fn rounded_pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let mut style = pick_list::default(theme, status);
    style.border = Border {
        radius: BORDER_RADIUS.into(),
        ..style.border
    };
    style
}
