//! VideoService - unified service combining multiple video providers

use std::sync::Arc;

use crate::{
    ChannelConfig, ChannelInfo, ChannelProvider, ChannelTab, ChannelVideos, NextPageToken,
    ProviderError, SearchResults, Subtitle, Video, VideoMetadata, VideoProvider,
};

/// A unified video service that combines multiple providers.
///
/// The service delegates to the appropriate provider based on platform
/// and provides combined search across all enabled platforms.
pub struct VideoService {
    providers: Vec<Arc<dyn VideoProvider>>,
    channel_providers: Vec<Arc<dyn ChannelProvider>>,
}

impl VideoService {
    /// Create a new empty VideoService
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            channel_providers: Vec::new(),
        }
    }

    /// Add a video provider
    pub fn with_provider<P: VideoProvider + 'static>(mut self, provider: P) -> Self {
        self.providers.push(Arc::new(provider));
        self
    }

    /// Add a channel provider (also adds as video provider)
    pub fn with_channel_provider<P: ChannelProvider + 'static>(mut self, provider: P) -> Self {
        let arc = Arc::new(provider);
        self.providers.push(arc.clone());
        self.channel_providers.push(arc);
        self
    }

    /// Get provider for a specific platform by name
    fn provider_for_platform(&self, platform_name: &str) -> Option<&Arc<dyn VideoProvider>> {
        self.providers
            .iter()
            .find(|p| p.platform_name() == platform_name)
    }

    /// Get channel provider for a specific platform by name
    fn channel_provider_for_platform(
        &self,
        platform_name: &str,
    ) -> Option<&Arc<dyn ChannelProvider>> {
        self.channel_providers
            .iter()
            .find(|p| p.platform_name() == platform_name)
    }

    // =========================================================================
    // Search operations (platform-agnostic)
    // =========================================================================

    /// Search across all providers and combine results
    pub async fn search(&self, query: &str) -> Result<SearchResults, ProviderError> {
        self.search_with_locale(query, "en", "US").await
    }

    /// Search across all providers with locale
    pub async fn search_with_locale(
        &self,
        query: &str,
        hl: &str,
        gl: &str,
    ) -> Result<SearchResults, ProviderError> {
        use futures::future::join_all;

        let futures: Vec<_> = self
            .providers
            .iter()
            .map(|p| async move { p.search_with_locale(query, hl, gl).await })
            .collect();

        let results = join_all(futures).await;

        let mut all_videos: Vec<Video> = Vec::new();
        let mut next_page_tokens: Vec<NextPageToken> = Vec::new();
        let mut detected_locale = None;

        for sr in results.into_iter().flatten() {
            all_videos.extend(sr.results);
            next_page_tokens.extend(sr.next_page_tokens);
            if detected_locale.is_none() {
                detected_locale = sr.detected_locale;
            }
        }

        Ok(SearchResults {
            results: all_videos,
            next_page_tokens,
            detected_locale,
        })
    }

    /// Continue search using next page tokens
    pub async fn search_next_page(
        &self,
        next_page_tokens: &[NextPageToken],
    ) -> Result<SearchResults, ProviderError> {
        use futures::future::join_all;

        tracing::debug!(
            "search_next_page called with {} tokens: {:?}",
            next_page_tokens.len(),
            next_page_tokens
                .iter()
                .map(|t| &t.platform_name)
                .collect::<Vec<_>>()
        );

        let futures: Vec<_> = next_page_tokens
            .iter()
            .filter_map(|npt| {
                self.provider_for_platform(&npt.platform_name).map(|p| {
                    let token = npt.token.clone();
                    let (hl, gl) = npt.locale.clone();
                    async move { p.search_next_page(&token, &hl, &gl).await }
                })
            })
            .collect();

        let results = join_all(futures).await;

        let mut all_videos: Vec<Video> = Vec::new();
        let mut new_next_page_tokens: Vec<NextPageToken> = Vec::new();
        let mut detected_locale = None;

        for sr in results.into_iter().flatten() {
            all_videos.extend(sr.results);
            new_next_page_tokens.extend(sr.next_page_tokens);
            if detected_locale.is_none() {
                detected_locale = sr.detected_locale;
            }
        }

        Ok(SearchResults {
            results: all_videos,
            next_page_tokens: new_next_page_tokens,
            detected_locale,
        })
    }

    // =========================================================================
    // Channel operations (use ChannelConfig or ChannelInfo for platform)
    // =========================================================================

    /// Get channel info using channel config
    pub async fn get_channel(&self, config: &ChannelConfig) -> Result<ChannelInfo, ProviderError> {
        if let Some(provider) = self.channel_provider_for_platform(&config.platform_name) {
            provider.get_channel(config).await
        } else {
            Err(ProviderError::NotFound {
                message: format!("No provider for {}", config.platform_name),
            })
        }
    }

    /// Get channel videos using channel config
    pub async fn get_channel_videos(
        &self,
        config: &ChannelConfig,
        tab: ChannelTab,
    ) -> Result<ChannelVideos, ProviderError> {
        let (hl, gl) = config
            .language
            .clone()
            .unwrap_or_else(crate::default_locale);
        self.get_channel_videos_with_locale(config, tab, &hl, &gl)
            .await
    }

    /// Get channel videos with locale
    pub async fn get_channel_videos_with_locale(
        &self,
        config: &ChannelConfig,
        tab: ChannelTab,
        hl: &str,
        gl: &str,
    ) -> Result<ChannelVideos, ProviderError> {
        if let Some(provider) = self.channel_provider_for_platform(&config.platform_name) {
            provider
                .get_channel_videos_with_locale(config, tab, hl, gl)
                .await
        } else {
            Err(ProviderError::NotFound {
                message: format!("No provider for {}", config.platform_name),
            })
        }
    }

    /// Get more channel videos using continuation
    pub async fn get_channel_videos_continuation(
        &self,
        config: &ChannelConfig,
        token: &str,
        hl: &str,
        gl: &str,
    ) -> Result<ChannelVideos, ProviderError> {
        if let Some(provider) = self.channel_provider_for_platform(&config.platform_name) {
            provider
                .get_channel_videos_continuation_with_locale(config, token, hl, gl)
                .await
        } else {
            Err(ProviderError::NotFound {
                message: format!("No provider for {}", config.platform_name),
            })
        }
    }

    // =========================================================================
    // Video operations (use Video or PlaybackInfo for platform)
    // =========================================================================

    /// Fetch thumbnail for a video
    pub async fn fetch_thumbnail_for_video(&self, video: &Video) -> Result<Vec<u8>, ProviderError> {
        if let Some(provider) = self.provider_for_platform(&video.platform_name) {
            provider.fetch_hq_thumbnail(&video.id).await
        } else if let Some(url) = video.thumbnail_url() {
            self.fetch_thumbnail(url).await
        } else {
            Err(ProviderError::NotFound {
                message: "No thumbnail available".to_string(),
            })
        }
    }

    /// Fetch thumbnail from URL using the first available provider's HTTP client
    pub async fn fetch_thumbnail(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        if let Some(provider) = self.providers.first() {
            provider.fetch_thumbnail(url).await
        } else {
            Err(ProviderError::NotFound {
                message: "No providers available".to_string(),
            })
        }
    }

    /// Get video details by platform name and video ID
    pub async fn get_video(
        &self,
        platform_name: &str,
        video_id: &str,
    ) -> Result<Video, ProviderError> {
        if let Some(provider) = self.provider_for_platform(platform_name) {
            provider.get_video(video_id).await
        } else {
            Err(ProviderError::NotFound {
                message: format!("{} provider not available", platform_name),
            })
        }
    }

    /// Get additional video metadata (full description, channel info)
    pub async fn get_video_metadata(&self, video: &Video) -> Result<VideoMetadata, ProviderError> {
        if let Some(provider) = self.provider_for_platform(&video.platform_name) {
            provider.get_video_metadata(video).await
        } else {
            Err(ProviderError::NotFound {
                message: format!("{} provider not available", video.platform_name),
            })
        }
    }

    /// Get available subtitles for a video
    pub async fn get_subtitles(&self, video: &Video) -> Result<Vec<Subtitle>, ProviderError> {
        if let Some(provider) = self.provider_for_platform(&video.platform_name) {
            provider.get_subtitles(video).await
        } else {
            Ok(vec![])
        }
    }
}

impl Default for VideoService {
    fn default() -> Self {
        Self::new()
    }
}
