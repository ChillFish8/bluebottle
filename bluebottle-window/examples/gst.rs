//! Video playback with `bluebottle-window`, rendered by GStreamer.
//!
//! [`create_video_overlay`] hands back a [`Window`] whose main surface has a
//! transparent *content* subsurface stacked beneath the Iced overlay. This
//! example points GStreamer's `waylandsink` at that surface (via the
//! `GstVideoOverlay` interface), so GStreamer renders the video itself, below
//! the UI. The Iced overlay is a small transparent play/pause + seek bar drawn
//! on top.
//!
//! Run it with a file or URL, or with no argument for a test pattern:
//!
//! ```text
//! cargo run --example gst -- /path/to/video.mp4
//! cargo run --example gst -- https://example.com/stream.webm
//! cargo run --example gst
//! ```
//!
//! [`create_video_overlay`]: bluebottle_window::create_video_overlay
//! [`Window`]: bluebottle_window::Window

use std::ffi::c_void;
use std::sync::Arc;
use std::time::Duration;

use bluebottle_window::Window;
use bluebottle_window::platform::wayland::WindowExtWayland;
use gstreamer as gst;
use gstreamer::glib;
use gstreamer::glib::translate::{ToGlibPtr, ToGlibPtrMut};
use gstreamer::prelude::*;
use gstreamer_video as gst_video;
use gstreamer_video::prelude::*;
use iced::widget::{button, column, row, slider, text};

/// A `wl_display` pointer made `Send`/`Sync` so it can move into the bus sync
/// handler. libwayland access is internally synchronised.
#[derive(Clone, Copy)]
struct DisplayPtr(*mut c_void);
// SAFETY: the pointer references a long-lived `wl_display`; libwayland
// synchronises access internally.
unsafe impl Send for DisplayPtr {}
unsafe impl Sync for DisplayPtr {}

/// The playback pipeline plus its overlay interface, shared between the main
/// thread (lifecycle/bus) and the Iced overlay thread (controls). GStreamer
/// objects are `Send`/`Sync`, so both threads drive them directly.
struct Player {
    pipeline: gst::Pipeline,
    overlay: gst_video::VideoOverlay,
}

fn main() {
    tracing_subscriber::fmt::init();
    gst::init().expect("initialise GStreamer");

    let (pipeline, sink) = build_pipeline(std::env::args().nth(1));
    let player = Arc::new(Player {
        pipeline,
        overlay: sink
            .dynamic_cast::<gst_video::VideoOverlay>()
            .expect("waylandsink implements GstVideoOverlay"),
    });

    // The Iced overlay holds the player too, so its controls can drive playback.
    let ui_player = Arc::clone(&player);
    let window = bluebottle_window::create_video_overlay(move || {
        iced::application(move || App::new(Arc::clone(&ui_player)), update, view)
            .title("Bluebottle GStreamer Demo")
            .subscription(subscription)
    })
    .expect("create video overlay window");

    play_video(&window, &player);

    // Stop the pipeline before the window tears down the Wayland connection:
    // waylandsink's subsurfaces reference the `wl_display`, which `join` drops.
    let _ = player.pipeline.set_state(gst::State::Null);
    let _ = player.pipeline.state(gst::ClockTime::from_seconds(2));

    window.request_close();
    window.join().expect("overlay loop exited cleanly");
}

/// Build the pipeline and return it alongside the `waylandsink` element.
///
/// With an argument we let `playbin` handle demux/decode/audio for a file or
/// URL; with none we play a `videotestsrc` pattern so the example runs with no
/// setup.
fn build_pipeline(media: Option<String>) -> (gst::Pipeline, gst::Element) {
    let sink = gst::ElementFactory::make("waylandsink")
        .build()
        .expect("create waylandsink (gst-plugins-bad)");

    let pipeline = match media {
        Some(media) => {
            let uri = to_uri(&media);
            let playbin = gst::ElementFactory::make("playbin")
                .property("uri", &uri)
                .property("video-sink", &sink)
                .build()
                .expect("create playbin");
            // playbin is itself a GstPipeline.
            playbin
                .downcast::<gst::Pipeline>()
                .expect("playbin is a pipeline")
        },
        None => {
            let src = gst::ElementFactory::make("videotestsrc")
                .build()
                .expect("create videotestsrc");
            let convert = gst::ElementFactory::make("videoconvert")
                .build()
                .expect("create videoconvert");
            let pipeline = gst::Pipeline::with_name("bluebottle-gst");
            pipeline
                .add_many([&src, &convert, &sink])
                .expect("add elements");
            gst::Element::link_many([&src, &convert, &sink]).expect("link elements");
            pipeline
        },
    };

    (pipeline, sink)
}

/// Turn a CLI argument into a URI: pass through anything that already looks like
/// one, otherwise treat it as a local file path.
fn to_uri(media: &str) -> String {
    if media.contains("://") {
        return media.to_owned();
    }

    // `filename_to_uri` rejects relative paths, so resolve to an absolute one
    // even when the file is missing (canonicalize fails): that way a bad path
    // surfaces as a clean pipeline error rather than panicking here.
    let path = std::path::Path::new(media);
    let absolute = path.canonicalize().unwrap_or_else(|_| {
        std::env::current_dir()
            .map(|dir| dir.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    });
    gst::glib::filename_to_uri(&absolute, None)
        .map(|uri| uri.to_string())
        .unwrap_or_else(|_| media.to_owned())
}

/// Build the `wl_display` context `waylandsink` requests.
///
/// libgstwayland's `gst_wayland_display_handle_context_new` is not wrapped by
/// gstreamer-rs (and linking it directly trips the linker's `--as-needed`), so
/// assemble the equivalent here: a `GstWaylandDisplayHandleContextType` carrying
/// the `wl_display` under the pointer field `"handle"`.
fn wayland_display_context(display: *mut c_void) -> gst::Context {
    let mut context = gst::Context::new("GstWaylandDisplayHandleContextType", true);
    let structure = context
        .get_mut()
        .expect("freshly created context is uniquely owned")
        .structure_mut();
    // SAFETY: a pointer-typed GValue is not expressible through the safe
    // Structure API, so set it through the C calls. `display` outlives the
    // sink's use of the context.
    unsafe {
        let mut value = glib::Value::from_type(glib::Type::POINTER);
        glib::gobject_ffi::g_value_set_pointer(value.to_glib_none_mut().0, display);
        gst::ffi::gst_structure_set_value(
            structure.as_mut_ptr(),
            c"handle".as_ptr(),
            value.to_glib_none().0,
        );
    }
    context
}

/// Embed `waylandsink` into the content surface and drive the pipeline until the
/// window closes.
fn play_video(window: &Window, player: &Player) {
    let bus = player.pipeline.bus().expect("pipeline has a bus");

    // waylandsink asks for the `wl_display` via a need-context message; answer
    // it with the connection bluebottle owns.
    let display = DisplayPtr(window.wl_display_ptr());
    bus.set_sync_handler(move |_bus, message| {
        // Capture the `Send` wrapper as a whole, not its `!Send` pointer field
        // (Rust 2021 precise capture).
        let display = display;
        if let gst::MessageView::NeedContext(need) = message.view()
            && need.context_type() == "GstWaylandDisplayHandleContextType"
            && let Some(source) = message.src()
            && let Some(element) = source.dynamic_cast_ref::<gst::Element>()
        {
            element.set_context(&wayland_display_context(display.0));
        }
        gst::BusSyncReply::Pass
    });

    // Point the sink at the content surface (cast to a window-handle integer)
    // and size it to the window, before the sink builds its surfaces.
    // SAFETY: the pointer is the content `wl_surface` the library owns and keeps
    // alive for the window's lifetime.
    unsafe {
        player
            .overlay
            .set_window_handle(window.wl_video_surface_ptr() as usize);
    }
    let (width, height) = window.size();
    player
        .overlay
        .set_render_rectangle(0, 0, width as i32, height as i32)
        .expect("set render rectangle");

    player
        .pipeline
        .set_state(gst::State::Playing)
        .expect("start playback");

    // Keep the process alive and surface pipeline messages. Rendering and the UI
    // run on their own threads; here we only react to errors / end-of-stream.
    while window.is_open() {
        let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else {
            continue;
        };
        match message.view() {
            gst::MessageView::Error(err) => {
                tracing::error!(
                    "pipeline error from {:?}: {}",
                    err.src().map(|s| s.path_string()),
                    err.error()
                );
                break;
            },
            gst::MessageView::Eos(_) => {
                // Loop the file rather than exit.
                let _ = player.pipeline.seek_simple(
                    gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                    gst::ClockTime::ZERO,
                );
            },
            _ => {},
        }
    }
}

// ---------------------------------------------------------------------------
// Iced overlay: a transparent play/pause + seek bar drawn over the video.
// ---------------------------------------------------------------------------

struct App {
    player: Arc<Player>,
    paused: bool,
    /// Playback position in seconds (the slider value).
    position: f64,
    /// Total duration in seconds, or `0.0` if not yet known (e.g. live).
    duration: f64,
    /// While the user drags the slider we show their target and defer the seek
    /// to release, so the periodic tick does not fight the drag.
    scrubbing: bool,
}

impl App {
    fn new(player: Arc<Player>) -> Self {
        Self {
            player,
            paused: false,
            position: 0.0,
            duration: 0.0,
            scrubbing: false,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    TogglePause,
    /// Slider moved during a drag.
    Scrub(f64),
    /// Slider released: commit the seek.
    SeekCommit,
    /// Periodic refresh of position/duration.
    Tick,
    /// The window opened or resized to this logical size.
    Resized(u32, u32),
}

fn update(state: &mut App, message: Message) -> iced::Task<Message> {
    match message {
        Message::TogglePause => {
            state.paused = !state.paused;
            let next = if state.paused {
                gst::State::Paused
            } else {
                gst::State::Playing
            };
            let _ = state.player.pipeline.set_state(next);
        },
        Message::Scrub(position) => {
            state.scrubbing = true;
            state.position = position;
        },
        Message::SeekCommit => {
            state.scrubbing = false;
            let _ = state.player.pipeline.seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_nseconds((state.position * 1e9) as u64),
            );
        },
        Message::Tick => {
            if let Some(duration) =
                state.player.pipeline.query_duration::<gst::ClockTime>()
            {
                state.duration = duration.nseconds() as f64 / 1e9;
            }
            if !state.scrubbing
                && let Some(position) =
                    state.player.pipeline.query_position::<gst::ClockTime>()
            {
                state.position = position.nseconds() as f64 / 1e9;
            }
        },
        // Keep the sink's video rectangle matched to the window.
        Message::Resized(width, height) => {
            let _ = state.player.overlay.set_render_rectangle(
                0,
                0,
                width as i32,
                height as i32,
            );
        },
    }

    iced::Task::none()
}

fn view(state: &App) -> iced::Element<'_, Message> {
    let label = if state.paused { "play" } else { "pause" };
    let seek = slider(
        0.0..=state.duration.max(0.1),
        state.position,
        Message::Scrub,
    )
    .step(1.0)
    .on_release(Message::SeekCommit);

    column![
        row![
            button(label).on_press(Message::TogglePause),
            text(format!(
                "{} / {}",
                format_time(state.position),
                format_time(state.duration)
            ))
            .size(18),
        ]
        .spacing(16),
        seek,
    ]
    .spacing(12)
    .padding(24)
    .into()
}

fn subscription(_state: &App) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        iced::time::every(Duration::from_millis(250)).map(|_| Message::Tick),
        iced::event::listen_with(window_event),
    ])
}

/// Map window open/resize events to a [`Message::Resized`] so the overlay can
/// keep the sink's render rectangle in sync.
fn window_event(
    event: iced::Event,
    _status: iced::event::Status,
    _id: iced::window::Id,
) -> Option<Message> {
    use iced::window::Event;

    match event {
        iced::Event::Window(Event::Opened { size, .. } | Event::Resized(size)) => {
            Some(Message::Resized(size.width as u32, size.height as u32))
        },
        _ => None,
    }
}

/// Format a number of seconds as `m:ss`.
fn format_time(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
