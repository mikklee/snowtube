//! High-level video player widget with integrated controls, loading, and overlays.
//!
//! This module provides a self-contained video player that manages all internal state
//! and only emits high-level events to the parent application.

pub mod controls;
pub mod overlay;
pub mod spinner;

use crate::event::PlayerEvent;
use crate::loader::{LoadProgress, load_video};
use crate::source::VideoSource;
use crate::video::Video;
use crate::video_player::VideoPlayer;

use controls::{loading_control_bar, video_control_bar};
use overlay::{
    centered_play_button, error_overlay, loading_placeholder, seeking_overlay, title_overlay,
};

use iced::widget::{container, mouse_area, stack};
use iced::{Color, Element, Length, Renderer, Subscription, Task, Theme};
use std::time::{Duration, Instant};

/// Internal state for the video player widget.
#[derive(Debug)]
pub struct VideoPlayerState {
    /// The video source being played.
    pub source: VideoSource,
    /// The loaded video, if any.
    pub video: Option<Video>,
    /// Whether the video is currently loading.
    pub loading: bool,
    /// Current loading status message.
    pub loading_status: Option<String>,
    /// Error message if loading failed.
    pub error: Option<String>,
    /// Whether the video has been started (user clicked play).
    pub started: bool,
    /// Whether the video is currently seeking.
    pub seeking: bool,
    /// Target seek position.
    pub seek_target: Option<Duration>,
    /// Preview position while dragging slider (0.0 to 1.0).
    pub seek_preview: Option<f64>,
    /// Whether controls are visible.
    pub controls_visible: bool,
    /// Last time mouse moved over the video.
    pub last_mouse_move: Option<Instant>,
    /// Whether the video is in fullscreen mode.
    pub fullscreen: bool,
    /// Optional title to display.
    pub title: Option<String>,
    /// Unique ID for this player instance.
    id: u64,
}

impl VideoPlayerState {
    /// Create a new video player state for the given source.
    pub fn new(source: VideoSource) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        Self {
            source,
            video: None,
            loading: true,
            loading_status: Some("Initializing...".to_string()),
            error: None,
            started: false,
            seeking: false,
            seek_target: None,
            seek_preview: None,
            controls_visible: true,
            last_mouse_move: None,
            fullscreen: false,
            title: None,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Set the title to display.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Get the unique ID for this player instance.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Check if the video is paused.
    pub fn is_paused(&self) -> bool {
        self.video.as_ref().map(|v| v.paused()).unwrap_or(true)
    }

    /// Get the current position.
    pub fn position(&self) -> Duration {
        self.video
            .as_ref()
            .map(|v| v.position())
            .unwrap_or(Duration::ZERO)
    }

    /// Get the duration.
    pub fn duration(&self) -> Duration {
        self.video
            .as_ref()
            .map(|v| v.duration())
            .unwrap_or(Duration::ZERO)
    }
}

/// Messages for the video player widget.
#[derive(Debug, Clone)]
pub enum VideoPlayerMessage {
    /// Loading progress update.
    LoadProgress(LoadProgress),
    /// Toggle play/pause.
    TogglePlayPause,
    /// Start playback (from centered play button).
    StartPlayback,
    /// Seek preview while dragging slider.
    SeekPreview(f64),
    /// Release slider to perform seek.
    SeekRelease,
    /// Seek operation completed.
    SeekComplete,
    /// Mouse moved over video.
    MouseMoved,
    /// Controls timeout - hide controls.
    ControlsTimeout,
    /// Toggle fullscreen.
    ToggleFullscreen,
    /// Video ended.
    VideoEnded,
    /// Tick for updating position.
    Tick,
}

/// Create a Task to start loading a video.
/// Call this when creating a new VideoPlayerState to initiate loading.
pub fn start_loading(source: VideoSource) -> Task<VideoPlayerMessage> {
    Task::run(load_video(source), VideoPlayerMessage::LoadProgress)
}

/// Update the video player state based on a message.
/// Returns an optional PlayerEvent to emit and an optional Task.
pub fn update(
    state: &mut VideoPlayerState,
    message: VideoPlayerMessage,
) -> (Option<PlayerEvent>, Task<VideoPlayerMessage>) {
    match message {
        VideoPlayerMessage::LoadProgress(progress) => match progress {
            LoadProgress::Status(status) => {
                state.loading_status = Some(status);
                (None, Task::none())
            }
            LoadProgress::Done(video) => {
                state.loading = false;
                state.loading_status = None;
                // Unwrap the Arc - we're the only owner at this point
                match std::sync::Arc::try_unwrap(video) {
                    Ok(mut v) => {
                        // Start paused - user must click play button
                        v.set_paused(true);
                        state.video = Some(v);
                        let duration = state.duration();
                        (Some(PlayerEvent::Ready { duration }), Task::none())
                    }
                    Err(_) => {
                        state.error = Some("Failed to unwrap video Arc".to_string());
                        (
                            Some(PlayerEvent::Error("Failed to unwrap video Arc".to_string())),
                            Task::none(),
                        )
                    }
                }
            }
            LoadProgress::Error(error) => {
                state.loading = false;
                state.loading_status = None;
                state.error = Some(error.clone());
                (Some(PlayerEvent::Error(error)), Task::none())
            }
        },
        VideoPlayerMessage::TogglePlayPause => {
            if let Some(ref mut video) = state.video {
                let new_paused = !video.paused();
                video.set_paused(new_paused);
                (
                    Some(PlayerEvent::PlayStateChanged {
                        playing: !new_paused,
                    }),
                    Task::none(),
                )
            } else {
                (None, Task::none())
            }
        }
        VideoPlayerMessage::StartPlayback => {
            if let Some(ref mut video) = state.video {
                state.started = true;
                video.set_paused(false);
                (
                    Some(PlayerEvent::PlayStateChanged { playing: true }),
                    Task::none(),
                )
            } else {
                (None, Task::none())
            }
        }
        VideoPlayerMessage::SeekPreview(position) => {
            state.seek_preview = Some(position);
            // Pause while seeking for smoother experience
            if let Some(ref mut video) = state.video {
                video.set_paused(true);
            }
            (None, Task::none())
        }
        VideoPlayerMessage::SeekRelease => {
            if let Some(preview) = state.seek_preview.take() {
                if let Some(ref mut video) = state.video {
                    let duration = video.duration();
                    let target = Duration::from_secs_f64(duration.as_secs_f64() * preview);
                    state.seeking = true;
                    state.seek_target = Some(target);
                    // Perform the seek
                    if let Err(e) = video.seek(target, false) {
                        tracing::error!("Seek failed: {}", e);
                        state.seeking = false;
                    }
                    // Resume playback after seek (SeekDone will clear seeking state)
                    video.set_paused(false);
                }
            }
            (None, Task::none())
        }
        VideoPlayerMessage::SeekComplete => {
            state.seeking = false;
            state.seek_target = None;
            (None, Task::none())
        }
        VideoPlayerMessage::MouseMoved => {
            state.controls_visible = true;
            state.last_mouse_move = Some(Instant::now());
            (None, Task::none())
        }
        VideoPlayerMessage::ControlsTimeout => {
            // Hide controls if no mouse movement for 3 seconds
            if let Some(last_move) = state.last_mouse_move {
                if last_move.elapsed() > Duration::from_secs(3) && !state.is_paused() {
                    state.controls_visible = false;
                }
            }
            (None, Task::none())
        }
        VideoPlayerMessage::ToggleFullscreen => {
            state.fullscreen = !state.fullscreen;
            (
                Some(PlayerEvent::FullscreenChanged(state.fullscreen)),
                Task::none(),
            )
        }
        VideoPlayerMessage::VideoEnded => {
            state.started = false; // Reset to show play button again
            (Some(PlayerEvent::Ended), Task::none())
        }
        VideoPlayerMessage::Tick => {
            // Just request a redraw to update the position display
            (None, Task::none())
        }
    }
}

/// Create a subscription for the video player.
/// This handles periodic updates like controls timeout and position ticks.
pub fn subscription(state: &VideoPlayerState) -> Subscription<VideoPlayerMessage> {
    let mut subscriptions = vec![];

    // Controls timeout subscription
    if state.video.is_some() && state.controls_visible && !state.is_paused() {
        subscriptions.push(
            iced::time::every(Duration::from_millis(500))
                .map(|_| VideoPlayerMessage::ControlsTimeout),
        );
    }

    // Position update subscription
    if state.video.is_some() && !state.is_paused() {
        subscriptions
            .push(iced::time::every(Duration::from_millis(250)).map(|_| VideoPlayerMessage::Tick));
    }

    Subscription::batch(subscriptions)
}

/// Render the video player widget.
pub fn view<'a, Message: Clone + 'static>(
    state: &'a VideoPlayerState,
    on_message: impl Fn(VideoPlayerMessage) -> Message + 'a + Clone,
    available_width: f32,
    available_height: f32,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    if state.loading {
        // Loading state
        view_loading(state, on_message, available_width, available_height, theme)
    } else if let Some(ref error) = state.error {
        // Error state
        view_error(error, available_width, available_height, theme)
    } else if let Some(ref video) = state.video {
        // Playing state
        view_playing(
            state,
            video,
            on_message,
            available_width,
            available_height,
            theme,
        )
    } else {
        // Should not happen, but show error
        view_error("No video loaded", available_width, available_height, theme)
    }
}

fn view_loading<'a, Message: Clone + 'static>(
    state: &'a VideoPlayerState,
    on_message: impl Fn(VideoPlayerMessage) -> Message + 'a + Clone,
    available_width: f32,
    available_height: f32,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    // Standard 16:9 aspect ratio
    const ASPECT_RATIO: f32 = 16.0 / 9.0;

    // Calculate dimensions to fit within available space while maintaining 16:9
    let available_aspect = available_width / available_height;
    let (width, height) = if ASPECT_RATIO > available_aspect {
        (available_width, available_width / ASPECT_RATIO)
    } else {
        (available_height * ASPECT_RATIO, available_height)
    };

    let loading_content = loading_placeholder(state.loading_status.as_deref(), theme);

    let mut layers: Vec<Element<'a, Message, Theme, Renderer>> = vec![loading_content];

    // Title overlay
    if let Some(ref title) = state.title {
        layers.push(title_overlay(title));
    }

    // Disabled control bar
    let on_msg = on_message.clone();
    layers.push(
        container(loading_control_bar(
            on_msg(VideoPlayerMessage::TogglePlayPause),
            theme,
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(iced::alignment::Vertical::Bottom)
        .into(),
    );

    container(stack(layers))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        })
        .into()
}

fn view_error<'a, Message: 'a>(
    error: &'a str,
    available_width: f32,
    available_height: f32,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    // Standard 16:9 aspect ratio
    const ASPECT_RATIO: f32 = 16.0 / 9.0;

    let available_aspect = available_width / available_height;
    let (width, height) = if ASPECT_RATIO > available_aspect {
        (available_width, available_width / ASPECT_RATIO)
    } else {
        (available_height * ASPECT_RATIO, available_height)
    };

    container(error_overlay(error, theme))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        })
        .into()
}

fn view_playing<'a, Message: Clone + 'static>(
    state: &'a VideoPlayerState,
    video: &'a Video,
    on_message: impl Fn(VideoPlayerMessage) -> Message + 'a + Clone,
    available_width: f32,
    available_height: f32,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    let (video_width, video_height) = video.size();

    // Calculate scaled dimensions to fit within available space
    let video_aspect = video_width as f32 / video_height as f32;
    let available_aspect = available_width / available_height;

    let (scaled_width, scaled_height) = if state.fullscreen {
        (available_width, available_height)
    } else if video_aspect > available_aspect {
        (available_width, available_width / video_aspect)
    } else {
        (available_height * video_aspect, available_height)
    };

    let on_msg = on_message.clone();
    let on_msg2 = on_message.clone();
    let video_widget: Element<'a, Message, Theme, Renderer> = VideoPlayer::new(video)
        .width(scaled_width)
        .height(scaled_height)
        .content_fit(iced::ContentFit::Contain)
        .on_end_of_stream(on_msg(VideoPlayerMessage::VideoEnded))
        .on_double_click(on_message.clone()(VideoPlayerMessage::ToggleFullscreen))
        .on_seek_complete(on_msg2(VideoPlayerMessage::SeekComplete))
        .into();

    // Wrap in mouse area for tracking mouse movement
    let on_msg = on_message.clone();
    let video_with_mouse =
        mouse_area(video_widget).on_move(move |_| on_msg(VideoPlayerMessage::MouseMoved));

    let mut layers: Vec<Element<'a, Message, Theme, Renderer>> = vec![video_with_mouse.into()];

    // Title overlay (only show when controls visible)
    if state.controls_visible {
        if let Some(ref title) = state.title {
            layers.push(title_overlay(title));
        }
    }

    // If video hasn't started, show centered play button
    if !state.started {
        let on_msg = on_message.clone();
        layers.push(centered_play_button(
            on_msg(VideoPlayerMessage::StartPlayback),
            theme,
        ));
    } else if state.controls_visible {
        // Control bar
        let on_msg1 = on_message.clone();
        let on_msg2 = on_message.clone();
        let on_msg3 = on_message.clone();
        layers.push(
            container(video_control_bar(
                state.is_paused(),
                state.position(),
                state.duration(),
                state.seek_preview,
                on_msg1(VideoPlayerMessage::TogglePlayPause),
                move |pos| on_msg2(VideoPlayerMessage::SeekPreview(pos)),
                on_msg3(VideoPlayerMessage::SeekRelease),
                theme,
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Bottom)
            .into(),
        );
    }

    // Seeking overlay
    if state.seeking {
        layers.push(seeking_overlay(theme));
    }

    stack(layers)
        .width(Length::Fixed(scaled_width))
        .height(Length::Fixed(scaled_height))
        .into()
}

/// Re-export for convenience
pub use controls::glass_container_style;
pub use spinner::spinner;
