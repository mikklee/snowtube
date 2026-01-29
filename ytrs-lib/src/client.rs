//! InnerTube API client

use crate::constants::*;
use crate::error::{Error, Result};
use crate::models::*;
use crate::parsers;
use crate::utils;
use common::ChannelTab;
use reqwest::Client;
use serde_json::{Value, json};

/// YouTube-specific extension for ChannelTab to get InnerTube API params
trait ChannelTabExt {
    fn params(&self) -> &'static str;
}

impl ChannelTabExt for ChannelTab {
    fn params(&self) -> &'static str {
        match self {
            ChannelTab::Videos => "EgZ2aWRlb3PyBgQKAjoA",
            ChannelTab::Shorts => "EgZzaG9ydHPyBgUKA5oBAA%3D%3D",
            ChannelTab::Streams => "EgdzdHJlYW1z8gYECgJ6AA%3D%3D",
        }
    }
}

/// Main InnerTube client for interacting with YouTube's API
#[derive(Debug, Clone)]
pub struct InnerTube {
    client: Client,
    context: InnerTubeContext,
    pub(crate) api_key: String,
    #[cfg(test)]
    pub(crate) base_url: String,
}

impl InnerTube {
    /// Create a new InnerTube client
    pub async fn new() -> Result<Self> {
        let user_agent = random_user_agent();

        let client = Client::builder()
            .user_agent(&user_agent)
            .gzip(true)
            .build()?;

        let context = InnerTubeContext {
            client: InnerTubeClient {
                client_name: INNERTUBE_CLIENT_NAME.to_string(),
                client_version: INNERTUBE_CLIENT_VERSION.to_string(),
                hl: "en".to_string(),
                gl: "GB".to_string(),
                user_agent,
            },
        };

        Ok(Self {
            client,
            context,
            api_key: INNERTUBE_API_KEY.to_string(),
            #[cfg(test)]
            base_url: INNERTUBE_API_BASE.to_string(),
        })
    }

    /// Create a new InnerTube client with a custom base URL (for testing)
    #[cfg(test)]
    pub async fn new_with_base_url(base_url: String) -> Result<Self> {
        let mut client = Self::new().await?;
        client.base_url = base_url;
        Ok(client)
    }

    /// Create a new InnerTube client with custom language and region
    pub async fn with_locale(language: &str, region: &str) -> Result<Self> {
        let mut innertube = Self::new().await?;
        innertube.context.client.hl = language.to_string();
        innertube.context.client.gl = region.to_string();
        Ok(innertube)
    }

    /// Make a POST request to an InnerTube endpoint
    pub(crate) async fn post(&self, endpoint: &str, body: Value) -> Result<Value> {
        #[cfg(test)]
        let base = &self.base_url;
        #[cfg(not(test))]
        let base = INNERTUBE_API_BASE;

        let url = format!("{}{}?key={}", base, endpoint, self.api_key);
        const MAX_RETRIES: u32 = 3;

        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let response = match self.client.post(&url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(Error::from(e));
                    continue;
                }
            };

            let status = response.status();

            // Retry on 500-level errors
            if status.is_server_error() {
                eprintln!(
                    "YouTube API returned {}, attempt {}/{}, retrying...",
                    status,
                    attempt + 1,
                    MAX_RETRIES
                );

                // Exponential backoff: 1s, 2s, 4s
                if attempt < MAX_RETRIES - 1 {
                    let delay = std::time::Duration::from_secs(2_u64.pow(attempt));
                    tokio::time::sleep(delay).await;
                }

                last_error = Some(Error::ApiError(format!("API returned status: {}", status)));
                continue;
            }

            // Don't retry on client errors (4xx)
            if !status.is_success() {
                return Err(Error::ApiError(format!("API returned status: {}", status)));
            }

            // Success - parse and return
            let json = response.json::<Value>().await?;
            return Ok(json);
        }

        // All retries exhausted
        Err(last_error.unwrap_or_else(|| Error::ApiError("All retries failed".to_string())))
    }

    /// Search for videos on YouTube
    pub async fn search(&self, query: &str) -> Result<common::SearchResults> {
        // Default to English
        self.search_with_locale(query, "en", "US").await
    }

    /// Search for videos on YouTube with a specific locale
    pub async fn search_with_locale(
        &self,
        query: &str,
        hl: &str,
        gl: &str,
    ) -> Result<common::SearchResults> {
        let mut context = self.context.clone();
        context.client.hl = hl.to_string();
        context.client.gl = gl.to_string();

        let body = json!({
            "context": context,
            "query": query,
        });

        let response = self.post("/search", body).await?;
        let mut search_results = parsers::parse_search_results(&response)?;
        search_results.detected_locale = Some((hl.to_string(), gl.to_string()));
        Ok(search_results.into())
    }

    /// Get more search results using a continuation token with locale
    pub async fn search_continuation(
        &self,
        continuation_token: &str,
        hl: &str,
        gl: &str,
    ) -> Result<YtSearchResults> {
        let mut context = self.context.clone();
        context.client.hl = hl.to_string();
        context.client.gl = gl.to_string();

        let body = json!({
            "context": context,
            "continuation": continuation_token,
        });

        let response = self.post("/search", body).await?;
        let mut search_results = parsers::parse_search_results(&response)?;
        search_results.detected_locale = Some((hl.to_string(), gl.to_string()));
        Ok(search_results)
    }

    /// Get video information by video ID or URL
    pub async fn get_video(&self, video_id_or_url: &str) -> Result<VideoInfo> {
        let video_id = utils::extract_video_id(video_id_or_url)?;

        let player_body = json!({
            "context": self.context,
            "videoId": video_id,
        });

        let next_body = json!({
            "context": self.context,
            "videoId": video_id,
        });

        // Fetch both /player and /next endpoints concurrently
        // /player has streaming data, /next has full description
        let (player_response, next_response) = tokio::join!(
            self.post("/player", player_body),
            self.post("/next", next_body)
        );

        let player_response = player_response?;
        let mut video_info = parsers::parse_video_info(&player_response)?;

        // Try to get the full description and channel info from /next endpoint
        if let Ok(next_data) = next_response {
            let metadata = parsers::parse_video_metadata(&next_data);
            if let Some(desc) = metadata.description {
                video_info.description = Some(desc);
            }
            // Update channel info if available
            if let Some(name) = metadata.channel_name {
                video_info.channel.name = name;
            }
            if metadata.channel_id.is_some() {
                video_info.channel.id = metadata.channel_id;
            }
        }

        Ok(video_info)
    }

    /// Get basic video info (lightweight alternative to get_video)
    pub async fn get_basic_info(&self, video_id_or_url: &str) -> Result<VideoInfo> {
        self.get_video(video_id_or_url).await
    }

    /// Fetch additional video metadata (full description, channel info) from /next endpoint.
    /// This is useful when you have a video from search results and need the full description.
    pub async fn get_video_metadata(
        &self,
        video_id: &str,
        hl: &str,
        gl: &str,
    ) -> Result<parsers::ParsedVideoMetadata> {
        let mut context = self.context.clone();
        context.client.hl = hl.to_string();
        context.client.gl = gl.to_string();

        let body = json!({
            "context": context,
            "videoId": video_id,
        });

        let response = self.post("/next", body).await?;
        Ok(parsers::parse_video_metadata(&response))
    }

    /// Get related videos for a given video ID
    pub async fn get_related(&self, video_id: &str) -> Result<Vec<YtSearchResult>> {
        let body = json!({
            "context": self.context,
            "videoId": video_id,
        });

        let response = self.post("/next", body).await?;

        // Parse related videos from secondary results
        let mut results = Vec::new();
        if let Some(contents) = response.pointer(
            "/contents/twoColumnWatchNextResults/secondaryResults/secondaryResults/results",
        ) && let Some(items) = contents.as_array()
        {
            for item in items {
                if let Some(video) = item.get("compactVideoRenderer") {
                    // Parse compact video renderer (similar to video renderer but slightly different structure)
                    if let Some(video_id) = video.pointer("/videoId").and_then(|v| v.as_str()) {
                        results.push(YtSearchResult {
                            video_id: Some(video_id.to_string()),
                            title: "Related Video".to_string(), // Would need proper parsing
                            description: None,
                            channel: None,
                            view_count: None,
                            duration: None,
                            published_text: None,
                            thumbnails: vec![],
                            is_premium: None,
                            is_short: None,
                            badges: None,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get trending videos
    pub async fn get_trending(&self) -> Result<Vec<YtSearchResult>> {
        let body = json!({
            "context": self.context,
            "browseId": "FEtrending",
        });

        let _response = self.post("/browse", body).await?;

        // Would need proper parsing of browse endpoint response
        Ok(vec![])
    }

    /// Get channel information by channel ID
    pub async fn get_channel(&self, channel_id: &str) -> Result<YtChannelInfo> {
        let body = json!({
            "context": self.context,
            "browseId": channel_id,
        });

        let response = self.post("/browse", body).await?;
        parsers::parse_channel_info(&response)
    }

    /// Get videos from a channel
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The channel ID (e.g., "UCxxxxxx")
    /// * `tab` - The channel tab to browse (Videos, Shorts, Streams, etc.)
    pub async fn get_channel_videos(
        &self,
        channel_id: &str,
        tab: ChannelTab,
    ) -> Result<YtChannelVideos> {
        self.get_channel_videos_with_locale(channel_id, tab, None)
            .await
    }

    /// Get videos from a channel with optional locale detection
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The channel ID (e.g., "UCxxxxxx")
    /// * `tab` - The channel tab to browse (Videos, Shorts, Streams, etc.)
    /// * `locale_hint` - Optional text (e.g., channel name) to detect locale from
    pub async fn get_channel_videos_with_locale(
        &self,
        channel_id: &str,
        tab: ChannelTab,
        locale_hint: Option<&str>,
    ) -> Result<YtChannelVideos> {
        let params = tab.params();

        // Use default locale (locale_hint is ignored now that we removed auto-detection)
        let _ = locale_hint;
        let context = self.context.clone();

        let mut body = json!({
            "context": context,
            "browseId": channel_id,
        });

        // Only add params if not empty (Home tab has no params)
        if !params.is_empty() {
            body["params"] = json!(params);
        }

        let response = self.post("/browse", body).await?;
        parsers::parse_channel_videos(&response)
    }

    /// Get videos from a channel with explicit locale (hl, gl)
    ///
    /// # Arguments
    ///
    /// * `channel_id` - The channel ID (e.g., "UCxxxxxx")
    /// * `tab` - The channel tab to browse (Videos, Shorts, Streams, etc.)
    /// * `hl` - Language code (e.g., "ja", "en", "ko")
    /// * `gl` - Region code (e.g., "JP", "US", "KR")
    pub async fn get_channel_videos_with_explicit_locale(
        &self,
        channel_id: &str,
        tab: ChannelTab,
        hl: &str,
        gl: &str,
    ) -> Result<YtChannelVideos> {
        let params = tab.params();

        // Use explicit locale
        let mut context = self.context.clone();
        context.client.hl = hl.to_string();
        context.client.gl = gl.to_string();

        let mut body = json!({
            "context": context,
            "browseId": channel_id,
        });

        // Only add params if not empty (Home tab has no params)
        if !params.is_empty() {
            body["params"] = json!(params);
        }

        let response = self.post("/browse", body).await?;
        let mut channel_videos = parsers::parse_channel_videos(&response)?;
        channel_videos.detected_locale = Some((hl.to_string(), gl.to_string()));
        Ok(channel_videos)
    }

    /// Get more channel videos using a continuation token
    pub async fn get_channel_videos_continuation(
        &self,
        continuation_token: &str,
    ) -> Result<YtChannelVideos> {
        let body = json!({
            "context": self.context,
            "continuation": continuation_token,
        });

        let response = self.post("/browse", body).await?;
        parsers::parse_channel_videos(&response)
    }

    /// Get more channel videos using a continuation token with explicit locale
    pub async fn get_channel_videos_continuation_with_locale(
        &self,
        continuation_token: &str,
        hl: &str,
        gl: &str,
    ) -> Result<YtChannelVideos> {
        let mut context = self.context.clone();
        context.client.hl = hl.to_string();
        context.client.gl = gl.to_string();

        let body = json!({
            "context": context,
            "continuation": continuation_token,
        });

        let response = self.post("/browse", body).await?;
        let mut channel_videos = parsers::parse_channel_videos(&response)?;
        channel_videos.detected_locale = Some((hl.to_string(), gl.to_string()));
        Ok(channel_videos)
    }

    /// Fetch the high-resolution thumbnail for a video.
    /// Returns the image bytes if successful.
    pub async fn fetch_hq_thumbnail(&self, video_id: &str) -> Result<Vec<u8>> {
        let url = utils::get_hq_thumbnail_url(video_id);
        self.fetch_url(&url).await
    }

    /// Fetch bytes from a URL using the shared HTTP client.
    pub async fn fetch_url(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.client.get(url).send().await?;
        if response.status().is_success() {
            Ok(response.bytes().await?.to_vec())
        } else {
            Err(Error::DataNotFound(format!("Failed to fetch: {}", url)))
        }
    }

    /// Get subtitles for a video by video ID
    ///
    /// Returns a list of available manual subtitle tracks.
    /// Auto-generated captions are excluded.
    ///
    /// Note: This uses yt-dlp to fetch subtitle info because the InnerTube API
    /// no longer returns caption tracks for the WEB client.
    pub async fn get_subtitles(&self, video_id: &str) -> Result<Vec<common::Subtitle>> {
        let url = format!("https://www.youtube.com/watch?v={}", video_id);

        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("yt-dlp")
                .args(["--dump-single-json", "--skip-download", &url])
                .output()
        })
        .await
        .map_err(|e| Error::DataNotFound(format!("Task join error: {}", e)))?
        .map_err(|e| Error::DataNotFound(format!("Failed to run yt-dlp: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::DataNotFound(format!("yt-dlp failed: {}", stderr)));
        }

        let json: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| Error::DataNotFound(format!("Failed to parse yt-dlp JSON: {}", e)))?;

        // Parse subtitles from yt-dlp output (excludes auto-generated)
        let subtitles = json
            .get("subtitles")
            .and_then(|s| s.as_object())
            .map(|subs| {
                subs.iter()
                    .filter_map(|(lang_code, tracks)| {
                        // Find VTT format track
                        let vtt_track = tracks
                            .as_array()?
                            .iter()
                            .find(|t| t.get("ext").and_then(|e| e.as_str()) == Some("vtt"))?;

                        let url = vtt_track.get("url")?.as_str()?.to_string();

                        // Get language name from the track or use code as fallback
                        let language_name = vtt_track
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or(lang_code)
                            .to_string();

                        Some(common::Subtitle {
                            language_code: lang_code.clone(),
                            language_name,
                            url,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(subtitles)
    }
}

impl Default for InnerTube {
    fn default() -> Self {
        let user_agent = random_user_agent();

        Self {
            client: Client::new(),
            context: InnerTubeContext {
                client: InnerTubeClient {
                    client_name: INNERTUBE_CLIENT_NAME.to_string(),
                    client_version: INNERTUBE_CLIENT_VERSION.to_string(),
                    hl: "en".to_string(),
                    gl: "GB".to_string(),
                    user_agent,
                },
            },
            api_key: INNERTUBE_API_KEY.to_string(),
            #[cfg(test)]
            base_url: INNERTUBE_API_BASE.to_string(),
        }
    }
}
