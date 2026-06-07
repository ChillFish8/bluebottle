//! The shader runtime for [`BlurredImage`](super::BlurredImage).
//!
//! The pipeline owns one persistent cache entry per `(image, radius)` pair,
//! holding the source texture and a separable Gaussian blur baked into an
//! offscreen linear target. Keying on the radius too keeps two widgets that
//! share a backdrop at different blur radii from thrashing each other's
//! blurred texture. Each widget instance writes a small composite uniform into
//! a dynamically-offset slot, the composite pass mixes the sharp source and
//! the blurred target per pixel by rounded-rect coverage over the declared
//! regions, and iced shares one pipeline across every widget of this type so
//! the prepare and render paths cooperate via cursors the way [`skeleton`]
//! does.
//!
//! [`skeleton`]: crate::widget::skeleton

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use iced::widget::shader::{self, Action, Viewport};
use iced::{Event, Rectangle, Size, mouse, wgpu};

use crate::widget::blur::Backdrop;
use crate::widget::blur::gpu::{
    BLUR_FORMAT,
    SOURCE_FORMAT,
    blur_pass,
    blur_uniform_buffer,
    pack_blur_uniform,
    sampler as blur_sampler,
};
use crate::widget::blur::pipeline::{bind_layout as blur_bind_layout, blur_pipeline};

/// Cap on simultaneous blur regions in one widget. Substituted into the WGSL
/// at pipeline-creation time so the shader's array size and loop bound track
/// this constant automatically.
pub const MAX_REGIONS: usize = 16;

/// Cap on concurrent widget instances that can render in one frame. The
/// composite buffer is sized for this at startup. The widget gracefully drops
/// extras rather than panicking.
const MAX_INSTANCES: usize = 64;

/// Frames a cached image is kept after its last use before its GPU resources
/// are dropped. One second at 60 Hz is enough to ride out brief swaps without
/// re-uploading on the next frame.
const STALE_FRAMES: u64 = 60;

/// The packed `Composite` uniform. See `composite.wgsl`.
///
/// WGSL aligns vec4 to a 16-byte boundary, so the shader implicitly pads
/// between `progress_height` (ends at offset 40) and `progress_color`
/// (forced to offset 48). `_pad_progress` mirrors that padding so wgpu's
/// min_binding_size matches the shader's view. `region_radii` packs four
/// radii per vec4 to keep all of [`MAX_REGIONS`] in one uniform slot.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct CompositeUniform {
    target_size: [f32; 2],
    widget_origin_px: [f32; 2],
    widget_size_px: [f32; 2],
    region_count: u32,
    corner_radius: f32,
    progress_fill: f32,
    progress_height: f32,
    _pad_progress: [f32; 2],
    progress_color: [f32; 4],
    progress_track: [f32; 4],
    regions: [[f32; 4]; MAX_REGIONS],
    region_radii: [[f32; 4]; MAX_REGIONS / 4],
}

const _: () = assert!(
    MAX_REGIONS.is_multiple_of(4),
    "MAX_REGIONS must be a multiple of 4 so region_radii packs evenly into vec4 slots",
);

const COMPOSITE_UNIFORM_SIZE: u64 = size_of::<CompositeUniform>() as u64;

/// Cache key for the per-image GPU state. Keying on the radius bits as well as
/// the backdrop identity is what stops two widgets that share a backdrop at
/// different blur radii from overwriting each other's blurred texture.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct CacheKey {
    image: usize,
    radius_bits: u32,
}

impl CacheKey {
    fn new(backdrop: &Backdrop, radius: f32) -> Self {
        Self {
            image: backdrop.key(),
            radius_bits: radius.to_bits(),
        }
    }
}

/// The program behind every blurred image, one type so they share a pipeline.
pub struct BlurredImageProgram {
    backdrop: Backdrop,
    blur_radius: f32,
    corner_radius: f32,
    regions: RegionSource,
    progress: Option<ProgressStrip>,
}

/// Optional progress bar painted by the composite shader along the bottom
/// of the widget. Living inside the shader means the strip rounds off with
/// the same outer SDF that masks the image, instead of sitting flat on top
/// of the rounded corners.
#[derive(Clone, Copy, Debug)]
pub struct ProgressStrip {
    pub fill: f32,
    pub height: f32,
    pub accent: iced::Color,
    pub track: iced::Color,
}

impl ProgressStrip {
    pub(crate) const OFF: ProgressStrip = ProgressStrip {
        fill: 0.0,
        height: 0.0,
        accent: iced::Color::TRANSPARENT,
        track: iced::Color::TRANSPARENT,
    };
}

/// One frosted region painted by the composite shader. Each region
/// carries its own corner radius so a card pill and a panel can sit on
/// the same surface with different rounding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlurRegion {
    pub bounds: Rectangle,
    pub corner_radius: f32,
}

impl BlurRegion {
    /// A region rounded to its rect's smallest half-extent. Square
    /// bounds become circles and rectangles become round-ended pills.
    pub fn pill(bounds: Rectangle) -> Self {
        Self {
            bounds,
            corner_radius: f32::INFINITY,
        }
    }

    /// A region with a caller-chosen corner radius.
    pub fn rounded(bounds: Rectangle, corner_radius: f32) -> Self {
        Self {
            bounds,
            corner_radius,
        }
    }
}

/// How the program resolves blur regions for a given laid-out widget size.
/// `Static` regions are shared by `Arc` so cloning the program per frame stays
/// cheap; `Dynamic` callers compute regions from the bounds so the API works
/// for `Length::Fill` widgets too.
#[derive(Clone)]
pub enum RegionSource {
    Static(Arc<[BlurRegion]>),
    Dynamic(Arc<dyn Fn(Size) -> Vec<BlurRegion> + Send + Sync>),
}

impl RegionSource {
    fn resolve(&self, bounds: Size) -> Arc<[BlurRegion]> {
        match self {
            RegionSource::Static(regions) => Arc::clone(regions),
            RegionSource::Dynamic(f) => Arc::from(f(bounds)),
        }
    }
}

impl BlurredImageProgram {
    pub fn new(
        backdrop: Backdrop,
        blur_radius: f32,
        corner_radius: f32,
        regions: RegionSource,
        progress: Option<ProgressStrip>,
    ) -> Self {
        Self {
            backdrop,
            blur_radius,
            corner_radius,
            regions,
            progress,
        }
    }
}

impl<Message> shader::Program<Message> for BlurredImageProgram {
    type State = ();
    type Primitive = BlurredImagePrimitive;

    fn update(
        &self,
        _state: &mut Self::State,
        _event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        BlurredImagePrimitive {
            backdrop: self.backdrop.clone(),
            blur_radius: self.blur_radius,
            corner_radius: self.corner_radius,
            regions: self.regions.resolve(bounds.size()),
            progress: self.progress,
        }
    }
}

/// One blurred image's per-frame snapshot.
#[derive(Debug)]
pub struct BlurredImagePrimitive {
    backdrop: Backdrop,
    blur_radius: f32,
    corner_radius: f32,
    regions: Arc<[BlurRegion]>,
    progress: Option<ProgressStrip>,
}

impl shader::Primitive for BlurredImagePrimitive {
    type Pipeline = BlurredImagePipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        pipeline.prepare(device, queue, self, bounds, viewport.scale_factor());
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

/// The shared GPU resources for all blurred images.
pub struct BlurredImagePipeline {
    blur_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    blur_bind_layout: wgpu::BindGroupLayout,
    composite_bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    blur_uniform_h: wgpu::Buffer,
    blur_uniform_v: wgpu::Buffer,

    composite_buffer: wgpu::Buffer,
    composite_stride: u64,
    /// What is currently uploaded at each slot, so we can skip the upload when
    /// a widget's uniform hasn't changed frame-to-frame.
    slot_uniforms: Vec<Option<CompositeUniform>>,

    images: HashMap<CacheKey, ImageState>,
    instances: Vec<CacheKey>,
    prepare_cursor: usize,
    draw_cursor: AtomicUsize,
    frame: u64,
}

/// Cached GPU resources for one `(image, radius)` pair. The blur passes run
/// exactly once on insert, since changing the radius now produces a fresh
/// cache entry instead of mutating an existing one.
struct ImageState {
    blur_h_bind: wgpu::BindGroup,
    blur_v_bind: wgpu::BindGroup,
    intermediate_view: wgpu::TextureView,
    blurred_view: wgpu::TextureView,
    composite_bind: wgpu::BindGroup,
    last_used_frame: u64,
}

impl shader::Pipeline for BlurredImagePipeline {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let blur_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blurred image blur shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../blur/shader.wgsl").into()),
        });
        // Substitute MAX_REGIONS and its packed-radius companion so the
        // WGSL array sizes track the Rust constant. The const assert
        // above keeps `MAX_REGIONS` divisible by 4.
        let composite_source = include_str!("composite.wgsl")
            .replace("@MAX_REGIONS_DIV_4@", &(MAX_REGIONS / 4).to_string())
            .replace("@MAX_REGIONS@", &MAX_REGIONS.to_string());
        let composite_module =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("blurred image composite shader"),
                source: wgpu::ShaderSource::Wgsl(composite_source.into()),
            });

        let blur_bind_layout =
            blur_bind_layout(device, "blurred image blur bind layout");
        let blur_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blurred image blur pipeline layout"),
                bind_group_layouts: &[&blur_bind_layout],
                push_constant_ranges: &[],
            });
        let blur_pipeline = blur_pipeline(
            device,
            &blur_pipeline_layout,
            &blur_module,
            "blurred image blur pipeline",
        );

        let composite_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("blurred image composite bind layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: wgpu::BufferSize::new(
                                COMPOSITE_UNIFORM_SIZE,
                            ),
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
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("blurred image composite pipeline layout"),
                bind_group_layouts: &[&composite_bind_layout],
                push_constant_ranges: &[],
            });

        // Standard premultiplied-alpha blend so the composite output drops
        // cleanly into iced's render layer.
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
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blurred image composite pipeline"),
                layout: Some(&composite_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &composite_module,
                    entry_point: Some("vs_fullscreen"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &composite_module,
                    entry_point: Some("fs_composite"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(composite_blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let sampler = blur_sampler(device, "blurred image sampler");
        let blur_uniform_h =
            blur_uniform_buffer(device, "blurred image blur uniform (h)");
        let blur_uniform_v =
            blur_uniform_buffer(device, "blurred image blur uniform (v)");

        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let composite_stride = COMPOSITE_UNIFORM_SIZE.div_ceil(alignment) * alignment;
        let composite_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blurred image composite uniforms"),
            size: composite_stride * MAX_INSTANCES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            blur_pipeline,
            composite_pipeline,
            blur_bind_layout,
            composite_bind_layout,
            sampler,
            blur_uniform_h,
            blur_uniform_v,
            composite_buffer,
            composite_stride,
            slot_uniforms: vec![None; MAX_INSTANCES],
            images: HashMap::new(),
            instances: Vec::with_capacity(MAX_INSTANCES),
            prepare_cursor: 0,
            draw_cursor: AtomicUsize::new(0),
            frame: 0,
        }
    }

    fn trim(&mut self) {
        self.instances.clear();
        self.prepare_cursor = 0;
        *self.draw_cursor.get_mut() = 0;
        self.frame = self.frame.wrapping_add(1);

        // `<=` keeps an entry through `STALE_FRAMES` full frames after its last
        // use, matching the docstring's "one second at 60 Hz" promise.
        let frame = self.frame;
        self.images.retain(|_, state| {
            frame.wrapping_sub(state.last_used_frame) <= STALE_FRAMES
        });
    }
}

impl BlurredImagePipeline {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        primitive: &BlurredImagePrimitive,
        bounds: &Rectangle,
        scale_factor: f32,
    ) {
        if self.prepare_cursor >= MAX_INSTANCES {
            return;
        }

        let cache_key = CacheKey::new(&primitive.backdrop, primitive.blur_radius);
        self.ensure_image(device, queue, cache_key, primitive);

        let Some(state) = self.images.get_mut(&cache_key) else {
            return;
        };
        state.last_used_frame = self.frame;

        let slot = self.prepare_cursor;
        let uniform = composite_uniform(primitive, bounds, scale_factor);
        if self.slot_uniforms[slot] != Some(uniform) {
            queue.write_buffer(
                &self.composite_buffer,
                slot as u64 * self.composite_stride,
                bytemuck::bytes_of(&uniform),
            );
            self.slot_uniforms[slot] = Some(uniform);
        }

        self.instances.push(cache_key);
        self.prepare_cursor += 1;
    }

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let slot = self.draw_cursor.fetch_add(1, Ordering::Relaxed);
        // iced's render-side culling differs subtly from prepare's, so a slot
        // we never wrote could be asked for here. Skip it rather than reading
        // stale data.
        if slot >= self.instances.len() {
            return;
        }

        let cache_key = self.instances[slot];
        let Some(state) = self.images.get(&cache_key) else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blurred image composite pass"),
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

        let offset = (slot as u64 * self.composite_stride) as u32;
        pass.set_bind_group(0, &state.composite_bind, &[offset]);
        pass.draw(0..3, 0..1);
    }

    /// Uploads and blurs an `(image, radius)` pair on first encounter. The
    /// blur passes run exactly once per entry, so two widgets that share an
    /// image at the same radius both reuse the cached blurred view, and two
    /// at different radii get separate entries instead of trampling each
    /// other.
    fn ensure_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache_key: CacheKey,
        primitive: &BlurredImagePrimitive,
    ) {
        if self.images.contains_key(&cache_key) {
            return;
        }

        let state = self.upload(device, queue, &primitive.backdrop);
        let texel = [
            1.0 / primitive.backdrop.width() as f32,
            1.0 / primitive.backdrop.height() as f32,
        ];
        queue.write_buffer(
            &self.blur_uniform_h,
            0,
            bytemuck::cast_slice(&pack_blur_uniform(
                texel,
                [1.0, 0.0],
                primitive.blur_radius,
            )),
        );
        queue.write_buffer(
            &self.blur_uniform_v,
            0,
            bytemuck::cast_slice(&pack_blur_uniform(
                texel,
                [0.0, 1.0],
                primitive.blur_radius,
            )),
        );

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blurred image blur"),
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

        self.images.insert(cache_key, state);
    }

    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &Backdrop,
    ) -> ImageState {
        let extent = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };
        let source = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("blurred image source"),
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
            image.rgba(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * 4),
                rows_per_image: Some(image.height()),
            },
            extent,
        );

        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
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
        let intermediate_view = blur_target("blurred image intermediate");
        let blurred_view = blur_target("blurred image blurred");

        let blur_h_bind = self.blur_bind(device, &self.blur_uniform_h, &source_view);
        let blur_v_bind =
            self.blur_bind(device, &self.blur_uniform_v, &intermediate_view);
        let composite_bind = self.composite_bind(device, &source_view, &blurred_view);

        ImageState {
            blur_h_bind,
            blur_v_bind,
            intermediate_view,
            blurred_view,
            composite_bind,
            last_used_frame: self.frame,
        }
    }

    fn blur_bind(
        &self,
        device: &wgpu::Device,
        uniform: &wgpu::Buffer,
        input: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blurred image blur bind"),
            layout: &self.blur_bind_layout,
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
        sharp: &wgpu::TextureView,
        blurred: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blurred image composite bind"),
            layout: &self.composite_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.composite_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(COMPOSITE_UNIFORM_SIZE),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(sharp),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(blurred),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}

/// Packs the composite uniform for `primitive` drawn over `bounds` with the
/// surface's `scale_factor`. Regions past [`MAX_REGIONS`] are silently dropped
/// so the shader's bounded loop matches the count it sees. `widget_*_px` are
/// the widget's full physical-pixel rect, so the shader can resolve where
/// each fragment sits relative to the widget rather than the (possibly
/// clipped) viewport.
fn composite_uniform(
    primitive: &BlurredImagePrimitive,
    bounds: &Rectangle,
    scale_factor: f32,
) -> CompositeUniform {
    let mut regions = [[0.0_f32; 4]; MAX_REGIONS];
    let mut region_radii = [[0.0_f32; 4]; MAX_REGIONS / 4];
    let count = primitive.regions.len().min(MAX_REGIONS);
    for (i, region) in primitive.regions.iter().take(count).enumerate() {
        let r = region.bounds;
        regions[i] = [r.x, r.y, r.width, r.height];
        region_radii[i / 4][i % 4] = region.corner_radius;
    }

    let progress = primitive.progress.unwrap_or(ProgressStrip::OFF);
    CompositeUniform {
        target_size: [bounds.width, bounds.height],
        widget_origin_px: [bounds.x * scale_factor, bounds.y * scale_factor],
        widget_size_px: [bounds.width * scale_factor, bounds.height * scale_factor],
        region_count: count as u32,
        corner_radius: primitive.corner_radius,
        progress_fill: progress.fill,
        progress_height: progress.height,
        _pad_progress: [0.0, 0.0],
        progress_color: color_array(progress.accent),
        progress_track: color_array(progress.track),
        regions,
        region_radii,
    }
}

/// Linear-space colour for the composite shader. The shader samples and
/// mixes in linear, so a raw sRGB triple would read washed-out.
fn color_array(color: iced::Color) -> [f32; 4] {
    color.into_linear()
}
