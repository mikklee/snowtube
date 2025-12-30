//! Video loading logic for different source types.

use crate::source::VideoSource;
use crate::video::Video;
use gstreamer as gst;
use std::sync::Arc;
use std::sync::OnceLock;

/// Check if hardware AV1 decoding is available (VA-API or NVDEC).
fn has_hw_av1_decode() -> bool {
    static HAS_HW_AV1: OnceLock<bool> = OnceLock::new();
    *HAS_HW_AV1.get_or_init(|| {
        // Ensure GStreamer is initialized before checking element factories
        if gst::init().is_err() {
            tracing::warn!("Failed to initialize GStreamer for AV1 detection, assuming no HW AV1");
            return false;
        }
        // Check for VA-API AV1 decoder (Intel/AMD)
        let has_vaav1 = gst::ElementFactory::find("vaav1dec").is_some();
        // Check for NVDEC AV1 decoder (NVIDIA RTX 30+)
        let has_nvav1 = gst::ElementFactory::find("nvav1dec").is_some();
        let result = has_vaav1 || has_nvav1;
        tracing::info!(
            "Hardware AV1 decode: {} (vaav1dec={}, nvav1dec={})",
            result,
            has_vaav1,
            has_nvav1
        );
        result
    })
}

/// Progress updates during video loading.
#[derive(Debug, Clone)]
pub enum LoadProgress {
    /// Status message update (e.g., "Fetching video info...", "Waiting 5 seconds...")
    Status(String),
    /// Loading completed successfully.
    Done(Arc<Video>),
    /// Loading failed with an error.
    Error(String),
}

/// Load a video from the given source, yielding progress updates.
///
/// This function returns a stream that yields `LoadProgress` updates.
/// The caller should handle these updates to show loading status and
/// receive the final `Video` when ready.
pub fn load_video(
    source: VideoSource,
) -> impl futures::Stream<Item = LoadProgress> + Send + 'static {
    iced::stream::channel(10, move |mut sender| async move {
        match source {
            VideoSource::YouTube(video_id) => {
                load_youtube(&mut sender, &video_id).await;
            }
            VideoSource::YouTubeAudioOnly(video_id) => {
                load_youtube_audio_only(&mut sender, &video_id).await;
            }
            VideoSource::DirectUrl(url) => {
                load_direct_url(&mut sender, &url).await;
            }
            VideoSource::Live(url) => {
                load_live(&mut sender, &url).await;
            }
        }
    })
}

async fn load_youtube(
    sender: &mut iced::futures::channel::mpsc::Sender<LoadProgress>,
    video_id: &str,
) {
    use iced::futures::SinkExt;

    let _ = sender
        .send(LoadProgress::Status("Fetching video info...".to_string()))
        .await;

    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    // Phase 1: Run yt-dlp (blocking)
    // If no hardware AV1 decode, prefer H.264/VP9/HEVC to avoid software decode overhead
    let format_selector = if has_hw_av1_decode() {
        None
    } else {
        // Prefer vp9, then avc (H.264), then hevc, then any format as fallback
        Some("bv[vcodec^=vp9]+ba/bv[vcodec^=avc]+ba/bv[vcodec^=hev]+ba/bv+ba/b")
    };

    let yt_dlp_result = tokio::task::spawn_blocking(move || {
        tracing::info!("Starting yt-dlp for URL: {}", url);
        let mut cmd = std::process::Command::new("yt-dlp");
        if let Some(fmt) = format_selector {
            cmd.args(["-f", fmt]);
        }
        let output = cmd
            .args(["--dump-single-json", &url])
            .output()
            .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("yt-dlp failed: {}", stderr));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse yt-dlp JSON: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|r| r);

    let json: serde_json::Value = match yt_dlp_result {
        Ok(j) => j,
        Err(e) => {
            let _ = sender.send(LoadProgress::Error(e)).await;
            return;
        }
    };

    let is_live = json["is_live"].as_bool().unwrap_or(false);

    if is_live {
        let _ = sender
            .send(LoadProgress::Status("Loading live stream...".to_string()))
            .await;

        let hls_url = match json["url"].as_str() {
            Some(u) => u.to_string(),
            None => {
                let _ = sender
                    .send(LoadProgress::Error("No URL for live stream".to_string()))
                    .await;
                return;
            }
        };

        load_live(sender, &hls_url).await;
    } else {
        // VOD path
        let requested_formats = match json["requested_formats"].as_array() {
            Some(f) => f,
            None => {
                let _ = sender
                    .send(LoadProgress::Error("No formats in output".to_string()))
                    .await;
                return;
            }
        };

        let video_format = requested_formats
            .iter()
            .find(|f| f["vcodec"].as_str().map(|v| v != "none").unwrap_or(false));
        let audio_format = requested_formats
            .iter()
            .find(|f| f["acodec"].as_str().map(|a| a != "none").unwrap_or(false));

        let (video_format, audio_format) = match (video_format, audio_format) {
            (Some(v), Some(a)) => (v, a),
            _ => {
                let _ = sender
                    .send(LoadProgress::Error(
                        "Missing video/audio format".to_string(),
                    ))
                    .await;
                return;
            }
        };

        let video_url = video_format["url"].as_str().unwrap_or("").to_string();
        let audio_url = audio_format["url"].as_str().unwrap_or("").to_string();

        if video_url.is_empty() || audio_url.is_empty() {
            let _ = sender
                .send(LoadProgress::Error("Missing URLs".to_string()))
                .await;
            return;
        }

        let headers: Vec<(String, String)> = video_format["http_headers"]
            .as_object()
            .map(|h| {
                h.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        // Check for throttle wait
        // GStreamer video player already waits 5 seconds, so we only need
        // to wait the remaining time beyond that
        const GSTREAMER_WAIT_SECS: i64 = 5;
        if let Some(available_at) = video_format["available_at"].as_i64() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let wait_secs = (available_at - now - GSTREAMER_WAIT_SECS).max(0);
            if wait_secs > 0 {
                tracing::info!(
                    "Waiting {} seconds for YouTube throttle (after GStreamer's {}s)",
                    wait_secs,
                    GSTREAMER_WAIT_SECS
                );
                // Countdown each second
                for remaining in (1..=wait_secs).rev() {
                    let _ = sender
                        .send(LoadProgress::Status(format!(
                            "Waiting {} seconds...",
                            remaining
                        )))
                        .await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        let _ = sender
            .send(LoadProgress::Status("Loading video stream...".to_string()))
            .await;

        let result = tokio::task::spawn_blocking(move || {
            let header_refs: Vec<(&str, &str)> = headers
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let mut last_error = None;
            for attempt in 1..=3 {
                match Video::from_url_with_headers(&video_url, &audio_url, &header_refs) {
                    Ok(video) => return Ok(Arc::new(video)),
                    Err(e) => {
                        last_error = Some(e);
                        if attempt < 3 {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
            }
            Err(format!(
                "Failed after 3 attempts: {:?}",
                last_error.unwrap()
            ))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))
        .and_then(|r| r);

        match result {
            Ok(video) => {
                let _ = sender.send(LoadProgress::Done(video)).await;
            }
            Err(e) => {
                let _ = sender.send(LoadProgress::Error(e)).await;
            }
        }
    }
}

async fn load_youtube_audio_only(
    sender: &mut iced::futures::channel::mpsc::Sender<LoadProgress>,
    video_id: &str,
) {
    use iced::futures::SinkExt;

    tracing::info!("load_youtube_audio_only called for video_id: {}", video_id);

    let _ = sender
        .send(LoadProgress::Status("Fetching audio info...".to_string()))
        .await;

    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    // Phase 1: Run yt-dlp with audio-only format
    let yt_dlp_result = tokio::task::spawn_blocking(move || {
        tracing::info!("Starting yt-dlp (audio-only) for URL: {}", url);
        let output = std::process::Command::new("yt-dlp")
            .args(["-f", "bestaudio", "--dump-single-json", &url])
            .output()
            .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("yt-dlp failed: {}", stderr));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse yt-dlp JSON: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|r| r);

    let json: serde_json::Value = match yt_dlp_result {
        Ok(j) => j,
        Err(e) => {
            let _ = sender.send(LoadProgress::Error(e)).await;
            return;
        }
    };

    // For audio-only, we get the URL directly from the root
    let audio_url = match json["url"].as_str() {
        Some(u) => u.to_string(),
        None => {
            let _ = sender
                .send(LoadProgress::Error("No audio URL found".to_string()))
                .await;
            return;
        }
    };

    let headers: Vec<(String, String)> = json["http_headers"]
        .as_object()
        .map(|h| {
            h.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let _ = sender
        .send(LoadProgress::Status("Loading audio stream...".to_string()))
        .await;

    let result = tokio::task::spawn_blocking(move || {
        let header_refs: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let mut last_error = None;
        for attempt in 1..=3 {
            match Video::from_audio_url_only(&audio_url, &header_refs) {
                Ok(video) => return Ok(Arc::new(video)),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
            }
        }
        Err(format!(
            "Failed after 3 attempts: {:?}",
            last_error.unwrap()
        ))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|r| r);

    match result {
        Ok(video) => {
            let _ = sender.send(LoadProgress::Done(video)).await;
        }
        Err(e) => {
            let _ = sender.send(LoadProgress::Error(e)).await;
        }
    }
}

async fn load_direct_url(
    sender: &mut iced::futures::channel::mpsc::Sender<LoadProgress>,
    url: &str,
) {
    use iced::futures::SinkExt;

    let _ = sender
        .send(LoadProgress::Status("Loading video...".to_string()))
        .await;

    let url = url.to_string();
    let result = tokio::task::spawn_blocking(move || {
        let uri = url::Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;
        let mut last_error = None;
        for attempt in 1..=3 {
            match Video::new(&uri) {
                Ok(video) => return Ok(Arc::new(video)),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
            }
        }
        Err(format!(
            "Failed after 3 attempts: {:?}",
            last_error.unwrap()
        ))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|r| r);

    match result {
        Ok(video) => {
            let _ = sender.send(LoadProgress::Done(video)).await;
        }
        Err(e) => {
            let _ = sender.send(LoadProgress::Error(e)).await;
        }
    }
}

async fn load_live(sender: &mut iced::futures::channel::mpsc::Sender<LoadProgress>, url: &str) {
    use iced::futures::SinkExt;

    let _ = sender
        .send(LoadProgress::Status("Loading live stream...".to_string()))
        .await;

    let url = url.to_string();
    let result = tokio::task::spawn_blocking(move || {
        let uri = url::Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;
        let mut last_error = None;
        for attempt in 1..=3 {
            match Video::new(&uri) {
                Ok(video) => return Ok(Arc::new(video)),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
            }
        }
        Err(format!(
            "Failed after 3 attempts: {:?}",
            last_error.unwrap()
        ))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|r| r);

    match result {
        Ok(video) => {
            let _ = sender.send(LoadProgress::Done(video)).await;
        }
        Err(e) => {
            let _ = sender.send(LoadProgress::Error(e)).await;
        }
    }
}
