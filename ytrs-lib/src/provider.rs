//! VideoProvider and ChannelProvider trait implementations for InnerTube

use async_trait::async_trait;
use common::{
    ChannelConfig, ChannelInfo, ChannelProvider, ChannelTab, ChannelVideos, PlatformIcon,
    ProviderError, SearchResults, Video, VideoProvider,
};

use crate::client::InnerTube;
use crate::models::{PLATFORM_NAME, platform_icon};

#[async_trait]
impl VideoProvider for InnerTube {
    fn platform_name(&self) -> &'static str {
        PLATFORM_NAME
    }

    fn platform_icon(&self) -> PlatformIcon {
        platform_icon()
    }

    async fn search_with_locale(
        &self,
        query: &str,
        hl: &str,
        gl: &str,
    ) -> Result<SearchResults, ProviderError> {
        let results = InnerTube::search_with_locale(self, query, hl, gl)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })?;

        Ok(results)
    }

    async fn search_continuation(
        &self,
        token: &str,
        hl: &str,
        gl: &str,
    ) -> Result<SearchResults, ProviderError> {
        let results = InnerTube::search_continuation(self, token, hl, gl)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })?;

        Ok(results.into())
    }

    async fn get_video(&self, id: &str) -> Result<Video, ProviderError> {
        let video = InnerTube::get_video(self, id)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })?;

        Ok(video.into())
    }

    async fn fetch_thumbnail(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        InnerTube::fetch_url(self, url)
            .await
            .map_err(|e| ProviderError::Network {
                message: e.to_string(),
            })
    }

    async fn fetch_hq_thumbnail(&self, video_id: &str) -> Result<Vec<u8>, ProviderError> {
        InnerTube::fetch_hq_thumbnail(self, video_id)
            .await
            .map_err(|e| ProviderError::Network {
                message: e.to_string(),
            })
    }
}

#[async_trait]
impl ChannelProvider for InnerTube {
    async fn get_channel(&self, config: &ChannelConfig) -> Result<ChannelInfo, ProviderError> {
        let channel = InnerTube::get_channel(self, &config.channel_id)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })?;

        Ok(channel.into())
    }

    async fn get_channel_videos_with_locale(
        &self,
        config: &ChannelConfig,
        tab: ChannelTab,
        hl: &str,
        gl: &str,
    ) -> Result<ChannelVideos, ProviderError> {
        let videos = InnerTube::get_channel_videos_with_explicit_locale(
            self,
            &config.channel_id,
            tab,
            hl,
            gl,
        )
        .await
        .map_err(|e| ProviderError::Api {
            message: e.to_string(),
        })?;

        Ok(videos.into())
    }

    async fn get_channel_videos_continuation_with_locale(
        &self,
        _config: &ChannelConfig,
        token: &str,
        hl: &str,
        gl: &str,
    ) -> Result<ChannelVideos, ProviderError> {
        let videos = InnerTube::get_channel_videos_continuation_with_locale(self, token, hl, gl)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })?;

        Ok(videos.into())
    }
}
