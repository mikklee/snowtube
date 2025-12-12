//! Player events emitted to the parent application.

use std::time::Duration;

/// High-level events emitted by the video player widget.
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    /// Video has loaded and is ready to play.
    Ready {
        /// Total duration of the video.
        duration: Duration,
    },
    /// Video playback has ended.
    Ended,
    /// An error occurred during loading or playback.
    Error(String),
    /// Fullscreen state changed.
    FullscreenChanged(bool),
    /// Play/pause state changed.
    PlayStateChanged {
        /// True if playing, false if paused.
        playing: bool,
    },
}
