//! Example demonstrating the high-level video player widget with YouTube support.
//!
//! This example shows how to use the VideoPlayerWidget to play a YouTube video
//! with integrated loading, controls, and fullscreen support.
//!
//! Usage: cargo run --example youtube -- <VIDEO_ID>

use iced::{Element, Subscription, Task, Theme};
use iceplayer::{
    PlayerEvent, VideoSource,
    widget::{self, VideoPlayerMessage, VideoPlayerState},
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("YouTube Video Player")
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    VideoPlayer(VideoPlayerMessage),
    VideoEvent(PlayerEvent),
}

struct App {
    video_player: VideoPlayerState,
    window_width: f32,
    window_height: f32,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Get video ID from command line args
        let video_id = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "dQw4w9WgXcQ".to_string()); // Default: Rick Astley

        let source = VideoSource::YouTube {
            video_id: video_id.clone(),
        };
        let mut state =
            VideoPlayerState::new(source.clone()).with_title(format!("Video: {}", video_id));
        let (load_task, load_handle) = widget::start_loading(source);
        state.loading_handle = Some(load_handle);

        (
            Self {
                video_player: state,
                window_width: 1280.0,
                window_height: 720.0,
            },
            // Start loading the video
            load_task.map(Message::VideoPlayer),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::VideoPlayer(msg) => {
                let (event, task) = widget::update(&mut self.video_player, msg);

                // Handle any events emitted by the player
                let event_task = if let Some(ev) = event {
                    Task::done(Message::VideoEvent(ev))
                } else {
                    Task::none()
                };

                Task::batch([task.map(Message::VideoPlayer), event_task])
            }
            Message::VideoEvent(event) => {
                match event {
                    PlayerEvent::Ready { duration } => {
                        println!("Video ready! Duration: {:?}", duration);
                    }
                    PlayerEvent::Ended => {
                        println!("Video ended");
                    }
                    PlayerEvent::Error(err) => {
                        eprintln!("Video error: {}", err);
                    }
                    PlayerEvent::FullscreenChanged(fullscreen) => {
                        println!("Fullscreen: {}", fullscreen);
                        // Handle window fullscreen mode
                        let mode = if fullscreen {
                            iced::window::Mode::Fullscreen
                        } else {
                            iced::window::Mode::Windowed
                        };
                        return iced::window::latest()
                            .and_then(move |id| iced::window::set_mode(id, mode));
                    }
                    PlayerEvent::PlayStateChanged { playing } => {
                        println!("Playing: {}", playing);
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        widget::view(
            &self.video_player,
            Message::VideoPlayer,
            self.window_width,
            self.window_height,
            &Theme::Dark,
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        widget::subscription(&self.video_player).map(Message::VideoPlayer)
    }
}
