//! A tiny self contained startup splash.
//!
//! [`SplashRenderer`] owns its own wgpu context and paints a centred logo over a
//! solid background with a small white spinner just below it. It is meant to fill
//! a window's main surface while a heavier UI compiles its pipelines and settles
//! its first frame, then be dropped once that UI is ready to take over.
//!
//! The renderer is generic over the surface target, so it works with anything that
//! exposes raw window and display handles.

use std::borrow::Cow;
use std::time::{Duration, Instant};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use snafu::Snafu;

/// Largest fraction of the smaller window dimension the logo may occupy.
const LOGO_MAX_FRACTION: f32 = 0.28;
/// Spinner ring radius as a fraction of the window's smaller side. Relative to
/// the surface so the spinner keeps a consistent on screen size across display
/// scales rather than shrinking on HiDPI.
const SPINNER_RADIUS_FRACTION: f32 = 0.025;
/// Spinner ring thickness as a fraction of its radius.
const SPINNER_THICKNESS_FRACTION: f32 = 0.18;
/// Gap between the logo and the spinner as a fraction of the spinner radius.
const SPINNER_GAP_FRACTION: f32 = 1.8;
/// Target interval between frames. The present is non blocking, so this paces
/// the driving loop and keeps it responsive to a stop signal rather than parking
/// inside a vsync wait.
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Something that went wrong while building the splash renderer.
#[derive(Debug, Snafu)]
pub enum Error {
    /// The wgpu surface could not be created for the target.
    #[snafu(display("failed to create the splash surface: {message}"))]
    Surface { message: String },
    /// No GPU adapter could drive the surface.
    #[snafu(display("no suitable GPU adapter for the splash"))]
    Adapter,
    /// The GPU device could not be created.
    #[snafu(display("failed to create the splash device: {message}"))]
    Device { message: String },
    /// The surface has no configuration compatible with the adapter.
    #[snafu(display("the splash surface is not supported by the adapter"))]
    Unsupported,
}

/// A dead simple splash description: a logo painted over a background.
pub struct Splash {
    /// The logo image, drawn centred and scaled to fit.
    pub logo: image::DynamicImage,
    /// The solid background fill behind the logo and spinner.
    pub background: iced::Color,
}

impl Splash {
    /// Build a splash from a logo and a background colour.
    pub fn new(logo: image::DynamicImage, background: iced::Color) -> Self {
        Self { logo, background }
    }
}

/// The uniform block shared with `splash.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    resolution: [f32; 2],
    time: f32,
    fade: f32,
    logo_rect: [f32; 4],
    spinner: [f32; 4],
    background: [f32; 4],
}

/// Owns a wgpu context and paints a [`Splash`] onto a surface.
pub struct SplashRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    background: [f32; 4],
    logo_size: (u32, u32),
    fade: f32,
    start: Instant,
    last_frame: Instant,
}

impl SplashRenderer {
    /// Build a renderer targeting `target`'s surface at `size` physical pixels.
    pub fn new<W>(target: &W, size: (u32, u32), splash: &Splash) -> Result<Self, Error>
    where
        W: HasWindowHandle + HasDisplayHandle,
    {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let raw_display_handle = target
            .display_handle()
            .map_err(|err| Error::Surface {
                message: err.to_string(),
            })?
            .as_raw();
        let raw_window_handle = target
            .window_handle()
            .map_err(|err| Error::Surface {
                message: err.to_string(),
            })?
            .as_raw();

        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle,
                raw_window_handle,
            })
        }
        .map_err(|err| Error::Surface {
            message: err.to_string(),
        })?;

        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .map_err(|_| Error::Adapter)?;

        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default()),
        )
        .map_err(|err| Error::Device {
            message: err.to_string(),
        })?;

        let (width, height) = (size.0.max(1), size.1.max(1));
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or(Error::Unsupported)?;
        // Present without blocking on vblank so a frame is never parked waiting
        // for a compositor callback. The loop is paced in software instead (see
        // FRAME_INTERVAL), which keeps it responsive to the stop signal even if
        // the surface stops receiving frame callbacks.
        config.present_mode = wgpu::PresentMode::AutoNoVsync;
        // Composite with premultiplied alpha so the fade out blends over the UI
        // beneath. If the surface only offers opaque, the fade has no effect and
        // the splash is removed in one step instead.
        let caps = surface.get_capabilities(&adapter);
        if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            config.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
        }
        surface.configure(&device, &config);

        let gpu = build(&device, &queue, &config, splash);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline: gpu.pipeline,
            bind_group: gpu.bind_group,
            uniform_buffer: gpu.uniform_buffer,
            background: splash.background.into_linear(),
            logo_size: gpu.logo_size,
            fade: 1.0,
            start: Instant::now(),
            // Seed a frame in the past so the first present is not delayed.
            last_frame: Instant::now() - FRAME_INTERVAL,
        })
    }

    /// Set the overall opacity used by the next [`Self::render`].
    ///
    /// 1.0 shows the splash fully, 0.0 fades it out completely. The fade only
    /// blends over the UI beneath when the surface supports premultiplied alpha.
    pub fn set_fade(&mut self, fade: f32) {
        self.fade = fade.clamp(0.0, 1.0);
    }

    /// Reconfigure the surface for a new physical size.
    pub fn resize(&mut self, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) == (self.config.width, self.config.height) {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Present one animated frame, paced to roughly [`FRAME_INTERVAL`].
    ///
    /// The present itself does not block on vblank, so the pacing is a bounded
    /// software sleep. That keeps a driving loop free to re-check its stop
    /// condition every frame instead of parking indefinitely inside a present.
    pub fn render(&mut self) {
        // Pace to the target interval since the previous frame. Done first so the
        // sleep is bounded and never hides the loop's stop checks.
        let elapsed = self.last_frame.elapsed();
        if elapsed < FRAME_INTERVAL {
            std::thread::sleep(FRAME_INTERVAL - elapsed);
        }
        self.last_frame = Instant::now();

        let uniforms = self.uniforms();
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            },
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("splash"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.background[0] as f64,
                            g: self.background[1] as f64,
                            b: self.background[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
    }

    /// Compute the per frame uniforms, laying the logo and spinner out as a
    /// vertically centred group.
    fn uniforms(&self) -> Uniforms {
        let res = [self.config.width as f32, self.config.height as f32];
        let (logo_w, logo_h) = self.logo_size;

        let scale = if logo_w == 0 || logo_h == 0 {
            0.0
        } else {
            let max_w = res[0] * LOGO_MAX_FRACTION;
            let max_h = res[1] * LOGO_MAX_FRACTION;
            (max_w / logo_w as f32).min(max_h / logo_h as f32).min(1.0)
        };
        let disp_w = logo_w as f32 * scale;
        let disp_h = logo_h as f32 * scale;

        // Size the spinner relative to the surface so it scales with the display
        // rather than staying a fixed pixel size that shrinks on HiDPI.
        let unit = res[0].min(res[1]);
        let radius = (unit * SPINNER_RADIUS_FRACTION).max(2.0);
        let thickness = (radius * SPINNER_THICKNESS_FRACTION).max(1.5);
        let gap = radius * SPINNER_GAP_FRACTION;

        let group_h = disp_h + gap + 2.0 * radius;
        let top = (res[1] - group_h) * 0.5;
        let logo_x = (res[0] - disp_w) * 0.5;
        let spinner_cy = top + disp_h + gap + radius;

        Uniforms {
            resolution: res,
            time: self.start.elapsed().as_secs_f32(),
            fade: self.fade,
            logo_rect: [logo_x, top, disp_w, disp_h],
            spinner: [res[0] * 0.5, spinner_cy, radius, thickness],
            background: self.background,
        }
    }
}

/// The GPU resources built once for a [`SplashRenderer`].
struct SplashGpu {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    logo_size: (u32, u32),
}

/// Upload the logo and build the pipeline and bind group.
fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    config: &wgpu::SurfaceConfiguration,
    splash: &Splash,
) -> SplashGpu {
    let rgba = splash.logo.to_rgba8();
    let (logo_w, logo_h) = rgba.dimensions();

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("splash logo"),
        size: wgpu::Extent3d {
            width: logo_w.max(1),
            height: logo_h.max(1),
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
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * logo_w),
            rows_per_image: Some(logo_h),
        },
        wgpu::Extent3d {
            width: logo_w.max(1),
            height: logo_h.max(1),
            depth_or_array_layers: 1,
        },
    );
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("splash sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("splash uniforms"),
        size: std::mem::size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("splash binds"),
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

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("splash binds"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("splash"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("splash.wgsl"))),
    });
    let pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("splash"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("splash"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    SplashGpu {
        pipeline,
        bind_group,
        uniform_buffer,
        logo_size: (logo_w, logo_h),
    }
}
