//! Audio spectrum visualizer using wgpu shaders.

use crate::video::SPECTRUM_BANDS;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::{Element, Length, Rectangle, Size};
use iced_wgpu::primitive::{Pipeline, Primitive, Renderer as PrimitiveRenderer};
use iced_wgpu::wgpu;
use std::sync::{Arc, Mutex};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    rect: [f32; 4],
    time: [f32; 4], // time in [0], rest unused (for alignment)
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SpectrumData {
    // 64 bands packed as 16 vec4s
    bands: [[f32; 4]; 16],
}

pub struct VisualizerPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    spectrum_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Pipeline for VisualizerPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("visualizer.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("visualizer bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visualizer pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("visualizer pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
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
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let spectrum_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer spectrum buffer"),
            size: std::mem::size_of::<SpectrumData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("visualizer bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: spectrum_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            uniform_buffer,
            spectrum_buffer,
            bind_group,
        }
    }
}

/// Primitive for rendering the audio visualizer.
#[derive(Clone)]
pub struct VisualizerPrimitive {
    spectrum: Arc<Mutex<[f32; SPECTRUM_BANDS]>>,
    time: f32,
}

impl std::fmt::Debug for VisualizerPrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisualizerPrimitive")
            .field("time", &self.time)
            .finish()
    }
}

impl VisualizerPrimitive {
    pub fn new(spectrum: Arc<Mutex<[f32; SPECTRUM_BANDS]>>, time: f32) -> Self {
        Self { spectrum, time }
    }
}

impl Primitive for VisualizerPrimitive {
    type Pipeline = VisualizerPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &iced_wgpu::graphics::Viewport,
    ) {
        // Transform bounds to clip space
        let vp_width = viewport.logical_size().width;
        let vp_height = viewport.logical_size().height;

        let x1 = (bounds.x / vp_width) * 2.0 - 1.0;
        let y1 = 1.0 - (bounds.y / vp_height) * 2.0;
        let x2 = ((bounds.x + bounds.width) / vp_width) * 2.0 - 1.0;
        let y2 = 1.0 - ((bounds.y + bounds.height) / vp_height) * 2.0;

        let uniforms = Uniforms {
            rect: [x1, y1, x2, y2],
            time: [self.time, 0.0, 0.0, 0.0],
        };

        queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Pack spectrum data into vec4 array
        let spectrum_data = self.spectrum.lock().unwrap();
        let mut bands = [[0.0f32; 4]; 16];
        for (i, chunk) in spectrum_data.chunks(4).enumerate() {
            if i < 16 {
                for (j, &val) in chunk.iter().enumerate() {
                    bands[i][j] = val;
                }
            }
        }

        let spectrum = SpectrumData { bands };
        queue.write_buffer(&pipeline.spectrum_buffer, 0, bytemuck::bytes_of(&spectrum));
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("visualizer render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &pipeline.bind_group, &[]);
        pass.set_scissor_rect(
            clip_bounds.x,
            clip_bounds.y,
            clip_bounds.width,
            clip_bounds.height,
        );
        pass.draw(0..6, 0..1);
    }
}

/// Audio visualizer widget.
pub struct Visualizer<'a> {
    spectrum: Arc<Mutex<[f32; SPECTRUM_BANDS]>>,
    width: Length,
    height: Length,
    time: f32,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Visualizer<'a> {
    pub fn new(spectrum: Arc<Mutex<[f32; SPECTRUM_BANDS]>>) -> Self {
        Self {
            spectrum,
            width: Length::Fill,
            height: Length::Fill,
            time: 0.0,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn time(mut self, time: f32) -> Self {
        self.time = time;
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Visualizer<'_>
where
    Renderer: PrimitiveRenderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        renderer.draw_primitive(
            layout.bounds(),
            VisualizerPrimitive::new(Arc::clone(&self.spectrum), self.time),
        );
    }
}

impl<'a, Message, Theme, Renderer> From<Visualizer<'a>> for Element<'a, Message, Theme, Renderer>
where
    Renderer: PrimitiveRenderer,
{
    fn from(visualizer: Visualizer<'a>) -> Self {
        Self::new(visualizer)
    }
}
