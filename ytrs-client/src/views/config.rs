//! Configuration view for the ytrs-client application

use iced::{
    Alignment, Element, Length,
    widget::{button, column, combo_box, container, row, scrollable, text},
};

use crate::App;
use crate::messages::Message;

/// Render the configuration view
pub fn view(app: &App) -> Element<'_, Message> {
    let title = text("Configuration").size(32).color(iced::Color::WHITE);

    let header = container(title).padding(20).width(Length::Fill);

    // Language Section
    let language_section_title = text("Default Language").size(20).color(iced::Color::WHITE);

    let language_explanation = text(
        "Sets the default language for search results and channel videos. \
         Auto-detect will use the language from channel metadata. \
         You can still override this in Search and Channel views.",
    )
    .size(14)
    .color(iced::Color::from_rgb(0.8, 0.8, 0.85));

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

    // Back button
    let back_button = button(text("← Back"))
        .on_press(Message::CloseConfig)
        .padding(10);

    let content = column![
        header,
        container(
            column![
                language_section,
                iced::widget::space::vertical().height(30),
                back_button,
            ]
            .padding(20)
        )
        .width(Length::Fill)
    ];

    scrollable(content).into()
}
