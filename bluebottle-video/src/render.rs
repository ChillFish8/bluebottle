//! The per-window render engine: composes the libplacebo wrappers and a
//! platform surface into a swapchain, and presents one decoded frame per call.
//!
//! Owned and driven by [`crate::sink`] on the GStreamer streaming thread. All
//! access is serialised through the sink's state mutex, so although the
//! libplacebo objects are not intrinsically thread-safe we mark the context
//! `Send` (see the `unsafe impl` below) to satisfy glib's subclass bounds.

use std::collections::VecDeque;
use std::ffi::c_void;
use std::time::Instant;

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

/// Per-frame GPU resources retained until the GPU has finished with them: the
/// dmabuf-imported textures and the source `gst::Buffer`. Holding the buffer
/// keeps its dmabuf memory from being freed or recycled (overwritten) by the
/// decoder pool while the GPU is still sampling the import.
struct FrameHold {
    _textures: Vec<Texture>,
    _buffer: Option<gst::Buffer>,
}

/// How many recent frames' GPU resources to retain. The swapchain's default
/// depth is 3 (up to 3 frames queued), so keeping the last 4 guarantees a
/// frame's dmabuf is no longer in use by the time we release it.
const MAX_IN_FLIGHT: usize = 4;

/// How many recent present timestamps to keep for the FPS estimate.
const FPS_WINDOW: usize = 30;

/// A point-in-time view of the render loop's runtime counters.
pub(crate) struct RenderRuntime {
    pub frames_presented: u64,
    pub frames_skipped: u64,
    pub fps: f64,
}

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
    /// Resources for frames still in flight on the GPU (zero-copy path); see
    /// [`FrameHold`] and [`MAX_IN_FLIGHT`].
    in_flight: VecDeque<FrameHold>,
    _surface: Surface,
    device: Device,
    _instance: Instance,
    _log: Log,
    /// Tuned render parameters (scalers, colour management, dither), selected by
    /// the active [`RenderPreset`].
    params: pl::pl_render_params,
    frames_presented: u64,
    frames_skipped: u64,
    /// Recent present timestamps (most recent at the back), for the FPS estimate.
    present_times: VecDeque<Instant>,
}

// SAFETY: the contained libplacebo objects are only ever touched from the
// GStreamer streaming thread (the sink calls `render`/`resize`/`set_preset`
// only from `show_frame`; the cross-thread setter methods on the sink mutate
// plain request fields under the state mutex, never this context). We never
// access a `RenderContext` from two threads concurrently.
unsafe impl Send for RenderContext {}

impl RenderContext {
    /// Build a render context presenting onto `(display, surface)` (native
    /// `wl_display` / `wl_surface` pointers), sized to `width`×`height` logical
    /// pixels (the bluebottle content surface stays at buffer scale 1).
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
            in_flight: VecDeque::new(),
            _surface: surface,
            device,
            _instance: instance,
            _log: log,
            params,
            frames_presented: 0,
            frames_skipped: 0,
            present_times: VecDeque::new(),
        })
    }

    /// A snapshot of the runtime counters for the debug overlay.
    pub(crate) fn runtime_stats(&self) -> RenderRuntime {
        // FPS over the window's span; needs at least two samples to have one.
        let fps = match (self.present_times.front(), self.present_times.back()) {
            (Some(oldest), Some(newest)) if self.present_times.len() >= 2 => {
                let span = newest.duration_since(*oldest).as_secs_f64();
                if span > 0.0 {
                    (self.present_times.len() - 1) as f64 / span
                } else {
                    0.0
                }
            },
            _ => 0.0,
        };
        RenderRuntime {
            frames_presented: self.frames_presented,
            frames_skipped: self.frames_skipped,
            fps,
        }
    }

    /// Resize the swapchain to `width`×`height` logical pixels. Returns whether
    /// libplacebo adopted the new size (it can refuse while the surface is
    /// unavailable, in which case the caller should retry).
    pub fn resize(&self, width: u32, height: u32) -> bool {
        self.swapchain.resize(width, height)
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
        let (frame, textures) = match dma_drm {
            Some((fourcc, modifier)) => {
                self.import_dmabuf(buffer, info, fourcc, modifier)?
            },
            None => (self.upload_sysmem(buffer, info)?, Vec::new()),
        };

        let Some(sw_frame) = self.swapchain.start_frame() else {
            // Surface hidden/minimised: skip this frame, not an error. The
            // imported textures (and buffer) drop here, unused by the GPU.
            self.frames_skipped += 1;
            return Ok(());
        };

        let mut target = pl::pl_frame::default();
        // SAFETY: `sw_frame` came from a successful `start_frame`.
        unsafe { pl::pl_frame_from_swapchain(&mut target, &sw_frame) };

        // `pl_frame.planes` is a fixed array of `PL_MAX_PLANES`; never write past it.
        let plane_count = frame.planes.len().min(pl::PL_MAX_PLANES as usize);
        let mut image = pl::pl_frame {
            num_planes: plane_count as i32,
            repr: frame.repr,
            ..Default::default()
        };
        image.planes[..plane_count].copy_from_slice(&frame.planes[..plane_count]);

        // SAFETY: image/target frames and their textures belong to this
        // context's gpu and outlive the call; `params` is a valid
        // `pl_render_params`.
        let render_result =
            unsafe { self.renderer.render(&image, &target, &self.params) };

        // `start_frame` must be paired with `submit_frame` even when rendering
        // failed, or the swapchain is left in an invalid state and wedges the
        // next `start_frame`. Present only if submission succeeded.
        if self.swapchain.submit_frame() {
            self.swapchain.swap_buffers();
            self.frames_presented += 1;
            self.present_times.push_back(Instant::now());
            while self.present_times.len() > FPS_WINDOW {
                self.present_times.pop_front();
            }
        }

        // Retain this frame's GPU resources for the in-flight window so the
        // dmabuf import (and its backing buffer) is not freed/recycled mid-read.
        self.in_flight.push_back(FrameHold {
            _textures: textures,
            _buffer: dma_drm.is_some().then(|| buffer.clone()),
        });
        while self.in_flight.len() > MAX_IN_FLIGHT {
            self.in_flight.pop_front();
        }

        render_result
    }

    /// Upload the source frame from system memory (the packed BGRA/RGBA path).
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

    /// Import a dmabuf frame zero-copy: each plane is wrapped as a `pl_tex` with
    /// no CPU copy (packed RGB, or 2-plane NV12/P010). Returns the planes plus
    /// the imported textures, which the caller keeps alive (with the buffer)
    /// until the GPU is done.
    fn import_dmabuf(
        &mut self,
        buffer: &gst::Buffer,
        info: &gst_video::VideoInfo,
        fourcc: u32,
        modifier: u64,
    ) -> Result<(ImagePlanes, Vec<Texture>), Error> {
        let layout = dmabuf_layout(info.format(), fourcc).ok_or_else(|| {
            Error::UnsupportedFormat {
                message: format!(
                    "dmabuf import not implemented for {:?}",
                    info.format()
                ),
            }
        })?;

        // Per-plane offsets/strides come from the VideoMeta when it describes
        // all our planes, else the negotiated VideoInfo's standard layout. (VA
        // exposes planes as one shared fd at offsets, or one fd per plane.)
        let meta = buffer
            .meta::<gst_video::VideoMeta>()
            .filter(|meta| meta.n_planes() as usize >= layout.planes.len());
        let gpu = self.device.gpu();
        let mut planes = Vec::with_capacity(layout.planes.len());
        let mut textures = Vec::with_capacity(layout.planes.len());

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
                None => (info.offset()[index], info.stride()[index] as usize),
            };
            let texture = import_dmabuf(
                gpu,
                spec.fourcc,
                modifier,
                // Round up: subsampled chroma planes of odd-dimensioned frames
                // need the ceiling, not the floor.
                ((info.width() as i32) + (1 << spec.width_shift) - 1)
                    >> spec.width_shift,
                ((info.height() as i32) + (1 << spec.height_shift) - 1)
                    >> spec.height_shift,
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
            textures.push(texture);
        }

        Ok((
            ImagePlanes {
                planes,
                repr: layout.repr,
            },
            textures,
        ))
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

/// Whether the dmabuf import path can handle `format`.
///
/// Used by the sink to reject dmabuf caps it cannot import *during
/// negotiation*, so upstream falls back to a system-memory format rather than
/// failing on the first frame.
pub(crate) fn dmabuf_format_supported(format: gst_video::VideoFormat) -> bool {
    dmabuf_layout(format, 0).is_some()
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
