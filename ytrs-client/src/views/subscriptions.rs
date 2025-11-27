//! Channels (subscriptions) view for the ytrs-client application

use crate::App;
use crate::messages::Message;
use crate::widgets::{Wrap, bounceable_scrollable};
use iced::{
    Alignment::Center,
    Element, Length,
    widget::text::Shaping,
    widget::{Image, button, column, container, lazy, text},
};

/// Render the channels (subscriptions) view
pub fn view(app: &App) -> Element<'_, Message> {
    let _start = std::time::Instant::now();

    let header = container(text("Channels").size(24).shaping(Shaping::Advanced))
        .padding(20)
        .width(Length::Fill);

    let body: Element<Message> = if app.config.subscriptions.is_empty() {
        container(
            column![
                text("No channels yet").size(20).shaping(Shaping::Advanced),
                text("Subscribe to channels from search to see them here")
                    .size(14)
                    .shaping(Shaping::Advanced)
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
                let channel_handle = sub.channel_handle.clone();

                // Use lazy to cache the card rendering
                Some(
                    lazy(channel_id.clone(), move |_| {
                        let display_name = if let Some(ref h) = channel_handle {
                            format!("{}\n{}", name, h)
                        } else {
                            name.clone()
                        };

                        // Avatar is already circular from load_circular_thumb
                        let avatar = Image::new(handle.clone()).width(80).height(80);

                        let channel_name_text = text(display_name)
                            .size(14)
                            .shaping(Shaping::Advanced)
                            .align_x(Center)
                            .width(120);

                        let card = button(
                            column![avatar, channel_name_text]
                                .align_x(Center)
                                .spacing(10)
                                .width(120),
                        )
                        .on_press(Message::ViewChannel(channel_id.clone()))
                        .padding(10);

                        container(card)
                    })
                    .into(),
                )
            })
            .collect();

        let grid = Wrap::with_elements(channel_cards)
            .spacing(15.0)
            .line_spacing(15.0);

        let result = bounceable_scrollable(
            container(grid)
                .padding(iced::Padding {
                    top: 20.0,
                    bottom: 100.0, // Extra space for tab bar overlay
                    left: 20.0,
                    right: 20.0,
                })
                .width(Length::Fill),
        )
        .id("subscriptions")
        .into();

        eprintln!("  Subscriptions view TOTAL: {:?}", _start.elapsed());
        eprintln!(
            "    - Total subscriptions: {}",
            app.config.subscriptions.len()
        );

        result
    };

    column![header, body].into()
}
