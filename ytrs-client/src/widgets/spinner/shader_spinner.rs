//! Shader-based circular spinner widget
//!
//! Uses a WGSL shader to render an animated spinner, avoiding canvas caching issues.

use iced::wgpu;
use iced::widget::shader::{self, Pipeline, Viewport};
use iced::{Color, Element, Length, Rectangle, Renderer, Theme};
use std::fmt::Debug;
use std::time::Instant;

/// A circular spinner rendered using a custom shader
#[derive(Clone)]
pub struct ShaderSpinner {
    track_color: Color,
    bar_color: Color,
}

impl<Message> shader::Program<Message> for ShaderSpinner {
    type State = SpinnerState;
    type Primitive = SpinnerPrimitive;

    fn draw(
        &self,
        state: &Self::State,
        _cursor: iced::mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        let elapsed = state.start_time.elapsed().as_secs_f32();

        SpinnerPrimitive {
            bounds,
            time: elapsed,
            track_color: self.track_color,
            bar_color: self.bar_color,
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
pub struct SpinnerState {
    start_time: Instant,
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

#[derive(Debug)]
pub struct SpinnerPrimitive {
    bounds: Rectangle,
    time: f32,
    track_color: Color,
    bar_color: Color,
}

impl shader::Primitive for SpinnerPrimitive {
    type Pipeline = SpinnerPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        pipeline.update(queue, self);
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
    track_color: [f32; 4],
    bar_color: [f32; 4],
}

pub struct SpinnerPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Debug for SpinnerPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpinnerPipeline").finish_non_exhaustive()
    }
}

impl Pipeline for SpinnerPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Spinner Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("spinner.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spinner Uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Spinner Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Spinner Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Spinner Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Spinner Pipeline"),
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
            bind_group,
        }
    }
}

impl SpinnerPipeline {
    fn update(&self, queue: &wgpu::Queue, primitive: &SpinnerPrimitive) {
        let uniforms = Uniforms {
            size: [primitive.bounds.width, primitive.bounds.height],
            time: primitive.time,
            _padding: 0.0,
            track_color: [
                primitive.track_color.r,
                primitive.track_color.g,
                primitive.track_color.b,
                primitive.track_color.a,
            ],
            bar_color: [
                primitive.bar_color.r,
                primitive.bar_color.g,
                primitive.bar_color.b,
                primitive.bar_color.a,
            ],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    fn draw(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

/// Create a shader spinner element with theme colors
pub fn shader_spinner<Message: 'static>(
    size: f32,
    theme: &Theme,
) -> Element<'static, Message, Theme, Renderer> {
    let palette = theme.extended_palette();

    let spinner = ShaderSpinner {
        track_color: Color {
            a: 0.3,
            ..palette.background.weak.color
        },
        bar_color: palette.primary.base.color,
    };

    shader::Shader::new(spinner)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}
