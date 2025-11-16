//! Search view for the ytrs-client application

use iced::{
    Alignment::{self, Center},
    Element, Length,
    widget::{
        Image, button, column, combo_box, container, lazy, row, scrollable, text, text_input,
    },
};
use iced_aw::Wrap;

use crate::App;
use crate::helpers::{create_thumbnail, fmt_num, truncate_title};
use crate::messages::Message;

/// Render the search view
pub fn view(app: &App) -> Element<'_, Message> {
    let search_input: iced::widget::TextInput<'_, Message> =
        text_input("Search YouTube...", &app.query)
            .on_input(Message::InputChanged)
            .on_submit(Message::Search)
            .padding(10)
            .width(400);

    let search_button = button(text("Search")).on_press(Message::Search).padding(10);

    let language_label = text("Language:").size(14);

    let language_selector = combo_box(
        &app.language_combo_state,
        "Auto-detect",
        app.selected_language.as_ref(),
        Message::LanguageSelected,
    )
    .width(250);

    let settings_button = button(text("⚙ Settings"))
        .on_press(Message::OpenConfig)
        .padding(10);

    // Responsive layout: under 1000px width, stack controls in two rows
    let controls: Element<Message> = if app.window_width < 1000.0 {
        column![
            row![language_label, language_selector, settings_button]
                .align_y(Center)
                .spacing(10),
            row![search_input, search_button].spacing(10),
        ]
        .align_x(Center)
        .width(Length::Fill)
        .spacing(10)
        .into()
    } else {
        row![
            search_input,
            search_button,
            iced::widget::space::horizontal().width(Length::Fill),
            language_label,
            language_selector,
            settings_button,
        ]
        .align_y(Center)
        .spacing(10)
        .into()
    };

    let search = container(controls).padding(20).width(Length::Fill);

    let body: Element<Message> = if app.search_results.is_empty() {
        if app.searching {
            container(text("Searching...")).padding(40).into()
        } else {
            container(
                column![
                    text("ytrs").size(40),
                    text("YouTube for polyglots").size(14)
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            )
            .padding(60)
            .center_x(Length::FillPortion(1))
            .into()
        }
    } else {
        let cards: Vec<Element<Message>> = app
            .search_results
            .iter()
            .filter(|r| {
                // Filter out premium/members-only videos (keep videos where is_premium is NOT true)
                r.is_premium != Some(true)
            })
            .filter_map(|r| {
                let vid = r.video_id.clone()?;

                // Only render videos if thumbnail is loaded
                let h = app.thumbs.get(&vid)?.clone();

                // Clone all data for lazy closure (must be owned)
                let view_count = r.view_count;
                let duration = r.duration.clone();
                let title = r.title.clone();
                let channel = r.channel.clone();
                let is_playing = app.playing_video.as_ref() == Some(&vid);
                let countdown = app.countdown_value;

                // Lazy widget caches rendering - only rebuilds when (vid, is_playing, countdown) changes
                Some(
                    lazy((vid.clone(), is_playing, countdown), move |_| {
                        let thumb = Image::new(h.clone()).width(240).height(135);
                        let thumb_with_overlay = create_thumbnail(thumb, is_playing, countdown);

                        // Build metadata line
                        let mut meta_parts = vec![];
                        if let Some(v) = view_count {
                            meta_parts.push(format!("{} views", fmt_num(v)));
                        }
                        if let Some(ref d) = duration {
                            meta_parts.push(d.clone());
                        }

                        // Create info section with title and metadata
                        let full_title = title.clone();
                        let display_title = truncate_title(&title, 25);

                        let title_widget = iced::widget::tooltip(
                            text(display_title).size(14),
                            container(text(full_title))
                                .style(container::dark)
                                .padding(10),
                            iced::widget::tooltip::Position::FollowCursor,
                        );

                        let mut info_col = column![title_widget];

                        // Add clickable channel name if available
                        if let Some(ref ch) = channel {
                            if let Some(ref cid) = ch.id {
                                info_col = info_col.push(
                                    // users.rust-lang.org/t/how-to-make-a-static-str-from-a-variable/53718/15
                                    // Leaking memory here is done to make the channel name have a 'static lifetime
                                    // This allows us to 'cache' the video tiles, improving performance drastically.
                                    // However, the downside being that memory is not regained before exiting the application.
                                    // This is probably acceptable for normal use, but there may be a better way of doing this.
                                    button(&*Box::leak(ch.name.clone().into_boxed_str()))
                                        .style(|theme: &iced::Theme, status| match status {
                                            button::Status::Active => button::Style {
                                                text_color: theme.palette().text,
                                                ..Default::default()
                                            },
                                            button::Status::Hovered => button::Style {
                                                text_color: theme.palette().success,
                                                ..Default::default()
                                            },
                                            button::Status::Pressed => button::Style {
                                                text_color: theme.palette().text,
                                                ..Default::default()
                                            },
                                            button::Status::Disabled => button::Style {
                                                text_color: theme.palette().background,
                                                ..Default::default()
                                            },
                                        })
                                        .padding(0)
                                        .on_press(Message::ViewChannel(cid.clone())),
                                );
                            } else {
                                info_col = info_col
                                    .push(text(&*Box::leak(ch.name.clone().into_boxed_str())));
                            }
                        }

                        // Add metadata line if we have any
                        if !meta_parts.is_empty() {
                            info_col = info_col.push(text(meta_parts.join(" • ")).size(12));
                        }

                        let card = column![
                            thumb_with_overlay,
                            container(info_col.spacing(4))
                                .padding(8)
                                .width(240)
                                .height(Length::Fixed(100.0))
                        ]
                        .spacing(0)
                        .width(240);

                        button(card).on_press(Message::Play(vid.clone())).padding(0)
                    })
                    .into(),
                )
            })
            .collect();

        let mut search_content = column![
            container(Wrap::with_elements(cards).spacing(15.0).line_spacing(15.0))
                .center_x(Length::Fill)
        ]
        .align_x(Alignment::Center);

        // Show "Load More" button or loading indicator
        if app.search_loading_more {
            let loading_indicator = container(text("Loading more...").size(14))
                .padding(20)
                .center_x(Length::Fill);
            search_content = search_content.push(loading_indicator);
        } else if app.search_continuation.is_some() {
            // Show "Load More" button if we have more results to load
            let load_more_btn = container(
                button(text("Load More Results"))
                    .on_press(Message::LoadMoreSearchResults)
                    .padding(10),
            )
            .padding(20)
            .center_x(Length::Fill);
            search_content = search_content.push(load_more_btn);
        }

        scrollable(container(search_content).padding(20).width(Length::Fill)).into()
    };

    column![search, body].into()
}
