//! The per-window render engine: composes the libplacebo wrappers and a
//! platform surface into a swapchain, and presents one decoded frame per call.
//!
//! Owned and driven by [`crate::sink`] on the GStreamer streaming thread. All
//! access is serialised through the sink's state mutex, so although the
//! libplacebo objects are not intrinsically thread-safe we mark the context
//! `Send` (see the `unsafe impl` below) to satisfy glib's subclass bounds.

use std::ffi::c_void;

use gstreamer as gst;
use gstreamer_allocators::DmaBufMemory;
use gstreamer_video as gst_video;
use gstreamer_video::prelude::*;
use placebo_sys as pl;

use crate::config::RenderPreset;
use crate::error::Error;
use crate::placebo::{
    Device,
    DmabufPlane,
    Instance,
    Log,
    Renderer,
    Swapchain,
    SysmemUploader,
    Texture,
    import_dmabuf,
};
use crate::platform::{self, Surface};

/// libplacebo render engine bound to one presentation surface.
///
/// Field order is the drop order and is load-bearing: GPU objects (renderer,
/// swapchain, textures) must be torn down before the surface, which must be
/// torn down before the device and instance, which must outlive nothing but the
/// log.
pub struct RenderContext {
    renderer: Renderer,
    swapchain: Swapchain,
    /// One persistent upload texture per source plane (sysmem path).
    uploaders: Vec<SysmemUploader>,
    /// dmabuf-imported textures for the in-flight frame, kept alive until the
    /// next frame replaces them (the zero-copy path).
    imported: Vec<Texture>,
    _surface: Surface,
    device: Device,
    _instance: Instance,
    _log: Log,
    /// Tuned render parameters (scalers, colour management, dither). M3 will
    /// expose these; for now the libplacebo defaults.
    params: pl::pl_render_params,
}

// SAFETY: the contained libplacebo objects are only ever touched from the
// GStreamer streaming thread, serialised by the sink's state mutex. We never
// share a `RenderContext` across threads concurrently.
unsafe impl Send for RenderContext {}

impl RenderContext {
    /// Build a render context presenting onto `(display, surface)` (native
    /// `wl_display` / `wl_surface` pointers), sized to `width`×`height` physical
    /// pixels.
    pub fn new(
        display: *mut c_void,
        surface_ptr: *mut c_void,
        width: u32,
        height: u32,
        preset: RenderPreset,
    ) -> Result<Self, Error> {
        let log = Log::new()?;
        let instance = Instance::new(&log, platform::surface_extensions())?;
        let surface = platform::create_surface(&instance, display, surface_ptr)?;
        let device = Device::for_surface(&log, &instance, surface.handle())?;
        let swapchain = Swapchain::new(&device, surface.handle())?;
        swapchain.resize(width, height);
        let renderer = Renderer::new(&log, device.gpu())?;
        let params = preset.to_params();

        Ok(Self {
            renderer,
            swapchain,
            uploaders: Vec::new(),
            imported: Vec::new(),
            _surface: surface,
            device,
            _instance: instance,
            _log: log,
            params,
        })
    }

    /// Resize the swapchain to `width`×`height` physical pixels.
    pub fn resize(&self, width: u32, height: u32) {
        self.swapchain.resize(width, height);
    }

    /// Switch the render-quality preset; takes effect on the next frame.
    pub fn set_preset(&mut self, preset: RenderPreset) {
        self.params = preset.to_params();
    }

    /// Render one decoded frame to the swapchain and present it.
    ///
    /// `dma_drm` carries the DRM fourcc + modifier when the negotiated caps are
    /// `video/x-raw(memory:DMABuf)`, selecting the zero-copy import path;
    /// otherwise frames are uploaded from system memory.
    ///
    /// Returns `Ok(())` even when the surface is temporarily unavailable (the
    /// frame is silently dropped, as libplacebo recommends).
    pub fn render(
        &mut self,
        buffer: &gst::Buffer,
        info: &gst_video::VideoInfo,
        dma_drm: Option<(u32, u64)>,
    ) -> Result<(), Error> {
        // Release the previous frame's imported textures.
        self.imported.clear();

        let frame = match dma_drm {
            Some((fourcc, modifier)) => {
                self.import_dmabuf(buffer, info, fourcc, modifier)?
            },
            None => self.upload_sysmem(buffer, info)?,
        };

        let Some(sw_frame) = self.swapchain.start_frame() else {
            // Surface hidden/minimised: skip this frame, not an error.
            return Ok(());
        };

        let mut target = pl::pl_frame::default();
        // SAFETY: `sw_frame` came from a successful `start_frame`.
        unsafe { pl::pl_frame_from_swapchain(&mut target, &sw_frame) };

        let mut image = pl::pl_frame {
            num_planes: frame.planes.len() as i32,
            repr: frame.repr,
            ..Default::default()
        };
        for (index, plane) in frame.planes.iter().enumerate() {
            image.planes[index] = *plane;
        }

        // SAFETY: image/target frames and their textures belong to this
        // context's gpu and outlive the call; `params` is a valid
        // `pl_render_params`.
        let result = unsafe { self.renderer.render(&image, &target, &self.params) };
        if result.is_ok() {
            self.swapchain.submit_frame();
            self.swapchain.swap_buffers();
        }
        result
    }

    /// Upload each source plane from system memory and return the planes,
    /// referencing this context's persistent upload textures.
    fn upload_sysmem(
        &mut self,
        buffer: &gst::Buffer,
        info: &gst_video::VideoInfo,
    ) -> Result<ImagePlanes, Error> {
        let frame = gst_video::VideoFrame::from_buffer_readable(buffer.clone(), info)
            .map_err(|_| Error::UnsupportedFormat {
                message: "could not map video frame for reading".into(),
            })?;

        let component_map =
            packed_rgba_map(info.format()).ok_or_else(|| Error::UnsupportedFormat {
                message: format!("unsupported sysmem format {:?}", info.format()),
            })?;

        // Single packed plane (BGRA/RGBA family).
        let pixels = frame.plane_data(0).map_err(|_| Error::UnsupportedFormat {
            message: "frame has no plane 0".into(),
        })?;
        let stride = frame.plane_stride()[0] as usize;

        if self.uploaders.is_empty() {
            self.uploaders.push(SysmemUploader::new(self.device.gpu()));
        }
        let data = crate::placebo::packed8_plane_data(
            info.width() as i32,
            info.height() as i32,
            component_map.components,
            component_map.map,
            4,
            stride,
            pixels.as_ptr() as *const c_void,
        );
        let plane = self.uploaders[0].upload(&data)?;
        Ok(ImagePlanes {
            planes: vec![plane],
            repr: rgb_repr(),
        })
    }

    /// Import a dmabuf frame zero-copy: each plane's DRM-fourcc'd buffer is
    /// wrapped as a `pl_tex` with no CPU copy. Handles single-plane packed RGB
    /// (e.g. from `vapostproc`) and 2-plane NV12 (the typical VA-API output).
    fn import_dmabuf(
        &mut self,
        buffer: &gst::Buffer,
        info: &gst_video::VideoInfo,
        fourcc: u32,
        modifier: u64,
    ) -> Result<ImagePlanes, Error> {
        let layout = dmabuf_layout(info.format(), fourcc).ok_or_else(|| {
            Error::UnsupportedFormat {
                message: format!(
                    "dmabuf import not implemented for {:?}",
                    info.format()
                ),
            }
        })?;

        // dmabuf fds and per-plane offsets/strides: VA exposes the planes as one
        // shared fd at different offsets (n_memory == 1) or one fd per plane.
        let meta = buffer.meta::<gst_video::VideoMeta>();
        let gpu = self.device.gpu();
        let mut planes = Vec::with_capacity(layout.planes.len());

        for (index, spec) in layout.planes.iter().enumerate() {
            let memory_index = if buffer.n_memory() == 1 { 0 } else { index };
            let dmabuf = buffer
                .peek_memory(memory_index)
                .downcast_memory_ref::<DmaBufMemory>()
                .ok_or_else(|| Error::UnsupportedFormat {
                    message: "dmabuf caps but buffer memory is not a dmabuf".into(),
                })?;
            let (offset, stride) = match &meta {
                Some(meta) => (meta.offset()[index], meta.stride()[index] as usize),
                None => (0, info.stride()[index] as usize),
            };
            let texture = import_dmabuf(
                gpu,
                spec.fourcc,
                modifier,
                (info.width() as i32) >> spec.width_shift,
                (info.height() as i32) >> spec.height_shift,
                &DmabufPlane {
                    fd: dmabuf.fd(),
                    offset,
                    stride,
                },
            )?;
            planes.push(pl::pl_plane {
                texture: texture.raw(),
                components: spec.components,
                component_mapping: spec.mapping,
                ..Default::default()
            });
            self.imported.push(texture);
        }

        Ok(ImagePlanes {
            planes,
            repr: layout.repr,
        })
    }
}

/// The image planes plus the colour representation to interpret them with.
struct ImagePlanes {
    planes: Vec<pl::pl_plane>,
    repr: pl::pl_color_repr,
}

/// One dmabuf plane's import spec: its DRM fourcc, subsampling shift, and the
/// libplacebo component mapping (semantic index of each channel).
struct PlaneSpec {
    fourcc: u32,
    width_shift: i32,
    height_shift: i32,
    components: i32,
    mapping: [i32; 4],
}

/// The full plane layout for a dmabuf format.
struct DmabufLayout {
    planes: Vec<PlaneSpec>,
    repr: pl::pl_color_repr,
}

/// `fourcc_code(a, b, c, d)` — assemble a little-endian DRM FourCC.
const fn fourcc(code: &[u8; 4]) -> u32 {
    (code[0] as u32)
        | (code[1] as u32) << 8
        | (code[2] as u32) << 16
        | (code[3] as u32) << 24
}

/// Per-plane import layout for a dmabuf video format, or `None` if unsupported.
///
/// `combined` is the overall DRM fourcc from the caps; for packed single-plane
/// formats it is used directly, while planar formats use fixed per-plane
/// fourccs (e.g. NV12 → `R8` luma + `GR88` interleaved chroma).
fn dmabuf_layout(format: gst_video::VideoFormat, combined: u32) -> Option<DmabufLayout> {
    use gst_video::VideoFormat;
    match format {
        // Packed RGB(A): one plane; the DRM format encodes channel order, so the
        // mapping is the identity.
        VideoFormat::Bgra
        | VideoFormat::Rgba
        | VideoFormat::Bgrx
        | VideoFormat::Rgbx
        | VideoFormat::Argb
        | VideoFormat::Abgr => Some(DmabufLayout {
            planes: vec![PlaneSpec {
                fourcc: combined,
                width_shift: 0,
                height_shift: 0,
                components: 4,
                mapping: [0, 1, 2, 3],
            }],
            repr: rgb_repr(),
        }),
        // NV12: luma plane (R8) + half-res interleaved chroma plane (GR88).
        VideoFormat::Nv12 => Some(DmabufLayout {
            planes: vec![
                PlaneSpec {
                    fourcc: fourcc(b"R8  "),
                    width_shift: 0,
                    height_shift: 0,
                    components: 1,
                    mapping: [0, -1, -1, -1],
                },
                PlaneSpec {
                    fourcc: fourcc(b"GR88"),
                    width_shift: 1,
                    height_shift: 1,
                    components: 2,
                    mapping: [1, 2, -1, -1],
                },
            ],
            repr: ycbcr_repr(8, 8),
        }),
        // P010: like NV12 but 16-bit samples carrying 10 bits in the high bits
        // (R16 luma + GR1616 chroma).
        VideoFormat::P01010le => Some(DmabufLayout {
            planes: vec![
                PlaneSpec {
                    fourcc: fourcc(b"R16 "),
                    width_shift: 0,
                    height_shift: 0,
                    components: 1,
                    mapping: [0, -1, -1, -1],
                },
                PlaneSpec {
                    fourcc: fourcc(b"GR32"),
                    width_shift: 1,
                    height_shift: 1,
                    components: 2,
                    mapping: [1, 2, -1, -1],
                },
            ],
            repr: ycbcr_repr(16, 10),
        }),
        _ => None,
    }
}

/// A full-range RGB colour representation.
fn rgb_repr() -> pl::pl_color_repr {
    pl::pl_color_repr {
        sys: pl::pl_color_system_PL_COLOR_SYSTEM_RGB,
        ..Default::default()
    }
}

/// A limited-range BT.709 YCbCr colour representation (the common HD video
/// case). `sample_depth` is the storage bit depth and `color_depth` the
/// meaningful bits, packed in the high bits (P010-style `bit_shift`). Finer
/// per-stream colorimetry mapping is a future refinement.
fn ycbcr_repr(sample_depth: i32, color_depth: i32) -> pl::pl_color_repr {
    pl::pl_color_repr {
        sys: pl::pl_color_system_PL_COLOR_SYSTEM_BT_709,
        levels: pl::pl_color_levels_PL_COLOR_LEVELS_LIMITED,
        bits: pl::pl_bit_encoding {
            sample_depth,
            color_depth,
            bit_shift: sample_depth - color_depth,
        },
        ..Default::default()
    }
}

/// The semantic component mapping for a supported packed 8-bit format.
struct PackedRgba {
    components: usize,
    map: [i32; 4],
}

/// Map a packed 8-bit video format to its libplacebo component order, or `None`
/// if unsupported by the sysmem path. (videoconvert can always produce BGRA.)
fn packed_rgba_map(format: gst_video::VideoFormat) -> Option<PackedRgba> {
    use gst_video::VideoFormat;
    let (components, map) = match format {
        VideoFormat::Rgba => (4, [0, 1, 2, 3]),
        VideoFormat::Bgra => (4, [2, 1, 0, 3]),
        VideoFormat::Rgbx => (3, [0, 1, 2, -1]),
        VideoFormat::Bgrx => (3, [2, 1, 0, -1]),
        VideoFormat::Argb => (4, [3, 0, 1, 2]),
        VideoFormat::Abgr => (4, [3, 2, 1, 0]),
        _ => return None,
    };
    Some(PackedRgba { components, map })
}
