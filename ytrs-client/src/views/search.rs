//! Search view for the ytrs-client application

use crate::App;
use crate::helpers::{
    ChannelInfo, centered_grid_padding, create_thumbnail, create_video_tile, fmt_num,
};
use crate::messages::Message;
use crate::theme::{rounded_button_style, rounded_text_input_style};
use crate::widgets::{Wrap, bounceable_scrollable, glass_container_style};
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Image, button, column, container, lazy, row, stack, text, text_input},
};

/// Render the search view
pub fn view(app: &App) -> Element<'_, Message> {
    let _start = std::time::Instant::now();

    let search_input: iced::widget::TextInput<'_, Message> =
        text_input("Search YouTube...", &app.query)
            .on_input(Message::InputChanged)
            .on_submit(Message::Search)
            .padding(10)
            .width(300)
            .style(rounded_text_input_style);

    let search_button = button(text("Search"))
        .on_press(Message::Search)
        .padding(10)
        .style(rounded_button_style);

    // Floating search bar with glass style
    let search_controls = row![search_input, search_button].spacing(10);

    let floating_search = container(
        container(search_controls)
            .padding(Padding::new(12.0))
            .style(glass_container_style),
    )
    .padding(Padding {
        top: 8.0,
        bottom: 8.0,
        left: 12.0,
        right: 12.0,
    })
    .width(Length::Shrink)
    .center_x(Length::Fill);

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

        const CARD_WIDTH: f32 = 240.0;
        const CARD_SPACING: f32 = 15.0;

        let grid_padding = centered_grid_padding(
            app.window_width,
            CARD_WIDTH,
            CARD_SPACING,
            20.0,  // min_padding
            20.0,  // top
            180.0, // bottom - extra space for tab bar + floating search bar
        );

        let mut search_content = column![
            Wrap::with_elements(cards)
                .spacing(CARD_SPACING)
                .line_spacing(CARD_SPACING)
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
                    .padding(10)
                    .style(rounded_button_style),
            )
            .padding(20)
            .center_x(Length::Fill);
            search_content = search_content.push(load_more_btn);
        }

        eprintln!(
            "  Search: wrap + column creation took {:?}",
            wrap_start.elapsed()
        );

        eprintln!("  Search view TOTAL: {:?}", _start.elapsed());

        bounceable_scrollable(container(search_content).padding(grid_padding))
            .id("search")
            .into()
    };

    stack![
        container(body).width(Length::Fill).height(Length::Fill),
        container(floating_search)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: 0.0,
                bottom: 110.0, // Position above tab bar with spacing
                left: 0.0,
                right: 0.0,
            })
            .align_y(iced::alignment::Vertical::Bottom)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
