//! A high-level video player built on `playbin` and [`PlaceboSink`].
//!
//! [`Player`] owns a GStreamer pipeline whose video sink is our libplacebo sink,
//! and exposes the usual transport controls (play/pause/seek/position) plus
//! [`Player::bind_window`], which points the sink at a `bluebottle-window`
//! content surface. The pipeline and sink are GObjects (`Send + Sync`), so a
//! `Player` can be shared across the UI and lifecycle threads via `Arc`.

use std::time::Duration;

use bluebottle_window::Window;
use bluebottle_window::platform::wayland::WindowExtWayland;
use gst::prelude::*;
use gstreamer as gst;

use crate::error::Error;
use crate::sink::PlaceboSink;

/// A libplacebo-rendered video player.
pub struct Player {
    pipeline: gst::Pipeline,
    sink: PlaceboSink,
}

impl Player {
    /// Open a media URI (file `file://`, http(s), ...) using `playbin`, which
    /// handles demux/decode/audio and auto-plugs conversion to our sink.
    pub fn open(uri: &str) -> Result<Self, Error> {
        Self::init()?;
        let sink = PlaceboSink::new();
        let playbin = gst::ElementFactory::make("playbin")
            .property("uri", uri)
            .property("video-sink", &sink)
            .build()
            .map_err(into_gst("create playbin"))?;
        let pipeline = playbin
            .downcast::<gst::Pipeline>()
            .expect("playbin is a pipeline");
        Ok(Self { pipeline, sink })
    }

    /// Build a `videotestsrc` pipeline rendering through the sink, for demos and
    /// smoke-testing with no media file (system-memory path).
    pub fn test_pattern() -> Result<Self, Error> {
        Self::init()?;
        let sink = PlaceboSink::new();
        let src = gst::ElementFactory::make("videotestsrc")
            .build()
            .map_err(into_gst("create videotestsrc"))?;
        let convert = gst::ElementFactory::make("videoconvert")
            .build()
            .map_err(into_gst("create videoconvert"))?;
        let pipeline = gst::Pipeline::with_name("bluebottle-video");
        pipeline
            .add_many([&src, &convert, sink.upcast_ref()])
            .map_err(into_gst("add elements"))?;
        gst::Element::link_many([&src, &convert, sink.upcast_ref()])
            .map_err(into_gst("link elements"))?;
        Ok(Self { pipeline, sink })
    }

    /// Build a VA-API pipeline that produces real packed dmabuf frames into the
    /// sink (`videotestsrc ! vapostproc ! placebosink`, forcing a single-plane
    /// packed DRM format), exercising the zero-copy import path with no media
    /// file or container demuxer.
    pub fn test_pattern_dmabuf() -> Result<Self, Error> {
        Self::init()?;
        let sink = PlaceboSink::new();
        let src = gst::ElementFactory::make("videotestsrc")
            .build()
            .map_err(into_gst("create videotestsrc"))?;
        let postproc = gst::ElementFactory::make("vapostproc")
            .build()
            .map_err(into_gst("create vapostproc (gst va plugin)"))?;
        // Force dmabuf output (exercising the zero-copy path); vapostproc picks
        // the DRM format and modifier, and the sink imports whatever it picks
        // (packed RGB or NV12).
        let caps = gst::Caps::builder("video/x-raw")
            .features(["memory:DMABuf"])
            .build();
        let filter = gst::ElementFactory::make("capsfilter")
            .property("caps", &caps)
            .build()
            .map_err(into_gst("create capsfilter"))?;
        let pipeline = gst::Pipeline::with_name("bluebottle-video-dmabuf");
        pipeline
            .add_many([&src, &postproc, &filter, sink.upcast_ref()])
            .map_err(into_gst("add elements"))?;
        gst::Element::link_many([&src, &postproc, &filter, sink.upcast_ref()])
            .map_err(into_gst("link elements"))?;
        Ok(Self { pipeline, sink })
    }

    /// Point the sink at a `bluebottle-window` content surface and size it to the
    /// window. Call before starting playback.
    ///
    /// The render size is the window's *logical* size: `bluebottle-window` leaves
    /// the content surface at buffer scale 1, so a logical-sized swapchain
    /// composites at the correct on-screen size (sharp HiDPI would additionally
    /// require the content surface to adopt the output scale).
    pub fn bind_window(&self, window: &Window) {
        // SAFETY: the window owns the `wl_display`/`wl_surface` for its lifetime,
        // which outlives the pipeline (the caller stops playback before dropping
        // the window — see the example).
        unsafe {
            self.sink.set_display(window.wl_display_ptr());
            self.sink.set_window_handle(window.wl_video_surface_ptr());
        }
        let (width, height) = window.size();
        self.sink.set_render_rectangle(width, height);
    }

    /// Update the on-screen render size in logical pixels (call on resize).
    pub fn set_render_size(&self, width: u32, height: u32) {
        self.sink.set_render_rectangle(width, height);
    }

    /// Select the render-quality preset (see [`RenderPreset`]). Safe to call at
    /// any time; takes effect on the next frame.
    ///
    /// [`RenderPreset`]: crate::RenderPreset
    pub fn set_render_preset(&self, preset: crate::config::RenderPreset) {
        self.sink.set_render_preset(preset);
    }

    /// Start (or resume) playback.
    pub fn play(&self) -> Result<(), Error> {
        self.set_state(gst::State::Playing)
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<(), Error> {
        self.set_state(gst::State::Paused)
    }

    /// Set the paused state.
    pub fn set_paused(&self, paused: bool) -> Result<(), Error> {
        if paused { self.pause() } else { self.play() }
    }

    /// Seek to `position` from the start of the stream (flushing).
    pub fn seek(&self, position: Duration) {
        let _ = self.pipeline.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            gst::ClockTime::from_nseconds(position.as_nanos() as u64),
        );
    }

    /// Current playback position, if known.
    pub fn position(&self) -> Option<Duration> {
        self.pipeline
            .query_position::<gst::ClockTime>()
            .map(|t| Duration::from_nanos(t.nseconds()))
    }

    /// Total stream duration, if known (e.g. unknown for live sources).
    pub fn duration(&self) -> Option<Duration> {
        self.pipeline
            .query_duration::<gst::ClockTime>()
            .map(|t| Duration::from_nanos(t.nseconds()))
    }

    /// The pipeline bus, for the caller's lifecycle/error loop.
    pub fn bus(&self) -> Option<gst::Bus> {
        self.pipeline.bus()
    }

    /// Stop the pipeline (transition to NULL). Call before the window tears down
    /// the Wayland connection the sink presents onto.
    pub fn stop(&self) {
        let _ = self.pipeline.set_state(gst::State::Null);
        let _ = self.pipeline.state(gst::ClockTime::from_seconds(2));
    }

    fn set_state(&self, state: gst::State) -> Result<(), Error> {
        self.pipeline
            .set_state(state)
            .map(|_| ())
            .map_err(|err| Error::Gstreamer {
                message: format!("failed to set pipeline state to {state:?}: {err}"),
            })
    }

    fn init() -> Result<(), Error> {
        gst::init().map_err(|err| Error::Gstreamer {
            message: format!("failed to initialise GStreamer: {err}"),
        })
    }
}

/// Build a closure mapping a GStreamer error into [`Error::Gstreamer`] with
/// context `what`.
fn into_gst(what: &'static str) -> impl FnOnce(gst::glib::BoolError) -> Error {
    move |err| Error::Gstreamer {
        message: format!("{what}: {err}"),
    }
}
