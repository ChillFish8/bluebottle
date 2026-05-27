use std::marker::PhantomData;

use iced::time::Instant;
use iced::widget::shader::{self, Action, Viewport};
use iced::{Event, Rectangle, mouse, wgpu, window};

use super::gpu::{BLUR_FORMAT, SOURCE_FORMAT, blur_pass};
use super::{Backdrop, CompositeKind, Look};
use crate::style;

/// The program driving a frosted background; `K` selects the pipeline instance.
pub struct CompositeProgram<K> {
    image: Option<Backdrop>,
    look: Look,
    kind: PhantomData<K>,
}

impl<K> CompositeProgram<K> {
    pub fn new(image: Option<Backdrop>, look: Look) -> Self {
        Self {
            image,
            look,
            kind: PhantomData,
        }
    }
}

/// The animation clock, persisted per widget in the tree.
#[derive(Default)]
pub struct State {
    /// The image currently settled on screen.
    shown: Option<Backdrop>,
    /// The image being faded out during a transition.
    from: Option<Backdrop>,
    /// When the live transition began; `None` once settled.
    started: Option<Instant>,
}

impl<Message, K: CompositeKind> shader::Program<Message> for CompositeProgram<K> {
    type State = State;
    type Primitive = CompositePrimitive<K>;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return None;
        };

        // Start a transition when the image changes. The first paint goes from
        // `None`, fading the image in over the glow.
        if !same(&state.shown, &self.image) {
            state.from = state.shown.take();
            state.shown = self.image.clone();
            state.started = Some(*now);
        }

        let started = state.started?;
        if progress(started, *now) >= 1.0 {
            state.started = None;
            state.from = None;
            None
        } else {
            Some(Action::request_redraw())
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        _cursor: mouse::Cursor,
        _bounds: Rectangle,
    ) -> Self::Primitive {
        let transition = state
            .started
            .map(|started| progress(started, Instant::now()).min(1.0));

        CompositePrimitive {
            to: self.image.clone(),
            from: state.from.clone(),
            transition,
            look: self.look,
            kind: PhantomData,
        }
    }
}

/// The primitive produced each draw; carries the live transition.
#[derive(Debug)]
pub struct CompositePrimitive<K: CompositeKind> {
    /// The incoming image, or `None` for the glow.
    to: Option<Backdrop>,
    /// The outgoing image, only meaningful while `transition` is `Some`.
    from: Option<Backdrop>,
    /// Crossfade factor in `[0, 1]`, or `None` once settled.
    transition: Option<f32>,
    look: Look,
    kind: PhantomData<K>,
}

impl<K: CompositeKind> shader::Primitive for CompositePrimitive<K> {
    type Pipeline = CompositePipeline<K>;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        _viewport: &Viewport,
    ) {
        pipeline.prepare(device, queue, self, bounds);
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

/// True when both sides reference the same image (or both are absent).
fn same(a: &Option<Backdrop>, b: &Option<Backdrop>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.key() == b.key(),
        (None, None) => true,
        _ => false,
    }
}

/// Elapsed transition time as a fraction of the crossfade duration.
fn progress(started: Instant, now: Instant) -> f32 {
    now.duration_since(started).as_secs_f32() / style::CROSSFADE.as_secs_f32()
}

/// The GPU resources for a background, with one render pass per fading image.
pub struct CompositePipeline<K> {
    shared: Shared,
    to: Pass,
    from: Pass,
    kind: PhantomData<K>,
}

/// Resources shared by both passes and both blur stages.
struct Shared {
    composite_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    blur_uniform_h: wgpu::Buffer,
    blur_uniform_v: wgpu::Buffer,
    /// 1x1 placeholder bound when a pass has no image.
    dummy_view: wgpu::TextureView,
}

/// One composite pass: its uniform, its cached image, and its bind group.
struct Pass {
    uniform: wgpu::Buffer,
    image: Option<ImageState>,
    bind: Option<wgpu::BindGroup>,
}

/// The wgpu handles threaded together through preparation.
struct Gpu<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

/// The textures and bind groups for one uploaded image.
struct ImageState {
    /// Identity of the source image, to detect when it changes.
    key: usize,
    width: u32,
    height: u32,
    /// Blur radius baked into `blurred`, so a repeat at the same radius is free.
    blurred_radius: f32,
    blur_h_bind: wgpu::BindGroup,
    blur_v_bind: wgpu::BindGroup,
    intermediate_view: wgpu::TextureView,
    blurred_view: wgpu::TextureView,
}

impl<K: CompositeKind> shader::Pipeline for CompositePipeline<K> {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let label = |suffix: &str| format!("{} {suffix}", K::LABEL);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&label("shader")),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shader_common.wgsl"),
                    include_str!("background.wgsl"),
                )
                .into(),
            ),
        });

        let bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&label("bind layout")),
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
                label: Some(&label("pipeline layout")),
                bind_group_layouts: &[&bind_layout],
                push_constant_ranges: &[],
            });

        let make_pipeline = |label: &str,
                             entry: &str,
                             target: wgpu::TextureFormat,
                             blend: wgpu::BlendState| {
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
                        blend: Some(blend),
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

        // The composite pass alpha-blends so the incoming image can fade over the
        // outgoing one. At full opacity this matches a plain overwrite.
        let composite_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let composite_pipeline =
            make_pipeline(&label("composite"), "fs_composite", format, composite_blend);
        let blur_pipeline = make_pipeline(
            &label("blur"),
            "fs_blur",
            BLUR_FORMAT,
            wgpu::BlendState::REPLACE,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&label("sampler")),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let composite_uniform = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: COMPOSITE_UNIFORM_SIZE,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let blur_uniform = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: BLUR_UNIFORM_SIZE,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let dummy = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&label("dummy image")),
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
            shared: Shared {
                composite_pipeline,
                blur_pipeline,
                bind_layout,
                sampler,
                blur_uniform_h: blur_uniform(&label("blur uniform (h)")),
                blur_uniform_v: blur_uniform(&label("blur uniform (v)")),
                dummy_view: dummy.create_view(&wgpu::TextureViewDescriptor::default()),
            },
            to: Pass::new(composite_uniform(&label("composite uniform (to)"))),
            from: Pass::new(composite_uniform(&label("composite uniform (from)"))),
            kind: PhantomData,
        }
    }
}

impl<K: CompositeKind> CompositePipeline<K> {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        primitive: &CompositePrimitive<K>,
        bounds: &Rectangle,
    ) {
        let gpu = Gpu { device, queue };
        let opacity = primitive.transition.unwrap_or(1.0);
        self.to.prepare(
            &self.shared,
            &gpu,
            primitive.to.as_ref(),
            primitive.look,
            opacity,
            bounds,
        );

        if primitive.transition.is_some() {
            self.from.prepare(
                &self.shared,
                &gpu,
                primitive.from.as_ref(),
                primitive.look,
                1.0,
                bounds,
            );
        } else {
            // Settled: drop the outgoing image so it stops holding GPU memory.
            self.from.image = None;
            self.from.bind = None;
        }
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        // The outgoing image goes down first, then the incoming one fades over it.
        if let Some(bind) = &self.from.bind {
            self.shared.draw_pass(encoder, target, clip_bounds, bind);
        }
        if let Some(bind) = &self.to.bind {
            self.shared.draw_pass(encoder, target, clip_bounds, bind);
        }
    }
}

impl Pass {
    fn new(uniform: wgpu::Buffer) -> Self {
        Self {
            uniform,
            image: None,
            bind: None,
        }
    }

    fn prepare(
        &mut self,
        shared: &Shared,
        gpu: &Gpu,
        source: Option<&Backdrop>,
        look: Look,
        opacity: f32,
        bounds: &Rectangle,
    ) {
        let (mode, source_size) = match source {
            Some(image) => {
                shared.ensure_image(gpu, &mut self.image, image, look.blur);
                (1.0, [image.width() as f32, image.height() as f32])
            },
            None => {
                self.image = None;
                (0.0, [bounds.width, bounds.height])
            },
        };

        let uniform = composite_uniform(look, mode, opacity, source_size, bounds);
        gpu.queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&uniform));

        let view = match &self.image {
            Some(state) => &state.blurred_view,
            None => &shared.dummy_view,
        };
        self.bind = Some(shared.composite_bind(gpu.device, &self.uniform, view));
    }
}

impl Shared {
    /// Uploads `image` if it changed, then re-runs the blur if `blur` changed.
    fn ensure_image(
        &self,
        gpu: &Gpu,
        slot: &mut Option<ImageState>,
        image: &Backdrop,
        blur: f32,
    ) {
        let key = image.key();
        if slot.as_ref().is_none_or(|state| state.key != key) {
            *slot = Some(self.upload(gpu, image, key));
        }

        let Some(state) = slot.as_mut() else {
            return;
        };
        if state.blurred_radius == blur {
            return;
        }
        state.blurred_radius = blur;

        let texel = [1.0 / state.width as f32, 1.0 / state.height as f32];
        gpu.queue.write_buffer(
            &self.blur_uniform_h,
            0,
            bytemuck::cast_slice(&[texel[0], texel[1], 1.0, 0.0, blur, 0.0, 0.0, 0.0]),
        );
        gpu.queue.write_buffer(
            &self.blur_uniform_v,
            0,
            bytemuck::cast_slice(&[texel[0], texel[1], 0.0, 1.0, blur, 0.0, 0.0, 0.0]),
        );

        let mut encoder =
            gpu.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("composite blur"),
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
        gpu.queue.submit(Some(encoder.finish()));
    }

    /// Uploads `image` to a fresh source texture and builds the blur targets.
    fn upload(&self, gpu: &Gpu, image: &Backdrop, key: usize) -> ImageState {
        let extent = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };
        let source = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("composite image source"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SOURCE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &source,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image.rgba(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * 4),
                rows_per_image: Some(image.height()),
            },
            extent,
        );

        let blur_target = |label| {
            gpu.device
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
        let intermediate_view = blur_target("composite blur intermediate");
        let blurred_view = blur_target("composite blur result");

        ImageState {
            key,
            width: image.width(),
            height: image.height(),
            blurred_radius: f32::NAN,
            blur_h_bind: self.blur_bind(gpu.device, &self.blur_uniform_h, &source_view),
            blur_v_bind: self.blur_bind(
                gpu.device,
                &self.blur_uniform_v,
                &intermediate_view,
            ),
            intermediate_view,
            blurred_view,
        }
    }

    /// Builds a bind group of a uniform, an input texture, and the sampler.
    fn blur_bind(
        &self,
        device: &wgpu::Device,
        uniform: &wgpu::Buffer,
        input: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite blur bind"),
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

    fn composite_bind(
        &self,
        device: &wgpu::Device,
        uniform: &wgpu::Buffer,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite bind"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn draw_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
        bind: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("composite pass"),
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

/// Packs the `Composite` uniform (see `background.wgsl`) for a `look` drawn over
/// `bounds`, where `mode` is `1.0` for an image or `0.0` for the glow, `opacity`
/// is the pass alpha, and `source_size` is the image size (or the bounds).
fn composite_uniform(
    look: Look,
    mode: f32,
    opacity: f32,
    source_size: [f32; 2],
    bounds: &Rectangle,
) -> [f32; COMPOSITE_UNIFORM_LEN] {
    let base = look.base.into_linear();
    let glow = look.glow.into_linear();
    [
        bounds.width,
        bounds.height,
        source_size[0],
        source_size[1],
        base[0],
        base[1],
        base[2],
        base[3],
        glow[0],
        glow[1],
        glow[2],
        glow[3],
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
        look.glow_strength,
        opacity,
        0.0,
        0.0,
    ]
}

/// Length of the packed `Composite` uniform; see `background.wgsl`.
const COMPOSITE_UNIFORM_LEN: usize = 28;
/// Byte size of the `Composite` uniform.
const COMPOSITE_UNIFORM_SIZE: u64 = (COMPOSITE_UNIFORM_LEN * 4) as u64;
/// 8 `f32`s; see the `Blur` struct in `shader_common.wgsl`.
const BLUR_UNIFORM_SIZE: u64 = 8 * 4;
