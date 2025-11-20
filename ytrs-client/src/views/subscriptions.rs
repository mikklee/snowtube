//! Subscriptions view for the ytrs-client application

use iced::{
    Alignment::Center,
    Element, Length,
    widget::{Image, button, column, container, row, scrollable, text},
};
use iced_aw::Wrap;

use crate::App;
use crate::messages::Message;

/// Render the subscriptions view
pub fn view(app: &App) -> Element<'_, Message> {
    let back_button = button(text("← Back"))
        .on_press(Message::BackToSearch)
        .padding(10);

    let header = container(
        row![back_button, text("Subscriptions").size(24),]
            .align_y(Center)
            .spacing(20),
    )
    .padding(20)
    .width(Length::Fill);

    let body: Element<Message> = if app.config.subscriptions.is_empty() {
        container(
            column![
                text("No subscriptions yet").size(20),
                text("Subscribe to channels to see them here").size(14)
            ]
            .spacing(10)
            .align_x(Center),
        )
        .padding(60)
        .center_x(Length::Fill)
        .into()
    } else {
        let channel_cards: Vec<Element<Message>> = app
            .config
            .subscriptions
            .iter()
            .filter_map(|sub| {
                let channel_id = sub.channel_id.clone();
                let handle = app.subscription_thumbs.get(&channel_id)?.clone();

                let name = sub.channel_name.clone();
                let display_name = if let Some(ref h) = sub.channel_handle {
                    format!("{}\n{}", name, h)
                } else {
                    name
                };

                // Avatar is already circular from load_circular_thumb
                let avatar = Image::new(handle).width(80).height(80);

                let channel_name_text = text(display_name).size(14).align_x(Center).width(120);

                let card = button(
                    column![avatar, channel_name_text]
                        .align_x(Center)
                        .spacing(10)
                        .width(120),
                )
                .on_press(Message::ViewChannel(channel_id))
                .padding(10);

                Some(container(card).into())
            })
            .collect();

        let grid = Wrap::with_elements(channel_cards)
            .spacing(15.0)
            .line_spacing(15.0);

        scrollable(container(grid).padding(20).width(Length::Fill)).into()
    };

    column![header, body].into()
}
