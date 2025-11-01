//! InnerTube API client

use crate::constants::*;
use crate::error::{Error, Result};
use crate::models::*;
use crate::parsers;
use crate::utils;
use reqwest::Client;
use serde_json::{Value, json};

/// Main InnerTube client for interacting with YouTube's API
#[derive(Debug, Clone)]
pub struct InnerTube {
    client: Client,
    context: InnerTubeContext,
    api_key: String,
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
                user_agent: user_agent,
            },
        };

        Ok(Self {
            client,
            context,
            api_key: INNERTUBE_API_KEY.to_string(),
        })
    }

    /// Create a new InnerTube client with custom language and region
    pub async fn with_locale(language: &str, region: &str) -> Result<Self> {
        let mut innertube = Self::new().await?;
        innertube.context.client.hl = language.to_string();
        innertube.context.client.gl = region.to_string();
        Ok(innertube)
    }

    /// Make a POST request to an InnerTube endpoint
    async fn post(&self, endpoint: &str, body: Value) -> Result<Value> {
        let url = format!("{}{}?key={}", INNERTUBE_API_BASE, endpoint, self.api_key);

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            return Err(Error::ApiError(format!(
                "API returned status: {}",
                response.status()
            )));
        }

        let json = response.json::<Value>().await?;
        Ok(json)
    }

    /// Search for videos on YouTube
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        // Detect language and update context
        let (hl, gl) = utils::detect_locale(query);
        let mut context = self.context.clone();
        context.client.hl = hl;
        context.client.gl = gl;

        let body = json!({
            "context": context,
            "query": query,
        });

        let response = self.post("/search", body).await?;
        parsers::parse_search_results(&response)
    }

    /// Get video information by video ID or URL
    pub async fn get_video(&self, video_id_or_url: &str) -> Result<VideoInfo> {
        let video_id = utils::extract_video_id(video_id_or_url)?;

        let body = json!({
            "context": self.context,
            "videoId": video_id,
        });

        let response = self.post("/player", body).await?;
        parsers::parse_video_info(&response)
    }

    /// Get basic video info (lightweight alternative to get_video)
    pub async fn get_basic_info(&self, video_id_or_url: &str) -> Result<VideoInfo> {
        self.get_video(video_id_or_url).await
    }

    /// Get the next page of results (for pagination)
    pub async fn get_continuation(&self, continuation_token: &str) -> Result<Vec<SearchResult>> {
        let body = json!({
            "context": self.context,
            "continuation": continuation_token,
        });

        let response = self.post("/search", body).await?;
        parsers::parse_search_results(&response)
    }

    /// Get related videos for a given video ID
    pub async fn get_related(&self, video_id: &str) -> Result<Vec<SearchResult>> {
        let body = json!({
            "context": self.context,
            "videoId": video_id,
        });

        let response = self.post("/next", body).await?;

        // Parse related videos from secondary results
        let mut results = Vec::new();
        if let Some(contents) = response.pointer(
            "/contents/twoColumnWatchNextResults/secondaryResults/secondaryResults/results",
        ) {
            if let Some(items) = contents.as_array() {
                for item in items {
                    if let Some(video) = item.get("compactVideoRenderer") {
                        // Parse compact video renderer (similar to video renderer but slightly different structure)
                        if let Some(video_id) = video.pointer("/videoId").and_then(|v| v.as_str()) {
                            results.push(SearchResult {
                                video_id: Some(video_id.to_string()),
                                title: "Related Video".to_string(), // Would need proper parsing
                                description: None,
                                channel: None,
                                view_count: None,
                                duration: None,
                                published_text: None,
                                thumbnails: vec![],
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get trending videos
    pub async fn get_trending(&self) -> Result<Vec<SearchResult>> {
        let body = json!({
            "context": self.context,
            "browseId": "FEtrending",
        });

        let _response = self.post("/browse", body).await?;

        // Would need proper parsing of browse endpoint response
        Ok(vec![])
    }

    /// Get channel information by channel ID
    pub async fn get_channel(&self, channel_id: &str) -> Result<ChannelInfo> {
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
    ) -> Result<ChannelVideos> {
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
    ) -> Result<ChannelVideos> {
        let params = tab.params();

        // Detect locale if hint is provided
        let mut context = self.context.clone();
        let detected_locale = if let Some(hint) = locale_hint {
            let (hl, gl) = utils::detect_locale(hint);
            context.client.hl = hl.clone();
            context.client.gl = gl.clone();
            Some((hl, gl))
        } else {
            None
        };

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
        channel_videos.detected_locale = detected_locale;
        Ok(channel_videos)
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
    ) -> Result<ChannelVideos> {
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
    ) -> Result<ChannelVideos> {
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
    ) -> Result<ChannelVideos> {
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
                    user_agent: user_agent,
                },
            },
            api_key: INNERTUBE_API_KEY.to_string(),
        }
    }
}
