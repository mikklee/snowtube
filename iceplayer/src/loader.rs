//! Video loading logic for different source types.

use crate::source::VideoSource;
use crate::video::Video;
use gstreamer as gst;
use std::sync::Arc;
use std::sync::OnceLock;

/// Check if hardware AV1 decoding is available.
/// Checks for VA-API (Intel/AMD), NVDEC (NVIDIA) on Linux
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
            VideoSource::YouTube { video_id } => {
                load_youtube(&mut sender, &video_id).await;
            }
            VideoSource::YouTubeAudioOnly { video_id } => {
                load_youtube_audio_only(&mut sender, &video_id).await;
            }
            VideoSource::PeerTube { instance, video_id } => {
                load_peertube(&mut sender, &instance, &video_id).await;
            }
            VideoSource::PeerTubeAudioOnly { instance, video_id } => {
                load_peertube_audio_only(&mut sender, &instance, &video_id).await;
            }
            VideoSource::DirectUrl(url) => {
                load_with_playbin(&mut sender, &url).await;
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
    // Exclude AI-upscaled formats (format_note contains "upscaled") as they may have issues
    // If no hardware AV1 decode, also prefer H.264/VP9/HEVC to avoid software decode overhead
    let format_selector = if has_hw_av1_decode() {
        // Still exclude AI-upscaled formats
        Some("bv[format_note!*=upscaled]+ba/bv+ba/b")
    } else {
        // Prefer vp9, then avc (H.264), then hevc, exclude AI-upscaled
        Some(
            "bv[vcodec^=vp9][format_note!*=upscaled]+ba/bv[vcodec^=avc][format_note!*=upscaled]+ba/bv[vcodec^=hev][format_note!*=upscaled]+ba/bv[format_note!*=upscaled]+ba/b",
        )
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

    // Phase 1: Run yt-dlp to get video info
    let yt_dlp_result = tokio::task::spawn_blocking(move || {
        tracing::info!("Starting yt-dlp (audio-only) for URL: {}", url);
        let output = std::process::Command::new("yt-dlp")
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
        // Live streams don't have separate audio-only formats
        // Use the HLS URL with audio-only playbin
        tracing::info!("Detected live stream, using audio-only playbin");

        let _ = sender
            .send(LoadProgress::Status(
                "Loading live stream (audio only)...".to_string(),
            ))
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

        load_youtube_live_audio_only(sender, &hls_url).await;
    } else {
        // VOD: need to fetch again with bestaudio format
        let url = format!("https://www.youtube.com/watch?v={}", video_id);

        let yt_dlp_audio_result = tokio::task::spawn_blocking(move || {
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

        let audio_json: serde_json::Value = match yt_dlp_audio_result {
            Ok(j) => j,
            Err(e) => {
                let _ = sender.send(LoadProgress::Error(e)).await;
                return;
            }
        };

        let audio_url = match audio_json["url"].as_str() {
            Some(u) => u.to_string(),
            None => {
                let _ = sender
                    .send(LoadProgress::Error("No audio URL found".to_string()))
                    .await;
                return;
            }
        };

        let headers: Vec<(String, String)> = audio_json["http_headers"]
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
}

async fn load_youtube_live_audio_only(
    sender: &mut iced::futures::channel::mpsc::Sender<LoadProgress>,
    url: &str,
) {
    use iced::futures::SinkExt;

    let url = url.to_string();
    let result = tokio::task::spawn_blocking(move || {
        let uri = url::Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;
        let mut last_error = None;
        for attempt in 1..=3 {
            match Video::youtube_live_audio_only(&uri) {
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

/// Load video using GStreamer's playbin element.
/// This handles HLS streams (.m3u8), direct file URLs, and other formats.
async fn load_with_playbin(
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

async fn load_peertube(
    sender: &mut iced::futures::channel::mpsc::Sender<LoadProgress>,
    instance: &str,
    video_id: &str,
) {
    use iced::futures::SinkExt;

    let _ = sender
        .send(LoadProgress::Status(
            "Fetching PeerTube video info...".to_string(),
        ))
        .await;

    // Fetch video details from PeerTube API
    let api_url = format!("{}/api/v1/videos/{}", instance, video_id);

    let client = match reqwest::Client::builder().user_agent("ytrs/0.1").build() {
        Ok(c) => c,
        Err(e) => {
            let _ = sender
                .send(LoadProgress::Error(format!(
                    "Failed to create HTTP client: {}",
                    e
                )))
                .await;
            return;
        }
    };

    let video_info: serde_json::Value = match client.get(&api_url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                let _ = sender
                    .send(LoadProgress::Error(format!(
                        "PeerTube API error: {}",
                        response.status()
                    )))
                    .await;
                return;
            }
            match response.json().await {
                Ok(json) => json,
                Err(e) => {
                    let _ = sender
                        .send(LoadProgress::Error(format!(
                            "Failed to parse response: {}",
                            e
                        )))
                        .await;
                    return;
                }
            }
        }
        Err(e) => {
            let _ = sender
                .send(LoadProgress::Error(format!("Network error: {}", e)))
                .await;
            return;
        }
    };

    // Extract best quality video URL
    // Prefer HLS playlist URLs (.m3u8) as they support proper seeking
    // Fragmented MP4 files have duration=0 in their moov atom, breaking seeking
    let video_url = video_info["streamingPlaylists"]
        .as_array()
        .and_then(|playlists| {
            playlists.first().and_then(|p| {
                // Prefer HLS playlist URL - it supports seeking with proper duration
                // Find highest resolution HLS playlist
                p["files"]
                    .as_array()
                    .and_then(|files| {
                        files
                            .iter()
                            .filter_map(|f| {
                                let url = f["playlistUrl"].as_str()?;
                                let res = f["resolution"]["id"].as_u64().unwrap_or(0);
                                Some((res, url.to_string()))
                            })
                            .max_by_key(|(res, _)| *res)
                            .map(|(_, url)| url)
                    })
                    // Fall back to main playlist URL
                    .or_else(|| p["playlistUrl"].as_str().map(|s| s.to_string()))
            })
        })
        // Fall back to direct files if no streaming playlists (older PeerTube instances)
        .or_else(|| {
            video_info["files"].as_array().and_then(|files| {
                files
                    .iter()
                    .filter_map(|f| {
                        let url = f["fileUrl"].as_str()?;
                        let res = f["resolution"]["id"].as_u64().unwrap_or(0);
                        Some((res, url.to_string()))
                    })
                    .max_by_key(|(res, _)| *res)
                    .map(|(_, url)| url)
            })
        });

    let video_url = match video_url {
        Some(url) => url,
        None => {
            let _ = sender
                .send(LoadProgress::Error(
                    "No playable video URL found".to_string(),
                ))
                .await;
            return;
        }
    };

    let _ = sender
        .send(LoadProgress::Status(
            "Loading PeerTube video...".to_string(),
        ))
        .await;

    // Use playbin for all PeerTube URLs - it handles both HLS (.m3u8) and direct files
    load_with_playbin(sender, &video_url).await;
}

async fn load_peertube_audio_only(
    sender: &mut iced::futures::channel::mpsc::Sender<LoadProgress>,
    instance: &str,
    video_id: &str,
) {
    use iced::futures::SinkExt;

    let _ = sender
        .send(LoadProgress::Status(
            "Fetching PeerTube video info...".to_string(),
        ))
        .await;

    // Fetch video info from PeerTube API
    let api_url = format!("{}/api/v1/videos/{}", instance, video_id);
    let client = reqwest::Client::new();

    let video_info: serde_json::Value = match client.get(&api_url).send().await {
        Ok(resp) => match resp.json().await {
            Ok(json) => json,
            Err(e) => {
                let _ = sender
                    .send(LoadProgress::Error(format!(
                        "Failed to parse response: {}",
                        e
                    )))
                    .await;
                return;
            }
        },
        Err(e) => {
            let _ = sender
                .send(LoadProgress::Error(format!(
                    "Failed to fetch video info: {}",
                    e
                )))
                .await;
            return;
        }
    };

    // Extract video URL (prefer HLS, fall back to direct files)
    let video_url = video_info["streamingPlaylists"]
        .as_array()
        .and_then(|playlists| {
            playlists.first().and_then(|p| {
                p["files"]
                    .as_array()
                    .and_then(|files| {
                        files
                            .iter()
                            .filter_map(|f| {
                                let url = f["playlistUrl"].as_str()?;
                                let res = f["resolution"]["id"].as_u64().unwrap_or(0);
                                Some((res, url.to_string()))
                            })
                            .max_by_key(|(res, _)| *res)
                            .map(|(_, url)| url)
                    })
                    .or_else(|| p["playlistUrl"].as_str().map(|s| s.to_string()))
            })
        })
        .or_else(|| {
            video_info["files"].as_array().and_then(|files| {
                files
                    .iter()
                    .filter_map(|f| {
                        let url = f["fileUrl"].as_str()?;
                        let res = f["resolution"]["id"].as_u64().unwrap_or(0);
                        Some((res, url.to_string()))
                    })
                    .max_by_key(|(res, _)| *res)
                    .map(|(_, url)| url)
            })
        });

    let video_url = match video_url {
        Some(url) => url,
        None => {
            let _ = sender
                .send(LoadProgress::Error(
                    "No playable video URL found".to_string(),
                ))
                .await;
            return;
        }
    };

    let _ = sender
        .send(LoadProgress::Status(
            "Loading PeerTube audio...".to_string(),
        ))
        .await;

    // Use peertube_audio_only which discards video but keeps audio with visualizer
    let result = tokio::task::spawn_blocking(move || {
        let uri = url::Url::parse(&video_url).map_err(|e| format!("Invalid URL: {}", e))?;
        Video::peertube_audio_only(&uri)
            .map(Arc::new)
            .map_err(|e| format!("Failed to create player: {:?}", e))
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
