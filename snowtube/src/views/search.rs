//! Search view for the ytrs-client application

use crate::App;
use crate::helpers::{
    ChannelInfo, centered_grid_padding, create_thumbnail, create_video_tile, fmt_num,
};
use crate::messages::Message;
use crate::theme::{rounded_button_style, rounded_text_input_style};
use crate::widgets::icon_button::gen_icon_button;
use crate::widgets::icons::icon_search;
use crate::widgets::{Wrap, bounceable_scrollable, glass_container_style};
use common::{format_relative_time, parse_relative_time};
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Image, button, column, container, lazy, row, stack, text, text_input},
};

/// Render the search view
pub fn view(app: &App) -> Element<'_, Message> {
    let search_input: iced::widget::TextInput<'_, Message> =
        text_input("Search videos...", &app.query)
            .on_input(Message::InputChanged)
            .on_submit(Message::Search)
            .padding(10)
            .width(300)
            .style(rounded_text_input_style);

    let search_button = gen_icon_button(40.0, icon_search, "Search videos", Message::Search);

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
            container(text("SnowTube").size(40))
                .padding(60)
                .center_x(Length::FillPortion(1))
                .into()
        }
    } else {
        let cards: Vec<Element<Message>> = app
            .search_results
            .iter()
            .filter(|r| {
                // Filter out premium/members-only videos and Shorts
                r.is_premium != Some(true) && r.is_short != Some(true)
            })
            .filter_map(|r| {
                if r.id.is_empty() {
                    return None;
                }

                // Only render videos if thumbnail is loaded
                let h = app.video_thumbs.get(&r.watch_url)?.clone();

                // Clone all data for lazy closure (must be owned)
                let vid = r.id.clone();
                let platform_name = r.platform_name.clone();
                let view_count = r.view_count;
                let duration_string = r.duration_string.clone();
                let published_text = r.published_text.clone();
                let title = r.title.clone();
                let channel = r.channel.clone();
                let video = Box::new(r.clone());
                let instance = r.instance.clone();

                // Lazy widget caches rendering - only rebuilds when vid changes
                Some(
                    lazy(vid.clone(), move |_| {
                        let platform_icon = crate::providers::get_platform_icon(&platform_name);
                        let thumb = Image::new(h.clone()).width(240).height(135);
                        let thumb_with_overlay = create_thumbnail(thumb, false, 0);

                        // Build metadata line
                        let mut meta_parts = vec![];
                        if let Some(v) = view_count {
                            meta_parts.push(format!("{} views", fmt_num(v)));
                        }
                        if let Some(ref d) = duration_string {
                            meta_parts.push(d.clone());
                        }
                        let seconds = parse_relative_time(published_text.as_deref());
                        let time_ago = format_relative_time(seconds);
                        meta_parts.push(time_ago);
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
                            on_press: ch.id.clone().map(|cid| {
                                Message::ViewChannel(common::ChannelConfig {
                                    platform_name: platform_name.clone(),
                                    channel_id: cid,
                                    channel_name: ch.name.clone(),
                                    channel_handle: None,
                                    thumbnail_url: ch
                                        .thumbnails
                                        .first()
                                        .map(|t| t.url.clone())
                                        .unwrap_or_default(),
                                    instance: instance.clone(),
                                    subscribed: false,
                                    subscribed_at: None,
                                    language: None,
                                })
                            }),
                        });

                        create_video_tile(
                            thumb_with_overlay,
                            &title,
                            channel_info,
                            metadata_text,
                            Message::PlayVideo(video.clone()),
                            platform_icon,
                        )
                    })
                    .into(),
                )
            })
            .collect();

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

        // Show "Load More" button or loading indicator (hide while preloading)
        if app.search_loading_more || app.search_preloading {
            let loading_indicator = container(text("Loading more...").size(14))
                .padding(20)
                .center_x(Length::Fill);
            search_content = search_content.push(loading_indicator);
        } else if !app.search_continuations.is_empty() {
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

        bounceable_scrollable(container(search_content).padding(grid_padding))
            .id("search")
            .visible_scrollbar(app.config.show_scrollbar)
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
