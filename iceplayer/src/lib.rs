//! # Iced Video Player
//!
//! A convenient video player widget for Iced.
//!
//! ## Low-Level API
//!
//! For direct control, load a video from a URI using [`Video::new`](crate::Video::new),
//! then use [`VideoPlayer`] widget in your view function.
//!
//! ## High-Level API
//!
//! For a self-contained player with built-in controls, loading, and overlays,
//! use the [`widget`] module:
//!
//! ```rust,ignore
//! use iceplayer::{VideoSource, VideoPlayerState, VideoPlayerMessage, PlayerEvent, start_loading};
//! use iceplayer::widget::{view, update, subscription};
//!
//! // Create state and start loading task
//! let source = VideoSource::YouTube("dQw4w9WgXcQ".into());
//! let state = VideoPlayerState::new(source.clone()).with_title("My Video");
//! let load_task = start_loading(source).map(Message::VideoPlayer);
//!
//! // In your update function, handle VideoPlayerMessage and get PlayerEvent
//! let (event, task) = update(&mut state, message);
//!
//! // In your view function
//! view(&state, Message::VideoPlayer, width, height, &theme)
//!
//! // In your subscription function
//! subscription(&state).map(Message::VideoPlayer)
//! ```

mod event;
mod led_visualizer;
mod loader;
mod pipeline;
mod source;
mod subtitle;
mod video;
mod video_player;
mod visualizer;
pub mod widget;

use gstreamer as gst;
use thiserror::Error;

pub use video::Position;
pub use video::Video;
pub use video_player::VideoPlayer;

pub use event::PlayerEvent;
pub use led_visualizer::LedVisualizer;
pub use source::VideoSource;
pub use visualizer::{AudioVisualizer, Visualizer};
pub use widget::{VideoPlayerMessage, VideoPlayerState, start_loading};

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Glib(#[from] glib::Error),
    #[error("{0}")]
    Bool(#[from] glib::BoolError),
    #[error("failed to get the gstreamer bus")]
    Bus,
    #[error("failed to get AppSink element with name='{0}' from gstreamer pipeline")]
    AppSink(String),
    #[error("{0}")]
    StateChange(#[from] gst::StateChangeError),
    #[error("failed to cast gstreamer element")]
    Cast,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("invalid URI")]
    Uri,
    #[error("failed to get media capabilities")]
    Caps,
    #[error("failed to query media duration or position")]
    Duration,
    #[error("failed to sync with playback")]
    Sync,
    #[error("failed to lock internal sync primitive")]
    Lock,
    #[error("invalid framerate: {0}")]
    Framerate(f64),
}
