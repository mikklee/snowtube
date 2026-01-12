//! High-level video player widget with integrated controls, loading, and overlays.
//!
//! This module provides a self-contained video player that manages all internal state
//! and only emits high-level events to the parent application.

pub mod controls;
pub mod overlay;
pub mod snowflake;
pub mod spinner;

use crate::event::PlayerEvent;
use crate::led_visualizer::LedVisualizer;
use crate::loader::{LoadProgress, load_video};
use crate::source::VideoSource;
use crate::video::Video;
use crate::video_player::VideoPlayer;
use crate::visualizer::{AudioVisualizer, Visualizer};
use std::sync::Arc;

use controls::{
    ControlBarParams, fullscreen_control_bar, loading_control_bar, ready_control_bar,
    video_control_bar,
};
use overlay::{
    centered_play_button, error_overlay, loading_overlay, loading_placeholder, seeking_overlay,
    title_overlay,
};

use iced::widget::{column, container, mouse_area, stack};
use iced::{Color, Element, Length, Renderer, Subscription, Task, Theme, mouse};
use std::time::{Duration, Instant};

/// Internal state for the video player widget.
pub struct VideoPlayerState {
    /// The video source being played.
    pub source: VideoSource,
    /// The loaded video, if any.
    pub video: Option<Arc<Video>>,
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
    /// Optional thumbnail to show while loading.
    pub thumbnail: Option<iced::widget::image::Handle>,
    /// Optional pre-set duration (from video info before loading).
    pub preset_duration: Option<Duration>,
    /// Handle to abort the loading task.
    pub loading_handle: Option<iced::task::Handle>,
    /// Audio visualizer style for audio-only mode.
    pub visualizer: AudioVisualizer,
}

impl VideoPlayerState {
    /// Create a new video player state for the given source.
    pub fn new(source: VideoSource) -> Self {
        Self {
            source,
            video: None,
            loading: false,
            loading_status: None,
            error: None,
            started: false,
            seeking: false,
            seek_target: None,
            seek_preview: None,
            controls_visible: true,
            last_mouse_move: None,
            fullscreen: false,
            title: None,
            thumbnail: None,
            preset_duration: None,
            loading_handle: None,
            visualizer: AudioVisualizer::default(),
        }
    }

    /// Set the audio visualizer style.
    pub fn with_visualizer(mut self, visualizer: AudioVisualizer) -> Self {
        self.visualizer = visualizer;
        self
    }

    /// Set the title to display.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the thumbnail to show while loading.
    pub fn with_thumbnail(mut self, thumbnail: iced::widget::image::Handle) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }

    /// Set the duration (from video info before loading).
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.preset_duration = Some(duration);
        self
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
    /// Prefers preset_duration (from video metadata) over GStreamer-queried duration,
    /// as some streams (e.g., HLS from PeerTube) report incorrect durations.
    pub fn duration(&self) -> Duration {
        // Prefer preset duration if set (from video metadata)
        if let Some(preset) = self.preset_duration {
            if preset > Duration::ZERO {
                return preset;
            }
        }
        // Fall back to GStreamer-queried duration
        self.video
            .as_ref()
            .map(|v| v.duration())
            .filter(|d| *d > Duration::ZERO)
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
/// Returns the task and an abort handle that can be used to cancel loading.
pub fn start_loading(source: VideoSource) -> (Task<VideoPlayerMessage>, iced::task::Handle) {
    Task::run(load_video(source), VideoPlayerMessage::LoadProgress).abortable()
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
                // Auto-start playback since user already clicked play
                state.started = true;
                video.set_paused(false);
                state.video = Some(video);
                let duration = state.duration();
                (Some(PlayerEvent::Ready { duration }), Task::none())
            }
            LoadProgress::Error(error) => {
                state.loading = false;
                state.loading_status = None;
                state.error = Some(error.clone());
                (Some(PlayerEvent::Error(error)), Task::none())
            }
        },
        VideoPlayerMessage::TogglePlayPause => {
            if let Some(ref video) = state.video {
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
            if let Some(ref video) = state.video {
                // Video already loaded, just start playing
                state.started = true;
                video.set_paused(false);
                (
                    Some(PlayerEvent::PlayStateChanged { playing: true }),
                    Task::none(),
                )
            } else if !state.loading {
                // No video yet, start loading
                state.loading = true;
                state.loading_status = Some("Initializing...".to_string());
                let (load_task, handle) = start_loading(state.source.clone());
                state.loading_handle = Some(handle);
                (None, load_task)
            } else {
                // Already loading, do nothing
                (None, Task::none())
            }
        }
        VideoPlayerMessage::SeekPreview(position) => {
            state.seek_preview = Some(position);
            // Show controls and reset timeout when seeking
            state.controls_visible = true;
            state.last_mouse_move = Some(Instant::now());
            // Pause while seeking for smoother experience
            if let Some(ref video) = state.video {
                video.set_paused(true);
            }
            (None, Task::none())
        }
        VideoPlayerMessage::SeekRelease => {
            if let Some(preview) = state.seek_preview.take()
                && let Some(ref video) = state.video
            {
                let duration = video.duration();
                let target = Duration::from_secs_f64(duration.as_secs_f64() * preview);
                state.seeking = true;
                state.seek_target = Some(target);
                // Perform the seek
                if let Err(e) = video.seek(target, false) {
                    tracing::error!("Seek failed: {}", e);
                    state.seeking = false;
                }
                // Resume playback after seek (SeekComplete will clear seeking state)
                video.set_paused(false);
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
            if let Some(last_move) = state.last_mouse_move
                && last_move.elapsed() > Duration::from_secs(3)
                && !state.is_paused()
            {
                state.controls_visible = false;
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
            .push(iced::time::every(Duration::from_millis(6)).map(|_| VideoPlayerMessage::Tick));
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
    if let Some(ref error) = state.error {
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
    } else if state.loading {
        // Loading state
        view_loading(state, on_message, available_width, available_height, theme)
    } else {
        // Ready to play state - show thumbnail with play button
        view_ready(state, on_message, available_width, available_height, theme)
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
    const CONTROL_BAR_HEIGHT: f32 = 44.0;

    // Reserve space for controls
    let video_available_height = available_height - CONTROL_BAR_HEIGHT;

    // Calculate dimensions to fit within available space while maintaining 16:9
    let available_aspect = available_width / video_available_height;
    let (width, height) = if ASPECT_RATIO > available_aspect {
        (available_width, available_width / ASPECT_RATIO)
    } else {
        (
            video_available_height * ASPECT_RATIO,
            video_available_height,
        )
    };

    // Use thumbnail as background if available, otherwise use loading placeholder
    let background: Element<'a, Message, Theme, Renderer> =
        if let Some(ref thumbnail) = state.thumbnail {
            container(
                iced::widget::image(thumbnail.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Cover),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            loading_placeholder(state.loading_status.as_deref(), theme)
        };

    let mut layers: Vec<Element<'a, Message, Theme, Renderer>> = vec![background];

    // Loading spinner and status overlay (on top of thumbnail)
    if state.thumbnail.is_some() {
        layers.push(loading_overlay(state.loading_status.as_deref(), theme));
    }

    // Title overlay
    if let Some(ref title) = state.title {
        layers.push(title_overlay(title));
    }

    let video_frame = container(stack(layers))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        });

    // Disabled control bar below video
    let control_bar = loading_control_bar(
        state.duration(),
        on_message.clone()(VideoPlayerMessage::TogglePlayPause),
        on_message.clone()(VideoPlayerMessage::ToggleFullscreen),
        theme,
    );

    // Stack video and controls vertically
    column![
        video_frame,
        container(control_bar).width(Length::Fixed(width)),
    ]
    .spacing(0)
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

fn view_ready<'a, Message: Clone + 'static>(
    state: &'a VideoPlayerState,
    on_message: impl Fn(VideoPlayerMessage) -> Message + 'a + Clone,
    available_width: f32,
    available_height: f32,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    // Standard 16:9 aspect ratio
    const ASPECT_RATIO: f32 = 16.0 / 9.0;
    const CONTROL_BAR_HEIGHT: f32 = 44.0;

    // Reserve space for controls
    let video_available_height = available_height - CONTROL_BAR_HEIGHT;

    let available_aspect = available_width / video_available_height;
    let (width, height) = if ASPECT_RATIO > available_aspect {
        (available_width, available_width / ASPECT_RATIO)
    } else {
        (
            video_available_height * ASPECT_RATIO,
            video_available_height,
        )
    };

    // Background: thumbnail or black
    let background: Element<'a, Message, Theme, Renderer> =
        if let Some(ref thumbnail) = state.thumbnail {
            container(
                iced::widget::image(thumbnail.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Cover),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

    let mut layers: Vec<Element<'a, Message, Theme, Renderer>> = vec![background];

    // Title overlay
    if let Some(ref title) = state.title {
        layers.push(title_overlay(title));
    }

    // Centered play button
    layers.push(centered_play_button(
        on_message.clone()(VideoPlayerMessage::StartPlayback),
        theme,
    ));

    let video_frame = container(stack(layers))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::BLACK)),
            ..Default::default()
        });

    // Control bar below video (play enabled, seek disabled)
    let control_bar = ready_control_bar(
        state.duration(),
        on_message.clone()(VideoPlayerMessage::StartPlayback),
        on_message(VideoPlayerMessage::ToggleFullscreen),
        theme,
    );

    // Stack video and controls vertically
    column![
        video_frame,
        container(control_bar).width(Length::Fixed(width)),
    ]
    .spacing(0)
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
    let video_aspect = video_width as f32 / video_height as f32;

    if state.fullscreen {
        // Fullscreen mode: video fills screen, controls overlay at bottom
        view_playing_fullscreen(
            state,
            video,
            on_message,
            available_width,
            available_height,
            theme,
        )
    } else {
        // Windowed mode: video with controls below
        view_playing_windowed(
            state,
            video,
            on_message,
            available_width,
            available_height,
            video_aspect,
            theme,
        )
    }
}

fn view_playing_windowed<'a, Message: Clone + 'static>(
    state: &'a VideoPlayerState,
    video: &'a Video,
    on_message: impl Fn(VideoPlayerMessage) -> Message + 'a + Clone,
    available_width: f32,
    available_height: f32,
    video_aspect: f32,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    // Reserve space for controls (approximately 44px)
    const CONTROL_BAR_HEIGHT: f32 = 44.0;
    let video_available_height = available_height - CONTROL_BAR_HEIGHT;

    // Calculate video dimensions to fit in remaining space
    let available_aspect = available_width / video_available_height;
    let (scaled_width, scaled_height) = if video_aspect > available_aspect {
        (available_width, available_width / video_aspect)
    } else {
        (
            video_available_height * video_aspect,
            video_available_height,
        )
    };

    // Always use VideoPlayer widget for event handling (EOS, seek complete, etc.)
    let video_widget: Element<'a, Message, Theme, Renderer> = VideoPlayer::new(video)
        .width(scaled_width)
        .height(scaled_height)
        .content_fit(iced::ContentFit::Contain)
        .on_end_of_stream(on_message.clone()(VideoPlayerMessage::VideoEnded))
        .on_single_click(on_message.clone()(VideoPlayerMessage::TogglePlayPause))
        .on_double_click(on_message.clone()(VideoPlayerMessage::ToggleFullscreen))
        .on_seek_complete(on_message.clone()(VideoPlayerMessage::SeekComplete))
        .into();

    // Wrap in mouse area for tracking mouse movement
    let on_mouse_move = on_message.clone();
    let video_with_mouse =
        mouse_area(video_widget).on_move(move |_| on_mouse_move(VideoPlayerMessage::MouseMoved));

    // Video layers (for overlays like title, play button, seeking)
    let mut video_layers: Vec<Element<'a, Message, Theme, Renderer>> =
        vec![video_with_mouse.into()];

    // For audio-only, overlay thumbnail and visualizer on top of the (invisible) video widget
    if state.source.is_audio_only() {
        if let Some(ref thumbnail) = state.thumbnail {
            video_layers.push(
                container(
                    iced::widget::image(thumbnail.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(iced::ContentFit::Cover),
                )
                .width(Length::Fixed(scaled_width))
                .height(Length::Fixed(scaled_height))
                .into(),
            );
        }

        // Add audio visualizer overlay when playing
        if state.started
            && let Some(ref video) = state.video
        {
            let spectrum = video.spectrum();
            let time = state.position().as_secs_f32();
            let primary_color = theme.palette().primary;
            let visualizer_element: Option<Element<'a, Message, Theme, Renderer>> =
                match state.visualizer {
                    AudioVisualizer::Disabled => None,
                    AudioVisualizer::PlasmaGlobe => Some(
                        Visualizer::new(spectrum)
                            .time(time)
                            .color(primary_color)
                            .width(Length::Fixed(scaled_width))
                            .height(Length::Fixed(scaled_height))
                            .into(),
                    ),
                    AudioVisualizer::LedSpectrum => Some(
                        LedVisualizer::new(spectrum)
                            .time(time)
                            .color(primary_color)
                            .width(Length::Fixed(scaled_width))
                            .height(Length::Fixed(scaled_height))
                            .into(),
                    ),
                };
            if let Some(element) = visualizer_element {
                video_layers.push(element);
            }
        }
    }

    // Title overlay (only show when controls visible)
    if state.controls_visible
        && let Some(ref title) = state.title
    {
        video_layers.push(title_overlay(title));
    }

    // If video hasn't started, show centered play button
    if !state.started {
        video_layers.push(centered_play_button(
            on_message.clone()(VideoPlayerMessage::StartPlayback),
            theme,
        ));
    } else if state.position().as_millis() == 0 && !state.seeking {
        // Video started but waiting for first frame (not during seek)
        video_layers.push(loading_overlay(Some("Starting..."), theme));
    }

    // Seeking overlay
    if state.seeking {
        video_layers.push(seeking_overlay(theme));
    }

    let video_stack = stack(video_layers)
        .width(Length::Fixed(scaled_width))
        .height(Length::Fixed(scaled_height));

    // Control bar below video
    let on_seek_preview = on_message.clone();
    let control_bar = video_control_bar(ControlBarParams {
        is_paused: state.is_paused(),
        position: state.position(),
        duration: state.duration(),
        seek_preview: state.seek_preview,
        on_toggle_play: on_message.clone()(VideoPlayerMessage::TogglePlayPause),
        on_seek_preview: Box::new(move |pos| on_seek_preview(VideoPlayerMessage::SeekPreview(pos))),
        on_seek_release: on_message.clone()(VideoPlayerMessage::SeekRelease),
        on_toggle_fullscreen: on_message(VideoPlayerMessage::ToggleFullscreen),
    });

    // Stack video and controls vertically
    column![
        video_stack,
        container(control_bar).width(Length::Fixed(scaled_width)),
    ]
    .spacing(0)
    .into()
}

fn view_playing_fullscreen<'a, Message: Clone + 'static>(
    state: &'a VideoPlayerState,
    video: &'a Video,
    on_message: impl Fn(VideoPlayerMessage) -> Message + 'a + Clone,
    available_width: f32,
    available_height: f32,
    theme: &'a Theme,
) -> Element<'a, Message, Theme, Renderer> {
    // Always use VideoPlayer widget for event handling (EOS, seek complete, etc.)
    let video_widget: Element<'a, Message, Theme, Renderer> = VideoPlayer::new(video)
        .width(available_width)
        .height(available_height)
        .content_fit(iced::ContentFit::Contain)
        .on_end_of_stream(on_message.clone()(VideoPlayerMessage::VideoEnded))
        .on_single_click(on_message.clone()(VideoPlayerMessage::TogglePlayPause))
        .on_double_click(on_message.clone()(VideoPlayerMessage::ToggleFullscreen))
        .on_seek_complete(on_message.clone()(VideoPlayerMessage::SeekComplete))
        .into();

    // Wrap in mouse area for tracking mouse movement
    // Hide cursor when controls are hidden (after inactivity)
    let on_mouse_move = on_message.clone();
    let video_with_mouse = if state.controls_visible {
        mouse_area(video_widget).on_move(move |_| on_mouse_move(VideoPlayerMessage::MouseMoved))
    } else {
        mouse_area(video_widget)
            .on_move(move |_| on_mouse_move(VideoPlayerMessage::MouseMoved))
            .interaction(mouse::Interaction::Hidden)
    };

    let mut layers: Vec<Element<'a, Message, Theme, Renderer>> = vec![video_with_mouse.into()];

    // For audio-only, overlay thumbnail and visualizer on top of the (invisible) video widget
    if state.source.is_audio_only() {
        if let Some(ref thumbnail) = state.thumbnail {
            layers.push(
                container(
                    iced::widget::image(thumbnail.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(iced::ContentFit::Contain),
                )
                .width(Length::Fixed(available_width))
                .height(Length::Fixed(available_height))
                .center(Length::Fill)
                .into(),
            );
        }

        // Add audio visualizer overlay when playing
        if state.started
            && let Some(ref video) = state.video
        {
            let spectrum = video.spectrum();
            let time = state.position().as_secs_f32();
            let primary_color = theme.palette().primary;
            let visualizer_element: Option<Element<'a, Message, Theme, Renderer>> =
                match state.visualizer {
                    AudioVisualizer::Disabled => None,
                    AudioVisualizer::PlasmaGlobe => Some(
                        Visualizer::new(spectrum)
                            .time(time)
                            .color(primary_color)
                            .width(Length::Fixed(available_width))
                            .height(Length::Fixed(available_height))
                            .into(),
                    ),
                    AudioVisualizer::LedSpectrum => Some(
                        LedVisualizer::new(spectrum)
                            .time(time)
                            .color(primary_color)
                            .width(Length::Fixed(available_width))
                            .height(Length::Fixed(available_height))
                            .into(),
                    ),
                };
            if let Some(element) = visualizer_element {
                layers.push(element);
            }
        }
    }

    // Title overlay (only show when controls visible)
    if state.controls_visible
        && let Some(ref title) = state.title
    {
        layers.push(title_overlay(title));
    }

    // If video hasn't started, show centered play button
    if !state.started {
        layers.push(centered_play_button(
            on_message.clone()(VideoPlayerMessage::StartPlayback),
            theme,
        ));
    } else if state.position().as_millis() == 0 && !state.seeking {
        // Video started but waiting for first frame (not during seek)
        layers.push(loading_overlay(Some("Starting..."), theme));
    } else if state.controls_visible {
        // Fullscreen control bar at bottom
        let on_seek_preview = on_message.clone();
        layers.push(
            container(fullscreen_control_bar(ControlBarParams {
                is_paused: state.is_paused(),
                position: state.position(),
                duration: state.duration(),
                seek_preview: state.seek_preview,
                on_toggle_play: on_message.clone()(VideoPlayerMessage::TogglePlayPause),
                on_seek_preview: Box::new(move |pos| {
                    on_seek_preview(VideoPlayerMessage::SeekPreview(pos))
                }),
                on_seek_release: on_message.clone()(VideoPlayerMessage::SeekRelease),
                on_toggle_fullscreen: on_message.clone()(VideoPlayerMessage::ToggleFullscreen),
            }))
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
        .width(Length::Fixed(available_width))
        .height(Length::Fixed(available_height))
        .into()
}

/// Re-export for convenience
pub use controls::glass_container_style;
pub use snowflake::snowflake_spinner;
pub use spinner::spinner;
