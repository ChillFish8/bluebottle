//! Video playback with `bluebottle-window`, rendered by libplacebo.
//!
//! Builds a [`Player`] whose video sink is `placebosink` (libplacebo on a Vulkan
//! swapchain), points it at the content surface beneath a `bluebottle-window`
//! overlay, and draws a small transparent play/pause + seek UI on top.
//!
//! ```text
//! cargo run -p bluebottle-video --example player -- /path/to/video.mp4
//! cargo run -p bluebottle-video --example player -- https://example.com/v.webm
//! cargo run -p bluebottle-video --example player
//! ```
//!
//! [`Player`]: bluebottle_video::Player

use std::sync::Arc;
use std::time::Duration;

use bluebottle_video::{Player, RenderPreset};
use gstreamer as gst;
use gstreamer::prelude::*;
use iced::widget::{button, column, row, slider, text};

fn main() {
    tracing_subscriber::fmt::init();

    let player = Arc::new(build_player(std::env::args().nth(1)));

    let ui_player = Arc::clone(&player);
    let window = bluebottle_window::create_video_overlay(move || {
        iced::application(move || App::new(Arc::clone(&ui_player)), update, view)
            .title("Bluebottle libplacebo player")
            .subscription(subscription)
    })
    .expect("create video overlay window");

    // Embed the sink in the content surface, then start playback.
    player.bind_window(&window);
    player.play().expect("start playback");

    run_bus(&player, &window);

    // Stop the pipeline before the window tears down the Wayland connection the
    // libplacebo swapchain presents onto.
    player.stop();
    window.request_close();
    window.join().expect("overlay loop exited cleanly");
}

/// Build the player: a media URI via `playbin`, or a test pattern with no arg.
///
/// With no arg, set `BB_DMABUF=1` to exercise the zero-copy VA-API dmabuf path
/// (`videotestsrc ! vapostproc ! placebosink`) instead of the system-memory one.
fn build_player(media: Option<String>) -> Player {
    match media {
        Some(media) => Player::open(&to_uri(&media)).expect("open media"),
        None if std::env::var_os("BB_DMABUF").is_some() => {
            Player::test_pattern_dmabuf().expect("build dmabuf test pattern")
        },
        None => Player::test_pattern().expect("build test pattern"),
    }
}

/// Pass through anything URI-shaped; otherwise resolve a (possibly missing)
/// local path to an absolute `file://` URI so a bad path is a clean pipeline
/// error rather than a panic.
fn to_uri(media: &str) -> String {
    if media.contains("://") {
        return media.to_owned();
    }
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

/// Keep the process alive and surface pipeline messages; loop on end-of-stream.
fn run_bus(player: &Player, window: &bluebottle_window::Window) {
    let Some(bus) = player.bus() else {
        return;
    };
    while window.is_open() {
        let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else {
            continue;
        };
        match message.view() {
            gst::MessageView::Error(err) => {
                tracing::error!(
                    "pipeline error from {:?}: {}",
                    err.src().map(|source| source.path_string()),
                    err.error()
                );
                break;
            },
            gst::MessageView::Eos(_) => player.seek(Duration::ZERO),
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
    /// While dragging the slider we show the target and defer the seek to
    /// release, so the periodic tick does not fight the drag.
    scrubbing: bool,
    /// Current libplacebo render-quality preset.
    preset: RenderPreset,
}

impl App {
    fn new(player: Arc<Player>) -> Self {
        Self {
            player,
            paused: false,
            position: 0.0,
            duration: 0.0,
            scrubbing: false,
            preset: RenderPreset::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    TogglePause,
    Scrub(f64),
    SeekCommit,
    Tick,
    Resized(u32, u32),
    CyclePreset,
}

fn update(state: &mut App, message: Message) -> iced::Task<Message> {
    match message {
        Message::TogglePause => {
            state.paused = !state.paused;
            let _ = state.player.set_paused(state.paused);
        },
        Message::Scrub(position) => {
            state.scrubbing = true;
            state.position = position;
        },
        Message::SeekCommit => {
            state.scrubbing = false;
            state.player.seek(Duration::from_secs_f64(state.position));
        },
        Message::Tick => {
            if let Some(duration) = state.player.duration() {
                state.duration = duration.as_secs_f64();
            }
            if !state.scrubbing
                && let Some(position) = state.player.position()
            {
                state.position = position.as_secs_f64();
            }
        },
        Message::Resized(width, height) => {
            state.player.set_render_size(width, height);
        },
        Message::CyclePreset => {
            state.preset = match state.preset {
                RenderPreset::Fast => RenderPreset::Standard,
                RenderPreset::Standard => RenderPreset::HighQuality,
                RenderPreset::HighQuality => RenderPreset::Fast,
            };
            state.player.set_render_preset(state.preset);
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

    let quality = format!("quality: {:?}", state.preset);

    column![
        row![
            button(label).on_press(Message::TogglePause),
            button(text(quality)).on_press(Message::CyclePreset),
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

/// Map window open/resize events to [`Message::Resized`] so the sink's swapchain
/// tracks the window size.
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
