use std::sync::Arc;

use bluebottle_ui::color;
use iced::widget::shader::{self, Viewport};
use iced::{Rectangle, wgpu};

use super::{BackgroundLook, BackgroundSource, HIGHLIGHT};
use crate::backdrop::BackdropImage;

/// sRGB format for the uploaded source image, so sampling decodes it to linear.
const SOURCE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// Linear 16-bit-float format for the blur intermediates. The extra precision
/// keeps the smooth blur from banding when it is later up-scaled.
const BLUR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The primitive produced each draw; carries the live background parameters.
#[derive(Debug)]
pub struct BackgroundPrimitive {
    pub source: Arc<BackgroundSource>,
    pub look: BackgroundLook,
}

impl shader::Primitive for BackgroundPrimitive {
    type Pipeline = BackgroundPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        pipeline.prepare(device, queue, &self.source, self.look, bounds);
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        pipeline.render(encoder, target, clip_bounds);
    }
}

/// GPU resources shared by every [`BackgroundPrimitive`].
pub struct BackgroundPipeline {
    composite_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    composite_uniform: wgpu::Buffer,
    blur_uniform_h: wgpu::Buffer,
    blur_uniform_v: wgpu::Buffer,
    /// 1×1 placeholder bound in gradient mode, where the poster is never sampled.
    dummy_view: wgpu::TextureView,
    /// Per-image GPU state, present only while a backdrop image is shown.
    image: Option<ImageState>,
    composite_bind: Option<wgpu::BindGroup>,
}

/// The textures and bind groups for the currently uploaded backdrop image.
struct ImageState {
    /// Identity of the source `Arc`, to detect when the image changes.
    key: usize,
    width: u32,
    height: u32,
    /// Blur radius baked into `blurred`, so a repeat at the same radius is free.
    blurred_radius: f32,
    /// Horizontal pass: source → `intermediate`.
    blur_h_bind: wgpu::BindGroup,
    /// Vertical pass: `intermediate` → `blurred`.
    blur_v_bind: wgpu::BindGroup,
    intermediate_view: wgpu::TextureView,
    blurred_view: wgpu::TextureView,
}

impl shader::Pipeline for BackgroundPipeline {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("background shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("background.wgsl").into()),
        });

        // A uniform + sampled texture + sampler, shared by both pipelines.
        let bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("background bind layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("background pipeline layout"),
                bind_group_layouts: &[&bind_layout],
                push_constant_ranges: &[],
            });

        let make_pipeline = |label: &str, entry: &str, target: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_fullscreen"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };

        let composite_pipeline =
            make_pipeline("background composite", "fs_composite", format);
        let blur_pipeline = make_pipeline("background blur", "fs_blur", BLUR_FORMAT);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("background sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let composite_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("background composite uniform"),
            size: COMPOSITE_UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_uniform = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: BLUR_UNIFORM_SIZE,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let dummy = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("background dummy poster"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: BLUR_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Self {
            composite_pipeline,
            blur_pipeline,
            bind_layout,
            sampler,
            composite_uniform,
            blur_uniform_h: blur_uniform("background blur uniform (h)"),
            blur_uniform_v: blur_uniform("background blur uniform (v)"),
            dummy_view: dummy.create_view(&wgpu::TextureViewDescriptor::default()),
            image: None,
            composite_bind: None,
        }
    }
}

impl BackgroundPipeline {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &BackgroundSource,
        look: BackgroundLook,
        bounds: &Rectangle,
    ) {
        let (mode, source_size) = match source {
            BackgroundSource::Image(image) => {
                self.ensure_image(device, queue, image, look.blur);
                (1.0, [image.width as f32, image.height as f32])
            },
            BackgroundSource::Gradient => {
                self.image = None;
                (0.0, [bounds.width, bounds.height])
            },
        };

        let base = color::BACKGROUND.into_linear();
        let highlight = HIGHLIGHT.into_linear();
        let uniform: [f32; 24] = [
            bounds.width,
            bounds.height,
            source_size[0],
            source_size[1],
            base[0],
            base[1],
            base[2],
            base[3],
            highlight[0],
            highlight[1],
            highlight[2],
            highlight[3],
            look.saturate,
            mode,
            look.image_opacity_start,
            look.image_opacity_end,
            look.bg_opacity_start,
            look.bg_opacity_end,
            look.image_fade,
            look.bg_start,
            look.bg_end,
            look.bg_solid,
            look.focus,
            look.zoom,
        ];
        queue.write_buffer(&self.composite_uniform, 0, as_bytes(&uniform));

        // Point the composite at the freshly blurred poster, or the placeholder.
        let poster_view = match &self.image {
            Some(state) => &state.blurred_view,
            None => &self.dummy_view,
        };
        self.composite_bind =
            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("background composite bind"),
                layout: &self.bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.composite_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(poster_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));
    }

    /// Uploads `image` if it changed, then (re)runs the blur if `blur` changed.
    fn ensure_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &Arc<BackdropImage>,
        blur: f32,
    ) {
        let key = Arc::as_ptr(image) as usize;
        if self.image.as_ref().is_none_or(|state| state.key != key) {
            self.image = Some(self.upload(device, queue, image, key));
        }

        // Disjoint field borrows: `image` is mutated while the pipeline's blur
        // resources are read.
        let Some(state) = self.image.as_mut() else {
            return;
        };
        if state.blurred_radius == blur {
            return;
        }
        state.blurred_radius = blur;

        let texel = [1.0 / state.width as f32, 1.0 / state.height as f32];
        queue.write_buffer(
            &self.blur_uniform_h,
            0,
            as_bytes(&[texel[0], texel[1], 1.0, 0.0, blur, 0.0, 0.0, 0.0]),
        );
        queue.write_buffer(
            &self.blur_uniform_v,
            0,
            as_bytes(&[texel[0], texel[1], 0.0, 1.0, blur, 0.0, 0.0, 0.0]),
        );

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("background blur"),
            });
        blur_pass(
            &mut encoder,
            &self.blur_pipeline,
            &state.intermediate_view,
            &state.blur_h_bind,
        );
        blur_pass(
            &mut encoder,
            &self.blur_pipeline,
            &state.blurred_view,
            &state.blur_v_bind,
        );
        queue.submit(Some(encoder.finish()));
    }

    /// Uploads `image` to a fresh source texture and builds the blur targets.
    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &BackdropImage,
        key: usize,
    ) -> ImageState {
        let extent = wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        };
        let source = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("background poster source"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SOURCE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &source,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width * 4),
                rows_per_image: Some(image.height),
            },
            extent,
        );

        let blur_target = |label| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: BLUR_FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let intermediate_view = blur_target("background blur intermediate");
        let blurred_view = blur_target("background blur result");

        ImageState {
            key,
            width: image.width,
            height: image.height,
            blurred_radius: f32::NAN,
            blur_h_bind: self.blur_bind(device, &self.blur_uniform_h, &source_view),
            blur_v_bind: self.blur_bind(
                device,
                &self.blur_uniform_v,
                &intermediate_view,
            ),
            intermediate_view,
            blurred_view,
        }
    }

    /// Builds a blur bind group: its uniform, the input texture, and the sampler.
    fn blur_bind(
        &self,
        device: &wgpu::Device,
        uniform: &wgpu::Buffer,
        input: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("background blur bind"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(input),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let Some(bind) = &self.composite_bind else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("background composite pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_viewport(
            clip_bounds.x as f32,
            clip_bounds.y as f32,
            clip_bounds.width as f32,
            clip_bounds.height as f32,
            0.0,
            1.0,
        );
        pass.set_scissor_rect(
            clip_bounds.x,
            clip_bounds.y,
            clip_bounds.width,
            clip_bounds.height,
        );
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// 24 `f32`s; see the `Composite` struct in `background.wgsl`.
const COMPOSITE_UNIFORM_SIZE: u64 = 24 * 4;
/// 8 `f32`s; see the `Blur` struct in `background.wgsl`.
const BLUR_UNIFORM_SIZE: u64 = 8 * 4;

/// Runs one full-screen blur pass into `target` with `bind`.
fn blur_pass(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    target: &wgpu::TextureView,
    bind: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("background blur pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind, &[]);
    pass.draw(0..3, 0..1);
}

/// Reinterprets a packed `f32` slice as bytes for `queue.write_buffer`.
fn as_bytes(values: &[f32]) -> &[u8] {
    // SAFETY: `f32` has no padding and no invalid bit patterns, so viewing a
    // contiguous slice of them as bytes is sound; the borrow ties the returned
    // slice to `values`.
    unsafe {
        std::slice::from_raw_parts(
            values.as_ptr().cast::<u8>(),
            std::mem::size_of_val(values),
        )
    }
}
