use std::sync::Arc;

use iced::widget::shader::{self, Viewport};
use iced::{Rectangle, wgpu};

use super::SnapshotImage;
use crate::gpu::{BLUR_FORMAT, SOURCE_FORMAT, as_bytes, blur_pass};

/// The primitive produced each draw; carries the live scrim parameters.
#[derive(Debug)]
pub struct ScrimPrimitive {
    pub snapshot: Arc<SnapshotImage>,
    /// Scrim blur radius (outside the panel), in snapshot pixels.
    pub blur: f32,
    /// Extra blur radius applied to the panel, compounded over the scrim blur.
    pub panel_blur: f32,
    /// Tint outside the panel: sRGB rgb + coverage alpha.
    pub scrim_tint: [f32; 4],
    /// Tint inside the panel: sRGB rgb + coverage alpha.
    pub panel_tint: [f32; 4],
    /// Panel size, in logical pixels.
    pub panel_size: [f32; 2],
    /// Panel corner radius, in logical pixels.
    pub corner_radius: f32,
    /// Saturation multiplier for the blurred scene (1 = unchanged).
    pub saturate: f32,
    /// Animation factor in `[0, 1]`; the scrim's overall output alpha.
    pub factor: f32,
}

impl shader::Primitive for ScrimPrimitive {
    type Pipeline = ScrimPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        // Resolve the panel rect to physical pixels for the shader's SDF mask;
        // the scrim fills the window, so the snapshot samples 1:1.
        let scale = viewport.scale_factor();
        let target = viewport.physical_size();
        let uniform: [f32; 20] = [
            self.scrim_tint[0],
            self.scrim_tint[1],
            self.scrim_tint[2],
            self.scrim_tint[3],
            self.panel_tint[0],
            self.panel_tint[1],
            self.panel_tint[2],
            self.panel_tint[3],
            target.width as f32,
            target.height as f32,
            self.panel_size[0] * scale,
            self.panel_size[1] * scale,
            self.factor,
            self.corner_radius * scale,
            self.saturate,
            0.0,
            self.snapshot.width as f32,
            self.snapshot.height as f32,
            0.0,
            0.0,
        ];
        pipeline.prepare(
            device,
            queue,
            &self.snapshot,
            (self.blur, self.panel_blur),
            &uniform,
        );
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

/// GPU resources shared by every [`ScrimPrimitive`].
pub struct ScrimPipeline {
    scrim_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    /// Layout for the blur passes (uniform + input texture + sampler).
    blur_layout: wgpu::BindGroupLayout,
    /// Layout for the composite (adds the heavier panel texture at binding 3).
    scrim_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    scrim_uniform: wgpu::Buffer,
    /// Blur uniforms: scrim horizontal/vertical, then panel horizontal/vertical.
    blur_uniform_h: wgpu::Buffer,
    blur_uniform_v: wgpu::Buffer,
    panel_uniform_h: wgpu::Buffer,
    panel_uniform_v: wgpu::Buffer,
    /// Per-snapshot GPU state, present once a snapshot has been uploaded.
    image: Option<ImageState>,
    scrim_bind: Option<wgpu::BindGroup>,
}

/// The textures and bind groups for the currently uploaded snapshot. The scrim
/// level blurs the source; the panel level blurs that result again (compounding
/// into a visibly heavier blur for the card).
struct ImageState {
    /// Identity of the source `Arc`, to detect when the snapshot changes.
    key: usize,
    width: u32,
    height: u32,
    /// `(scrim, panel)` radii baked into the blurred textures.
    radii: (f32, f32),
    /// Scrim level: source → `scrim_intermediate` (h) → `scrim_blurred` (v).
    scrim_h_bind: wgpu::BindGroup,
    scrim_v_bind: wgpu::BindGroup,
    scrim_intermediate_view: wgpu::TextureView,
    scrim_blurred_view: wgpu::TextureView,
    /// Panel level (half resolution): `scrim_blurred` → `panel_intermediate` (h)
    /// → `panel_blurred` (v).
    panel_h_bind: wgpu::BindGroup,
    panel_v_bind: wgpu::BindGroup,
    panel_intermediate_view: wgpu::TextureView,
    panel_blurred_view: wgpu::TextureView,
}

impl shader::Pipeline for ScrimPipeline {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scrim shader"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../shader_common.wgsl"),
                    include_str!("inspect.wgsl"),
                )
                .into(),
            ),
        });

        let uniform_entry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };

        // Blur passes: uniform + one input texture + sampler.
        let blur_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("scrim blur layout"),
                entries: &[uniform_entry, texture_entry(1), sampler_entry],
            });
        // Composite: also binds the heavier panel texture at binding 3.
        let scrim_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("scrim composite layout"),
                entries: &[
                    uniform_entry,
                    texture_entry(1),
                    sampler_entry,
                    texture_entry(3),
                ],
            });

        let layout = |label, bind: &wgpu::BindGroupLayout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[bind],
                push_constant_ranges: &[],
            })
        };
        let blur_pipeline_layout = layout("scrim blur pipeline layout", &blur_layout);
        let scrim_pipeline_layout =
            layout("scrim composite pipeline layout", &scrim_layout);

        let make_pipeline =
            |label: &str,
             entry: &str,
             target: wgpu::TextureFormat,
             blend,
             pipeline_layout: &wgpu::PipelineLayout| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(pipeline_layout),
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

        // The scrim alpha-blends over the still-crisp scene so `factor` can fade
        // it in; the blur passes just replace their intermediate targets.
        let scrim_pipeline = make_pipeline(
            "scrim composite",
            "fs_scrim",
            format,
            wgpu::BlendState::ALPHA_BLENDING,
            &scrim_pipeline_layout,
        );
        let blur_pipeline = make_pipeline(
            "scrim blur",
            "fs_blur",
            BLUR_FORMAT,
            wgpu::BlendState::REPLACE,
            &blur_pipeline_layout,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scrim sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let scrim_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scrim uniform"),
            size: SCRIM_UNIFORM_SIZE,
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

        Self {
            scrim_pipeline,
            blur_pipeline,
            blur_layout,
            scrim_layout,
            sampler,
            scrim_uniform,
            blur_uniform_h: blur_uniform("scrim blur uniform (h)"),
            blur_uniform_v: blur_uniform("scrim blur uniform (v)"),
            panel_uniform_h: blur_uniform("panel blur uniform (h)"),
            panel_uniform_v: blur_uniform("panel blur uniform (v)"),
            image: None,
            scrim_bind: None,
        }
    }
}

impl ScrimPipeline {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        snapshot: &Arc<SnapshotImage>,
        radii: (f32, f32),
        uniform: &[f32; 20],
    ) {
        self.ensure_image(device, queue, snapshot, radii);
        queue.write_buffer(&self.scrim_uniform, 0, as_bytes(uniform));

        let Some(state) = &self.image else {
            return;
        };
        self.scrim_bind = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scrim composite bind"),
            layout: &self.scrim_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.scrim_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &state.scrim_blurred_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        &state.panel_blurred_view,
                    ),
                },
            ],
        }));
    }

    /// Uploads `snapshot` if it changed, then (re)runs the blur levels if either
    /// radius changed.
    fn ensure_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        snapshot: &Arc<SnapshotImage>,
        radii: (f32, f32),
    ) {
        let key = Arc::as_ptr(snapshot) as usize;
        if self.image.as_ref().is_none_or(|state| state.key != key) {
            self.image = Some(self.upload(device, queue, snapshot, key));
        }

        // Disjoint field borrows: `image` is mutated while the pipeline's blur
        // resources are read.
        let Some(state) = self.image.as_mut() else {
            return;
        };
        if state.radii == radii {
            return;
        }
        state.radii = radii;
        let (scrim_blur, panel_blur) = radii;

        // Each pass's texel is its *input's* size and `radius` is in input
        // pixels. The panel level renders at half resolution (its input is
        // already heavily blurred, so the downsample is invisible): its first
        // pass reads the full-res scrim blur, its second the half-res
        // intermediate — hence the halved radius there.
        let (hw, hh) = (half(state.width) as f32, half(state.height) as f32);
        let full = [1.0 / state.width as f32, 1.0 / state.height as f32];
        let half_texel = [1.0 / hw, 1.0 / hh];
        let dir = |buffer, texel: [f32; 2], dx: f32, dy: f32, radius: f32| {
            queue.write_buffer(
                buffer,
                0,
                as_bytes(&[texel[0], texel[1], dx, dy, radius, 0.0, 0.0, 0.0]),
            );
        };
        dir(&self.blur_uniform_h, full, 1.0, 0.0, scrim_blur);
        dir(&self.blur_uniform_v, full, 0.0, 1.0, scrim_blur);
        dir(&self.panel_uniform_h, full, 1.0, 0.0, panel_blur);
        dir(
            &self.panel_uniform_v,
            half_texel,
            0.0,
            1.0,
            panel_blur * 0.5,
        );

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scrim blur"),
            });
        // Scrim level: source → blurred (separable).
        blur_pass(
            &mut encoder,
            &self.blur_pipeline,
            &state.scrim_intermediate_view,
            &state.scrim_h_bind,
        );
        blur_pass(
            &mut encoder,
            &self.blur_pipeline,
            &state.scrim_blurred_view,
            &state.scrim_v_bind,
        );
        // Panel level: blur the scrim result again, compounding into a heavier blur.
        blur_pass(
            &mut encoder,
            &self.blur_pipeline,
            &state.panel_intermediate_view,
            &state.panel_h_bind,
        );
        blur_pass(
            &mut encoder,
            &self.blur_pipeline,
            &state.panel_blurred_view,
            &state.panel_v_bind,
        );
        queue.submit(Some(encoder.finish()));
    }

    /// Uploads `snapshot` to a fresh source texture and builds the blur targets.
    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        snapshot: &SnapshotImage,
        key: usize,
    ) -> ImageState {
        let extent = wgpu::Extent3d {
            width: snapshot.width,
            height: snapshot.height,
            depth_or_array_layers: 1,
        };
        let source = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scrim snapshot source"),
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
            &snapshot.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(snapshot.width * 4),
                rows_per_image: Some(snapshot.height),
            },
            extent,
        );

        let blur_target = |label, width: u32, height: u32| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
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
        let (w, h) = (snapshot.width, snapshot.height);
        let (hw, hh) = (half(w), half(h));
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let scrim_intermediate_view = blur_target("scrim blur intermediate", w, h);
        let scrim_blurred_view = blur_target("scrim blur result", w, h);
        // Panel level renders at half resolution — see `ensure_image`.
        let panel_intermediate_view = blur_target("panel blur intermediate", hw, hh);
        let panel_blurred_view = blur_target("panel blur result", hw, hh);

        ImageState {
            key,
            width: snapshot.width,
            height: snapshot.height,
            radii: (f32::NAN, f32::NAN),
            scrim_h_bind: self.blur_bind(device, &self.blur_uniform_h, &source_view),
            scrim_v_bind: self.blur_bind(
                device,
                &self.blur_uniform_v,
                &scrim_intermediate_view,
            ),
            panel_h_bind: self.blur_bind(
                device,
                &self.panel_uniform_h,
                &scrim_blurred_view,
            ),
            panel_v_bind: self.blur_bind(
                device,
                &self.panel_uniform_v,
                &panel_intermediate_view,
            ),
            scrim_intermediate_view,
            scrim_blurred_view,
            panel_intermediate_view,
            panel_blurred_view,
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
            label: Some("scrim blur bind"),
            layout: &self.blur_layout,
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
        let Some(bind) = &self.scrim_bind else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scrim composite pass"),
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
        pass.set_pipeline(&self.scrim_pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// 20 `f32`s; see the `Scrim` struct in `inspect.wgsl`.
const SCRIM_UNIFORM_SIZE: u64 = 20 * 4;
/// 8 `f32`s; see the `Blur` struct in `shader_common.wgsl`.
const BLUR_UNIFORM_SIZE: u64 = 8 * 4;

/// Half a texture dimension, floored but never zero.
fn half(dimension: u32) -> u32 {
    (dimension / 2).max(1)
}
