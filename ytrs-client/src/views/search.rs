//! Search view for the ytrs-client application

use crate::App;
use crate::helpers::{ChannelInfo, create_thumbnail, create_video_tile, fmt_num};
use crate::messages::Message;
use crate::widgets::{Wrap, bounceable_scrollable};
use iced::{
    Alignment::{self, Center},
    Element, Length,
    widget::{Image, button, column, combo_box, container, lazy, row, text, text_input},
};

/// Render the search view
pub fn view(app: &App) -> Element<'_, Message> {
    let _start = std::time::Instant::now();

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

    // Responsive layout: under 1000px width, stack controls in two rows
    let controls: Element<Message> = if app.window_width < 1000.0 {
        column![
            row![language_label, language_selector,]
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
        let iter_start = std::time::Instant::now();
        eprintln!(
            "  Search: starting iteration over {} results",
            app.search_results.len()
        );
        eprintln!("  Search: thumbs HashMap has {} entries", app.thumbs.len());

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
                        let metadata_text = if !meta_parts.is_empty() {
                            Some(meta_parts.join(" • "))
                        } else {
                            None
                        };

                        // Build channel info
                        // users.rust-lang.org/t/how-to-make-a-static-str-from-a-variable/53718/15
                        // Leaking memory here is done to make the channel name have a 'static lifetime
                        // This allows us to 'cache' the video tiles, improving performance drastically.
                        // However, the downside being that memory is not regained before exiting the application.
                        // This is probably acceptable for normal use, but there may be a better way of doing this.
                        let channel_info = channel.as_ref().map(|ch| ChannelInfo {
                            name: &*Box::leak(ch.name.clone().into_boxed_str()),
                            on_press: ch.id.clone().map(Message::ViewChannel),
                        });

                        create_video_tile(
                            thumb_with_overlay,
                            &title,
                            channel_info,
                            metadata_text,
                            Message::Play(vid.clone()),
                        )
                    })
                    .into(),
                )
            })
            .collect();

        eprintln!(
            "  Search: iteration + lazy creation took {:?}",
            iter_start.elapsed()
        );
        eprintln!("    - Total cards: {}", cards.len());
        eprintln!("    - Total results: {}", app.search_results.len());

        let wrap_start = std::time::Instant::now();
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

        eprintln!(
            "  Search: wrap + column creation took {:?}",
            wrap_start.elapsed()
        );

        let result = bounceable_scrollable(
            container(search_content)
                .padding(iced::Padding {
                    top: 20.0,
                    bottom: 100.0, // Extra space for tab bar overlay
                    left: 20.0,
                    right: 20.0,
                })
                .width(Length::Fill),
        )
        .id("search")
        .into();

        eprintln!("  Search view TOTAL: {:?}", _start.elapsed());

        result
    };

    column![search, body].into()
}
