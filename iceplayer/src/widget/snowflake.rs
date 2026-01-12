//! Shader-based snowflake spinner widget
//!
//! Uses a WGSL shader to render an animated rotating snowflake logo.

use iced::wgpu;
use iced::widget::shader::{self, Pipeline, Viewport};
use iced::{Element, Length, Rectangle, Renderer, Theme};
use std::fmt::Debug;
use std::sync::OnceLock;
use std::time::Instant;

/// Embedded snowflake SVG
const SNOWFLAKE_SVG: &[u8] = include_bytes!("../../assets/snowflake.svg");

/// Cached rasterized logo (RGBA pixels)
static LOGO_PIXELS: OnceLock<(Vec<u8>, u32, u32)> = OnceLock::new();

/// Rasterize the SVG to RGBA pixels
fn rasterize_svg() -> (Vec<u8>, u32, u32) {
    let size = 128u32;

    let tree = resvg::usvg::Tree::from_data(SNOWFLAKE_SVG, &resvg::usvg::Options::default())
        .expect("Failed to parse SVG");

    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("Failed to create pixmap");

    // Scale to fit
    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    (pixmap.take(), size, size)
}

fn get_logo_pixels() -> &'static (Vec<u8>, u32, u32) {
    LOGO_PIXELS.get_or_init(rasterize_svg)
}

/// A snowflake spinner rendered using a custom shader
#[derive(Clone)]
pub struct SnowflakeSpinner {
    color: [f32; 4],
}

impl<Message> shader::Program<Message> for SnowflakeSpinner {
    type State = SnowflakeState;
    type Primitive = SnowflakePrimitive;

    fn draw(
        &self,
        state: &Self::State,
        _cursor: iced::mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        let elapsed = state.start_time.elapsed().as_secs_f32();

        SnowflakePrimitive {
            bounds,
            time: elapsed,
            color: self.color,
        }
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &iced::Event,
        _bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<iced::widget::Action<Message>> {
        // Request continuous redraws for animation
        Some(iced::widget::Action::request_redraw())
    }
}

#[derive(Debug)]
pub struct SnowflakeState {
    start_time: Instant,
}

impl Default for SnowflakeState {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

#[derive(Debug)]
pub struct SnowflakePrimitive {
    bounds: Rectangle,
    time: f32,
    color: [f32; 4],
}

impl shader::Primitive for SnowflakePrimitive {
    type Pipeline = SnowflakePipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        pipeline.prepare(device, queue, self);
    }

    fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
        pipeline.draw(render_pass);
        true
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    size: [f32; 2],
    time: f32,
    _padding: f32,
    color: [f32; 4],
}

pub struct SnowflakePipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    texture: Option<wgpu::Texture>,
}

impl Debug for SnowflakePipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnowflakePipeline").finish_non_exhaustive()
    }
}

impl Pipeline for SnowflakePipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Snowflake Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("snowflake.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Snowflake Uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Snowflake Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Snowflake Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Snowflake Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group_layout,
            bind_group: None,
            texture: None,
        }
    }
}

impl SnowflakePipeline {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        primitive: &SnowflakePrimitive,
    ) {
        // Create texture if not exists
        if self.texture.is_none() {
            let (pixels, width, height) = get_logo_pixels();

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Snowflake Texture"),
                size: wgpu::Extent3d {
                    width: *width,
                    height: *height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * *width),
                    rows_per_image: Some(*height),
                },
                wgpu::Extent3d {
                    width: *width,
                    height: *height,
                    depth_or_array_layers: 1,
                },
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Snowflake Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Snowflake Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

            self.texture = Some(texture);
            self.bind_group = Some(bind_group);
        }

        // Update uniforms
        let uniforms = Uniforms {
            size: [primitive.bounds.width, primitive.bounds.height],
            time: primitive.time,
            _padding: 0.0,
            color: primitive.color,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        if let Some(bind_group) = &self.bind_group {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
    }
}

/// Create a snowflake spinner element
pub fn snowflake_spinner<Message: 'static>(
    size: f32,
    theme: &Theme,
) -> Element<'static, Message, Theme, Renderer> {
    let palette = theme.palette();
    let color = [palette.primary.r, palette.primary.g, palette.primary.b, 1.0];

    shader::Shader::new(SnowflakeSpinner { color })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}
