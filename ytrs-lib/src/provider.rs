//! VideoProvider and ChannelProvider trait implementations for InnerTube

use async_trait::async_trait;
use common::{
    ChannelConfig, ChannelInfo, ChannelProvider, ChannelTab, ChannelVideos, ProviderError,
    SearchResults, Video, VideoMetadata, VideoProvider,
};

use crate::client::InnerTube;
use crate::models::PLATFORM_NAME;

#[async_trait]
impl VideoProvider for InnerTube {
    fn platform_name(&self) -> &'static str {
        PLATFORM_NAME
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

    async fn search_next_page(
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

    async fn get_video_metadata(&self, video: &Video) -> Result<VideoMetadata, ProviderError> {
        let metadata = InnerTube::get_video_metadata(self, &video.id)
            .await
            .map_err(|e| ProviderError::Api {
                message: e.to_string(),
            })?;

        Ok(VideoMetadata {
            description: metadata.description,
            channel_name: metadata.channel_name,
            channel_id: metadata.channel_id,
            channel_avatar_url: metadata.channel_avatar_url,
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
