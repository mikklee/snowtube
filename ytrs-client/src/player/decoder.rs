//! Video decoder using vk-video for hardware H.264 decoding.

use iced::wgpu;
use std::sync::Arc;

/// Video decoder wrapper around vk-video
pub struct VideoDecoder {
    instance: Arc<vk_video::VulkanInstance>,
    device: Option<Arc<vk_video::VulkanDevice>>,
}

impl VideoDecoder {
    /// Create a new video decoder
    pub fn new() -> Result<Self, String> {
        let instance = vk_video::VulkanInstance::new()
            .map_err(|e| format!("Failed to create Vulkan instance: {:?}", e))?;

        Ok(Self {
            instance,
            device: None,
        })
    }

    /// Initialize the decoder with a wgpu surface
    pub fn init_with_surface(&mut self, surface: &wgpu::Surface<'static>) -> Result<(), String> {
        let device = self
            .instance
            .create_device(
                wgpu::Features::empty(),
                wgpu::Limits::default(),
                Some(surface),
            )
            .map_err(|e| format!("Failed to create Vulkan device: {:?}", e))?;

        self.device = Some(device);
        Ok(())
    }

    /// Create a textures decoder for decoding frames
    pub fn create_textures_decoder(&self) -> Result<TexturesDecoder, String> {
        let device = self.device.as_ref().ok_or("Device not initialized")?;

        let decoder = device
            .create_wgpu_textures_decoder()
            .map_err(|e| format!("Failed to create decoder: {:?}", e))?;

        Ok(TexturesDecoder { decoder })
    }

    /// Get the wgpu device (for creating textures, etc.)
    pub fn wgpu_device(&self) -> Option<&wgpu::Device> {
        self.device.as_ref().map(|d| d.wgpu_device())
    }

    /// Get the wgpu queue
    pub fn wgpu_queue(&self) -> Option<&wgpu::Queue> {
        self.device.as_ref().map(|d| d.wgpu_queue())
    }
}

/// Wrapper for the textures decoder with a fixed lifetime
pub struct TexturesDecoder<'a> {
    decoder: vk_video::WgpuTexturesDecoder<'a>,
}

impl<'a> TexturesDecoder<'a> {
    /// Decode H.264 NAL units and return the decoded frame textures
    pub fn decode(&mut self, h264_data: &[u8]) -> Result<Vec<DecodedFrame>, String> {
        let frames = self
            .decoder
            .decode(h264_data, None)
            .map_err(|e| format!("Decode error: {:?}", e))?;

        Ok(frames
            .into_iter()
            .map(|f| DecodedFrame {
                texture: f.texture,
                width: f.display_rect.width,
                height: f.display_rect.height,
            })
            .collect())
    }
}

/// A decoded video frame
pub struct DecodedFrame {
    /// The GPU texture containing the frame
    pub texture: wgpu::Texture,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
}
