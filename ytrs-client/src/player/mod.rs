//! Video player using vk-video for hardware-accelerated H.264 decoding.
//!
//! This module provides video playback capabilities using Vulkan Video
//! for decoding and wgpu for rendering, integrated with iced.

mod decoder;
mod stream;

pub use decoder::{DecodedFrame, TexturesDecoder, VideoDecoder};
pub use stream::{Mp4Demuxer, VideoStream};

/// Video player state
pub struct VideoPlayer {
    /// The video decoder
    decoder: Option<VideoDecoder>,
    /// Current video stream being played
    stream: Option<VideoStream>,
    /// Playback state
    state: PlaybackState,
}

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// No video loaded
    Idle,
    /// Loading video
    Loading,
    /// Playing
    Playing,
    /// Paused
    Paused,
    /// Stopped
    Stopped,
    /// Error occurred
    Error,
}

impl VideoPlayer {
    /// Create a new video player
    pub fn new() -> Self {
        Self {
            decoder: None,
            stream: None,
            state: PlaybackState::Idle,
        }
    }

    /// Get current playback state
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Check if player is currently playing
    pub fn is_playing(&self) -> bool {
        self.state == PlaybackState::Playing
    }
}

impl Default for VideoPlayer {
    fn default() -> Self {
        Self::new()
    }
}
