//! PeerTube API client

use async_trait::async_trait;
use reqwest::Client;

use crate::error::{Error, Result};
use crate::models::{ApiSearchResponse, ApiVideo, PLATFORM_NAME, platform_icon};
use common::{
    ChannelConfig, ChannelInfo, ChannelProvider, ChannelTab, ChannelVideos, ContinuationToken,
    PlatformIcon, ProviderError, SearchResults, Video, VideoProvider,
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
    async fn search_sepia(&self, query: &str, start: u32, count: u32) -> Result<ApiSearchResponse> {
        let url = format!(
            "{}/api/v1/search/videos?search={}&start={}&count={}&sort=-match",
            SEPIA_SEARCH_URL,
            urlencoding::encode(query),
            start,
            count
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                message: format!("SepiaSearch failed with status: {}", response.status()),
            });
        }

        let search_response: ApiSearchResponse = response.json().await?;
        Ok(search_response)
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

    fn platform_icon(&self) -> PlatformIcon {
        platform_icon()
    }

    async fn search_with_locale(
        &self,
        query: &str,
        _hl: &str,
        _gl: &str,
    ) -> std::result::Result<SearchResults, ProviderError> {
        let response = self
            .search_sepia(query, 0, 20)
            .await
            .map_err(ProviderError::from)?;

        let has_more = response.data.len() == 20 && response.total > 20;

        Ok(SearchResults {
            results: response.data.iter().map(|v| v.to_common_video()).collect(),
            continuations: if has_more {
                vec![ContinuationToken {
                    platform_name: PLATFORM_NAME.to_string(),
                    token: "20".to_string(), // Next start offset
                }]
            } else {
                vec![]
            },
            detected_locale: None,
        })
    }

    async fn search_continuation(
        &self,
        token: &str,
        _hl: &str,
        _gl: &str,
    ) -> std::result::Result<SearchResults, ProviderError> {
        // Token is the start offset; we need the original query too
        // For now, continuation isn't fully supported - would need to encode query in token
        let start: u32 = token.parse().unwrap_or(0);

        // TODO: Properly encode query in continuation token
        // For now, return empty since we don't have the query
        Ok(SearchResults {
            results: vec![],
            continuations: vec![],
            detected_locale: None,
        })
    }

    async fn get_video(&self, id: &str) -> std::result::Result<Video, ProviderError> {
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
    /// Get video details by UUID or short UUID
    pub async fn get_video_details(&self, id: &str) -> Result<ApiVideo> {
        let url = format!("{}/api/v1/videos/{}", self.instance, id);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::VideoNotFound { id: id.to_string() });
        }

        if !response.status().is_success() {
            return Err(Error::Api {
                message: format!("Get video failed with status: {}", response.status()),
            });
        }

        let video: ApiVideo = response.json().await?;
        Ok(video)
    }

    /// Get the best quality playable URL for a video
    pub async fn get_video_url(&self, id: &str) -> Result<String> {
        let video = self.get_video_details(id).await?;
        video
            .best_file_url()
            .map(|s| s.to_string())
            .ok_or(Error::NoPlayableFile)
    }

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
}
