//! Configuration view for the ytrs-client application

use iced::{
    Alignment, Background, Color, Element, Length, Theme,
    widget::{column, combo_box, container, pick_list, row, text},
};

use crate::widgets::bounceable_scrollable;
use strum::IntoEnumIterator;

use crate::App;
use crate::helpers::{ChannelInfo, create_video_tile};
use crate::messages::Message;
use crate::theme::AppTheme;

/// Create a mock video tile preview to show how the theme looks
fn create_theme_preview() -> Element<'static, Message> {
    // Mock thumbnail - simulating a real thumbnail with duration badge
    let mock_thumbnail = container(
        column![iced::widget::space::vertical().height(Length::Fill),]
            .align_x(Alignment::End)
            .padding(4),
    )
    .width(240)
    .height(135)
    .style(|_theme: &Theme| container::Style {
        background: Some(Background::Color(Color::BLACK)),
        ..Default::default()
    });

    create_video_tile(
        mock_thumbnail.into(),
        "Example video tile",
        Some(ChannelInfo {
            name: "Lorem Lipsum",
            on_press: Some(Message::NoOp),
        }),
        Some("20.4K views • 14:46".to_string()),
        Message::NoOp,
    )
}

/// Render the configuration view
pub fn view(app: &App) -> Element<'_, Message> {
    let title = text("Configuration").size(32);

    let header = container(title).padding(20).width(Length::Fill);

    // Language Section
    let language_section_title = text("Default Language").size(20);

    let language_explanation = text(
        "Sets the default language for search results and channel videos. \
         Auto-detect will use the language from channel metadata. \
         You can still override this in Search and Channel views.",
    )
    .size(14);

    let language_row = row![
        text("Language:").size(14),
        combo_box(
            &app.language_combo_state,
            "Auto-detect",
            app.selected_language.as_ref(),
            Message::LanguageSelected,
        )
        .width(250)
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let language_section = column![
        language_section_title,
        iced::widget::space::vertical().height(10),
        language_explanation,
        iced::widget::space::vertical().height(20),
        language_row,
    ]
    .spacing(5);

    // Theme Section
    let theme_section_title = text("Theme").size(20);

    let theme_explanation = text(
        "Choose your preferred color theme for the application. \
         The theme will be applied immediately and saved for future sessions.",
    )
    .size(14);

    let theme_options: Vec<AppTheme> = AppTheme::iter().collect();
    let theme_row = row![
        text("Theme:").size(14),
        pick_list(theme_options, Some(app.config.theme), Message::ThemeChanged).padding(5)
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    // Theme preview
    let preview_title = text("Preview:").size(14);
    let preview = create_theme_preview();

    let theme_section = column![
        theme_section_title,
        iced::widget::space::vertical().height(10),
        theme_explanation,
        iced::widget::space::vertical().height(20),
        theme_row,
        iced::widget::space::vertical().height(15),
        preview_title,
        iced::widget::space::vertical().height(10),
        preview,
    ]
    .spacing(5);

    let content = column![
        header,
        container(
            column![
                language_section,
                iced::widget::space::vertical().height(30),
                theme_section,
            ]
            .padding(20)
        )
        .width(Length::Fill)
    ];

    bounceable_scrollable(container(content).padding(iced::Padding {
        top: 0.0,
        bottom: 100.0, // Extra space for tab bar overlay
        left: 0.0,
        right: 0.0,
    }))
    .id("config")
    .into()
}
