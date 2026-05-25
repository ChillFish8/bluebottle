use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use gst::glib;
use gst_base::subclass::prelude::*;
use gst_video::subclass::prelude::*;
use gstreamer as gst;
use gstreamer_base as gst_base;
use gstreamer_video as gst_video;

use crate::config::RenderPreset;
use crate::render::RenderContext;
use crate::stats::RenderStats;

/// Mutable sink state, serialised behind a mutex (GStreamer may touch the
/// element from several threads; rendering only happens on the streaming
/// thread, which holds the lock for the duration of `show_frame`).
#[derive(Default)]
struct State {
    /// `wl_display` pointer, supplied via [`PlaceboSink::set_display`].
    display: Option<usize>,
    /// `wl_surface` pointer to present onto.
    window_handle: Option<usize>,
    /// Desired on-screen size in logical pixels (the window size).
    render_rect: Option<(u32, u32)>,
    /// Size last applied to the swapchain, to detect resizes.
    applied_rect: Option<(u32, u32)>,
    /// Render-quality preset (and the one last applied, to detect changes).
    preset: RenderPreset,
    applied_preset: Option<RenderPreset>,
    /// Negotiated input video format/geometry.
    info: Option<gst_video::VideoInfo>,
    /// DRM fourcc + modifier when the negotiated caps are dmabuf (zero-copy);
    /// `None` for the system-memory path.
    dma_drm: Option<(u32, u64)>,
    /// The render engine, created lazily once display+surface+caps are known.
    render: Option<RenderContext>,
}

/// `placebosink`: a video sink that renders frames with libplacebo and presents
/// them via a Vulkan swapchain onto a caller-provided Wayland surface.
#[derive(Default)]
pub struct PlaceboSink {
    state: Mutex<State>,
    /// Latest debug snapshot, published by the streaming thread after each
    /// frame. Kept under its own lock — separate from `state`, which the
    /// streaming thread holds across the whole GPU render+present — so a UI
    /// thread polling [`PlaceboSink::render_stats`] never blocks on the render.
    stats: Mutex<Option<RenderStats>>,
}

impl PlaceboSink {
    pub(super) fn set_display(&self, display: *mut c_void) {
        self.state.lock().unwrap().display = Some(display as usize);
    }

    pub(super) fn set_window_handle(&self, handle: *mut c_void) {
        self.state.lock().unwrap().window_handle = Some(handle as usize);
    }

    pub(super) fn set_render_rectangle(&self, width: u32, height: u32) {
        self.state.lock().unwrap().render_rect = Some((width, height));
    }

    pub(super) fn set_render_preset(&self, preset: RenderPreset) {
        self.state.lock().unwrap().preset = preset;
    }

    /// The latest render-path snapshot for the debug overlay, or `None` until
    /// the first frame has been rendered.
    ///
    /// Reads only the `stats` lock, so it does not contend with the per-frame
    /// render that holds `state`.
    pub(super) fn render_stats(&self) -> Option<RenderStats> {
        self.stats.lock().unwrap().clone()
    }
}

/// Summarise the negotiated colour space, flagging HDR transfer functions.
fn color_summary(info: &gst_video::VideoInfo) -> Option<String> {
    let colorimetry = info.colorimetry();
    let (transfer, hdr) = match colorimetry.transfer() {
        gst_video::VideoTransferFunction::Smpte2084 => ("PQ", true),
        gst_video::VideoTransferFunction::AribStdB67 => ("HLG", true),
        gst_video::VideoTransferFunction::Bt709 => ("BT.709", false),
        gst_video::VideoTransferFunction::Srgb => ("sRGB", false),
        _ => ("SDR", false),
    };
    let primaries = match colorimetry.primaries() {
        gst_video::VideoColorPrimaries::Bt709 => "BT.709",
        gst_video::VideoColorPrimaries::Bt2020 => "BT.2020",
        gst_video::VideoColorPrimaries::Smpte170m => "BT.601",
        _ => "unknown",
    };
    Some(format!(
        "{primaries} / {transfer}{}",
        if hdr { " (HDR)" } else { "" }
    ))
}

#[glib::object_subclass]
impl ObjectSubclass for PlaceboSink {
    const NAME: &'static str = "BluebottlePlaceboSink";
    type Type = super::PlaceboSink;
    type ParentType = gst_video::VideoSink;
}

impl ObjectImpl for PlaceboSink {}
impl GstObjectImpl for PlaceboSink {}

impl ElementImpl for PlaceboSink {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: OnceLock<gst::subclass::ElementMetadata> = OnceLock::new();
        Some(METADATA.get_or_init(|| {
            gst::subclass::ElementMetadata::new(
                "Bluebottle libplacebo sink",
                "Sink/Video",
                "Renders video with libplacebo and presents it via a Vulkan \
                 swapchain onto a Wayland surface",
                "bluebottle",
            )
        }))
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static TEMPLATES: OnceLock<Vec<gst::PadTemplate>> = OnceLock::new();
        TEMPLATES
            .get_or_init(|| {
                // dmabuf first so it's preferred (zero-copy); unsupported dmabuf
                // formats are rejected in `set_caps` and fall back to sysmem.
                let dmabuf = gst::Caps::builder("video/x-raw")
                    .features(["memory:DMABuf"])
                    .field("format", "DMA_DRM")
                    .build();
                let sysmem = gst_video::VideoCapsBuilder::new()
                    .format_list([
                        gst_video::VideoFormat::Bgra,
                        gst_video::VideoFormat::Rgba,
                        gst_video::VideoFormat::Bgrx,
                        gst_video::VideoFormat::Rgbx,
                        gst_video::VideoFormat::Argb,
                        gst_video::VideoFormat::Abgr,
                    ])
                    .build();
                let mut caps = gst::Caps::new_empty();
                {
                    let caps = caps.get_mut().unwrap();
                    caps.append(dmabuf);
                    caps.append(sysmem);
                }
                vec![
                    gst::PadTemplate::new(
                        "sink",
                        gst::PadDirection::Sink,
                        gst::PadPresence::Always,
                        &caps,
                    )
                    .unwrap(),
                ]
            })
            .as_slice()
    }
}

impl BaseSinkImpl for PlaceboSink {
    fn set_caps(&self, caps: &gst::Caps) -> Result<(), gst::LoggableError> {
        let (info, dma_drm) = if gst_video::is_dma_drm_caps(caps) {
            let drm = gst_video::VideoInfoDmaDrm::from_caps(caps).map_err(|_| {
                gst::loggable_error!(gst::CAT_RUST, "invalid dmabuf video caps")
            })?;
            let info = drm.to_video_info().map_err(|_| {
                gst::loggable_error!(gst::CAT_RUST, "dmabuf caps without video info")
            })?;
            // Reject dmabuf formats the import path can't handle, so negotiation
            // falls back to a system-memory format instead of failing mid-stream.
            if !crate::render::dmabuf_format_supported(info.format()) {
                return Err(gst::loggable_error!(
                    gst::CAT_RUST,
                    "unsupported dmabuf format {:?}",
                    info.format()
                ));
            }
            (info, Some((drm.fourcc(), drm.modifier())))
        } else {
            let info = gst_video::VideoInfo::from_caps(caps).map_err(|_| {
                gst::loggable_error!(gst::CAT_RUST, "invalid video caps")
            })?;
            (info, None)
        };
        let mut state = self.state.lock().unwrap();
        state.info = Some(info);
        state.dma_drm = dma_drm;
        Ok(())
    }

    fn propose_allocation(
        &self,
        query: &mut gst::query::Allocation,
    ) -> Result<(), gst::LoggableError> {
        // Advertise that we accept the video meta, which lets upstream (notably
        // VA-API decoders producing dmabufs) negotiate buffer allocation with
        // us; without this they fail with "could not decide allocation".
        query.add_allocation_meta::<gst_video::VideoMeta>(None);
        self.parent_propose_allocation(query)
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        // Tear down the render engine (and its Vulkan/surface objects) on the
        // streaming thread before the element leaves PLAYING.
        let mut state = self.state.lock().unwrap();
        state.render = None;
        state.applied_rect = None;
        *self.stats.lock().unwrap() = None;
        Ok(())
    }
}

impl VideoSinkImpl for PlaceboSink {
    fn show_frame(
        &self,
        buffer: &gst::Buffer,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut state = self.state.lock().unwrap();

        let info = state.info.clone().ok_or(gst::FlowError::NotNegotiated)?;

        // Lazily create the render engine once we know where to present.
        if state.render.is_none() {
            let (Some(display), Some(handle)) = (state.display, state.window_handle)
            else {
                // Not embedded yet; drop the frame rather than fail.
                return Ok(gst::FlowSuccess::Ok);
            };
            let (width, height) =
                state.render_rect.unwrap_or((info.width(), info.height()));
            let context = RenderContext::new(
                display as *mut c_void,
                handle as *mut c_void,
                width,
                height,
                state.preset,
            )
            .map_err(|err| {
                gst::element_imp_error!(
                    self,
                    gst::ResourceError::Failed,
                    ["failed to create libplacebo render context: {err}"]
                );
                gst::FlowError::Error
            })?;
            tracing::info!(
                zero_copy = state.dma_drm.is_some(),
                format = ?info.format(),
                "placebosink: render context created"
            );
            state.render = Some(context);
            state.applied_rect = state.render_rect;
            state.applied_preset = Some(state.preset);
        }

        // Resize on the streaming thread (never cross-thread). Mark it applied
        // only if libplacebo adopted the size, so a resize during surface
        // unavailability is retried on a later frame.
        if state.render_rect != state.applied_rect
            && let (Some((width, height)), Some(context)) =
                (state.render_rect, state.render.as_ref())
            && context.resize(width, height)
        {
            state.applied_rect = state.render_rect;
        }

        if state.applied_preset != Some(state.preset) {
            let preset = state.preset;
            if let Some(context) = state.render.as_mut() {
                context.set_preset(preset);
            }
            state.applied_preset = Some(preset);
        }

        let dma_drm = state.dma_drm;
        let context = state.render.as_mut().expect("render context present");
        context.render(buffer, &info, dma_drm).map_err(|err| {
            gst::element_imp_error!(
                self,
                gst::ResourceError::Failed,
                ["failed to render frame: {err}"]
            );
            gst::FlowError::Error
        })?;
        let runtime = context.runtime_stats();

        // Publish the snapshot under the `stats` lock (held only here, briefly),
        // not the `state` lock we hold across the render above.
        *self.stats.lock().unwrap() = Some(RenderStats {
            format: format!("{:?}", info.format()),
            width: info.width(),
            height: info.height(),
            zero_copy: dma_drm.is_some(),
            color: color_summary(&info),
            preset: state.preset,
            frames_presented: runtime.frames_presented,
            frames_skipped: runtime.frames_skipped,
            fps: runtime.fps,
        });

        Ok(gst::FlowSuccess::Ok)
    }
}
