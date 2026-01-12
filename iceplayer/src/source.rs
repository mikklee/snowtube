//! Video source types for the video player.

/// Represents the source of a video to be played.
#[derive(Debug, Clone)]
pub enum VideoSource {
    /// YouTube video by ID
    YouTube { video_id: String },
    /// YouTube audio-only by ID
    YouTubeAudioOnly { video_id: String },
    /// PeerTube video
    PeerTube { instance: String, video_id: String },
    /// PeerTube audio-only
    PeerTubeAudioOnly { instance: String, video_id: String },
    /// Direct video URL (e.g., file:// or https://)
    DirectUrl(String),
    /// Live stream URL (HLS/DASH)
    Live(String),
}

impl VideoSource {
    /// Create a VideoSource from a common::Video
    pub fn from_video(video: &common::Video) -> Result<Self, String> {
        match video.platform_name.as_str() {
            "youtube" => Ok(Self::YouTube {
                video_id: video.id.clone(),
            }),
            "peertube" => Ok(Self::PeerTube {
                instance: video.instance.clone().unwrap_or_default(),
                video_id: video.id.clone(),
            }),
            other => Err(format!("Unknown platform: {}", other)),
        }
    }

    /// Create a VideoSource for audio-only from a common::Video
    pub fn from_video_audio_only(video: &common::Video) -> Result<Self, String> {
        match video.platform_name.as_str() {
            "youtube" => Ok(Self::YouTubeAudioOnly {
                video_id: video.id.clone(),
            }),
            "peertube" => Ok(Self::PeerTubeAudioOnly {
                instance: video.instance.clone().unwrap_or_default(),
                video_id: video.id.clone(),
            }),
            other => Err(format!("Audio-only not supported for platform: {}", other)),
        }
    }

    /// Create a YouTube source from a video ID.
    pub fn youtube(id: impl Into<String>) -> Self {
        Self::YouTube {
            video_id: id.into(),
        }
    }

    /// Create a YouTube audio-only source from a video ID.
    pub fn youtube_audio_only(id: impl Into<String>) -> Self {
        Self::YouTubeAudioOnly {
            video_id: id.into(),
        }
    }

    /// Create a PeerTube source from instance URL and video ID.
    pub fn peertube(instance: impl Into<String>, video_id: impl Into<String>) -> Self {
        Self::PeerTube {
            instance: instance.into(),
            video_id: video_id.into(),
        }
    }

    /// Create a direct URL source.
    pub fn direct_url(url: impl Into<String>) -> Self {
        Self::DirectUrl(url.into())
    }

    /// Create a live stream source.
    pub fn live(url: impl Into<String>) -> Self {
        Self::Live(url.into())
    }

    /// Check if this source is audio-only.
    pub fn is_audio_only(&self) -> bool {
        matches!(
            self,
            Self::YouTubeAudioOnly { .. } | Self::PeerTubeAudioOnly { .. }
        )
    }
}
