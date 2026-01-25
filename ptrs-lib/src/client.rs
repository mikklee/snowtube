//! PeerTube API client

use async_trait::async_trait;
use reqwest::Client;

use crate::error::{Error, Result};
use crate::models::{ApiSearchResponse, PLATFORM_NAME};
use common::{
    ChannelConfig, ChannelInfo, ChannelProvider, ChannelTab, ChannelVideos, NextPageToken,
    ProviderError, SearchResults, Video, VideoMetadata, VideoProvider,
};

const SEPIA_SEARCH_URL: &str = "https://sepiasearch.org";

/// PeerTube client using SepiaSearch for federated search
#[derive(Debug, Clone)]
pub struct PeerTubeClient {
    client: Client,
}

impl PeerTubeClient {
    /// Create a new PeerTube client
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("ytrs/0.1")
            .gzip(true)
            .build()?;

        Ok(Self { client })
    }

    /// Create an instance-specific helper for direct API calls
    fn for_instance(&self, instance: &str) -> PeerTube {
        PeerTube {
            client: self.client.clone(),
            instance: instance.trim_end_matches('/').to_string(),
        }
    }

    /// Search videos using SepiaSearch (federated search)
    async fn search_sepia(
        &self,
        query: &str,
        start: u32,
        count: u32,
        language: Option<&str>,
    ) -> Result<ApiSearchResponse> {
        let mut url = format!(
            "{}/api/v1/search/videos?search={}&start={}&count={}&sort=-match",
            SEPIA_SEARCH_URL,
            urlencoding::encode(query),
            start,
            count
        );

        // Add language filter if specified
        if let Some(lang) = language {
            url.push_str(&format!("&languageOneOf={}", urlencoding::encode(lang)));
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                message: format!("SepiaSearch failed with status: {}", response.status()),
            });
        }

        let search_response: ApiSearchResponse = response.json().await?;
        Ok(search_response)
    }

    /// Get subtitles for a video
    ///
    /// # Arguments
    /// * `instance` - The PeerTube instance URL (e.g., "https://video.example.com")
    /// * `video_id` - The video UUID
    pub async fn get_subtitles(
        &self,
        instance: &str,
        video_id: &str,
    ) -> Result<Vec<common::Subtitle>> {
        let pt = self.for_instance(instance);
        pt.get_subtitles(video_id).await
    }
}

impl Default for PeerTubeClient {
    fn default() -> Self {
        Self::new().expect("Failed to create PeerTube HTTP client")
    }
}

#[async_trait]
impl VideoProvider for PeerTubeClient {
    fn platform_name(&self) -> &'static str {
        PLATFORM_NAME
    }

    async fn search_with_locale(
        &self,
        query: &str,
        hl: &str,
        _gl: &str,
    ) -> std::result::Result<SearchResults, ProviderError> {
        // Pass language filter if not default "en"
        let language = if hl.is_empty() || hl == "en" {
            None
        } else {
            Some(hl)
        };

        let response = self
            .search_sepia(query, 0, 20, language)
            .await
            .map_err(ProviderError::from)?;

        let has_more = response.data.len() == 20 && response.total > 20;
        let next_offset = 20u32;

        tracing::debug!(
            "PeerTube search_with_locale: query={}, got {} results, total={}, has_more={}",
            query,
            response.data.len(),
            response.total,
            has_more
        );

        Ok(SearchResults {
            results: response.data.iter().map(|v| v.to_common_video()).collect(),
            next_page_tokens: if has_more {
                vec![NextPageToken {
                    platform_name: PLATFORM_NAME.to_string(),
                    token: encode_next_page_token(query, next_offset),
                    locale: (hl.to_string(), _gl.to_string()),
                }]
            } else {
                vec![]
            },
            detected_locale: None,
        })
    }

    async fn search_next_page(
        &self,
        token: &str,
        hl: &str,
        _gl: &str,
    ) -> std::result::Result<SearchResults, ProviderError> {
        tracing::debug!("PeerTube search_next_page called with token: {}", token);

        let (query, offset) = decode_next_page_token(token).ok_or_else(|| ProviderError::Api {
            message: "Invalid next page token".to_string(),
        })?;

        tracing::debug!(
            "PeerTube search_next_page decoded: query={}, offset={}",
            query,
            offset
        );

        let language = if hl.is_empty() || hl == "en" {
            None
        } else {
            Some(hl.to_string())
        };

        let response = self
            .search_sepia(&query, offset, 20, language.as_deref())
            .await
            .map_err(ProviderError::from)?;

        let has_more = response.data.len() == 20 && (offset + 20) < response.total;
        let next_offset = offset + 20;

        tracing::debug!(
            "PeerTube search_next_page: got {} results, total={}, has_more={}",
            response.data.len(),
            response.total,
            has_more
        );

        Ok(SearchResults {
            results: response.data.iter().map(|v| v.to_common_video()).collect(),
            next_page_tokens: if has_more {
                vec![NextPageToken {
                    platform_name: PLATFORM_NAME.to_string(),
                    token: encode_next_page_token(&query, next_offset),
                    locale: (hl.to_string(), _gl.to_string()),
                }]
            } else {
                vec![]
            },
            detected_locale: None,
        })
    }

    async fn get_video(&self, _id: &str) -> std::result::Result<Video, ProviderError> {
        // id format should be "instance|video_id" for PeerTube
        // Or we need to look it up via the video's stored instance
        Err(ProviderError::Api {
            message: "Use get_video_from_instance for PeerTube videos".to_string(),
        })
    }

    async fn fetch_thumbnail(&self, url: &str) -> std::result::Result<Vec<u8>, ProviderError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ProviderError::Network {
                message: e.to_string(),
            })?;

        if response.status().is_success() {
            Ok(response
                .bytes()
                .await
                .map_err(|e| ProviderError::Network {
                    message: e.to_string(),
                })?
                .to_vec())
        } else {
            Err(ProviderError::Api {
                message: format!("Failed to fetch: {}", url),
            })
        }
    }

    async fn fetch_hq_thumbnail(
        &self,
        _video_id: &str,
    ) -> std::result::Result<Vec<u8>, ProviderError> {
        // For PeerTube, thumbnails are fetched via URL from the video data
        Err(ProviderError::Api {
            message: "Use fetch_thumbnail with URL for PeerTube".to_string(),
        })
    }

    // PeerTube does not do forced machine translations like YouTube,
    // so locale parameters are unused here.
    async fn get_video_metadata(
        &self,
        video: &Video,
        _hl: &str,
        _gl: &str,
    ) -> std::result::Result<VideoMetadata, ProviderError> {
        let instance = video.instance.as_ref().ok_or_else(|| ProviderError::Api {
            message: "PeerTube video requires instance URL for metadata".to_string(),
        })?;

        let pt = self.for_instance(instance);
        let video_details = pt
            .get_video(&video.id)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })?;

        // Get channel avatar URL (largest available)
        let channel_avatar_url = video_details
            .channel
            .avatars
            .last()
            .map(|a| format!("{}{}", instance, a.path));

        Ok(VideoMetadata {
            description: video_details.description,
            channel_name: Some(video_details.channel.display_name),
            channel_id: Some(video_details.channel.name),
            channel_avatar_url,
        })
    }

    async fn get_subtitles(
        &self,
        video: &Video,
    ) -> std::result::Result<Vec<common::Subtitle>, ProviderError> {
        let instance = video.instance.as_ref().ok_or_else(|| ProviderError::Api {
            message: "PeerTube video requires instance URL for subtitles".to_string(),
        })?;

        self.get_subtitles(instance, &video.id)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })
    }
}

#[async_trait]
impl ChannelProvider for PeerTubeClient {
    async fn get_channel(
        &self,
        config: &ChannelConfig,
    ) -> std::result::Result<ChannelInfo, ProviderError> {
        let instance = config.instance.as_ref().ok_or_else(|| ProviderError::Api {
            message: "PeerTube channel requires instance URL".to_string(),
        })?;

        let pt = self.for_instance(instance);
        let channel = pt
            .get_channel_info(&config.channel_id)
            .await
            .map_err(ProviderError::from)?;

        Ok(channel.to_channel_info(instance))
    }

    async fn get_channel_videos_with_locale(
        &self,
        config: &ChannelConfig,
        _tab: ChannelTab,
        _hl: &str,
        _gl: &str,
    ) -> std::result::Result<ChannelVideos, ProviderError> {
        let instance = config.instance.as_ref().ok_or_else(|| ProviderError::Api {
            message: "PeerTube channel requires instance URL".to_string(),
        })?;

        let pt = self.for_instance(instance);
        let response = pt
            .get_channel_videos(&config.channel_id, 0, 30)
            .await
            .map_err(ProviderError::from)?;

        Ok(ChannelVideos {
            videos: response
                .data
                .iter()
                .map(|v| v.to_common_video_with_instance(instance))
                .collect(),
            continuation: None,
            sort_filters: None,
            detected_locale: None,
        })
    }

    async fn get_channel_videos_continuation_with_locale(
        &self,
        _config: &ChannelConfig,
        _token: &str,
        _hl: &str,
        _gl: &str,
    ) -> std::result::Result<ChannelVideos, ProviderError> {
        // TODO: implement pagination
        Ok(ChannelVideos {
            videos: vec![],
            continuation: None,
            sort_filters: None,
            detected_locale: None,
        })
    }
}

/// Internal helper for instance-specific PeerTube API calls
#[derive(Debug, Clone)]
pub(crate) struct PeerTube {
    client: Client,
    instance: String,
}

impl PeerTube {
    /// Get channel details by handle
    pub async fn get_channel_info(&self, handle: &str) -> Result<crate::models::ApiVideoChannel> {
        let url = format!("{}/api/v1/video-channels/{}", self.instance, handle);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Api {
                message: format!("Channel not found: {}", handle),
            });
        }

        if !response.status().is_success() {
            return Err(Error::Api {
                message: format!("Get channel failed with status: {}", response.status()),
            });
        }

        let channel: crate::models::ApiVideoChannel = response.json().await?;
        Ok(channel)
    }

    /// Get videos from a channel
    pub async fn get_channel_videos(
        &self,
        handle: &str,
        start: u32,
        count: u32,
    ) -> Result<ApiSearchResponse> {
        let url = format!(
            "{}/api/v1/video-channels/{}/videos?start={}&count={}",
            self.instance, handle, start, count
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                message: format!(
                    "Get channel videos failed with status: {}",
                    response.status()
                ),
            });
        }

        let videos: ApiSearchResponse = response.json().await?;
        Ok(videos)
    }

    /// Get video details by UUID
    pub async fn get_video(&self, uuid: &str) -> Result<crate::models::ApiVideo> {
        let url = format!("{}/api/v1/videos/{}", self.instance, uuid);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::Api {
                message: format!("Video not found: {}", uuid),
            });
        }

        if !response.status().is_success() {
            return Err(Error::Api {
                message: format!("Get video failed with status: {}", response.status()),
            });
        }

        let video: crate::models::ApiVideo = response.json().await?;
        Ok(video)
    }

    /// Get subtitles/captions for a video by UUID
    pub async fn get_subtitles(&self, uuid: &str) -> Result<Vec<common::Subtitle>> {
        let url = format!("{}/api/v1/videos/{}/captions", self.instance, uuid);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // No subtitles available
            return Ok(vec![]);
        }

        if !response.status().is_success() {
            return Err(Error::Api {
                message: format!("Get subtitles failed with status: {}", response.status()),
            });
        }

        let captions: crate::models::ApiSubtitlesResponse = response.json().await?;
        Ok(captions
            .data
            .iter()
            .filter_map(|c| c.to_common_subtitle(&self.instance))
            .collect())
    }
}

/// Encode query and offset into a next page token
fn encode_next_page_token(query: &str, offset: u32) -> String {
    format!("{}|{}", query, offset)
}

/// Decode a next page token into query and offset
fn decode_next_page_token(token: &str) -> Option<(String, u32)> {
    let (query, offset_str) = token.rsplit_once('|')?;
    let offset = offset_str.parse().ok()?;
    Some((query.to_string(), offset))
}
