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

use bluebottle_video::{MediaStats, Player, RenderPreset};
use gstreamer as gst;
use gstreamer::prelude::*;
use iced::widget::{Space, button, column, container, row, slider, text};

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

    // Resize the video surface straight from the window's resize (on the
    // event-loop thread), so it tracks the window without the latency of routing
    // the resize through the overlay UI.
    let resize_player = Arc::clone(&player);
    window.on_resize(move |width, height| resize_player.set_render_size(width, height));

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
    /// Whether the "stats for nerds" debug panel is visible.
    show_stats: bool,
    /// Latest debug snapshot, refreshed on the tick while the panel is shown.
    stats: Option<MediaStats>,
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
            show_stats: false,
            stats: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    TogglePause,
    Scrub(f64),
    SeekCommit,
    Tick,
    CyclePreset,
    ToggleStats,
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
            if state.show_stats {
                state.stats = Some(state.player.media_stats());
            }
        },
        Message::CyclePreset => {
            state.preset = match state.preset {
                RenderPreset::Fast => RenderPreset::Standard,
                RenderPreset::Standard => RenderPreset::HighQuality,
                RenderPreset::HighQuality => RenderPreset::Fast,
            };
            state.player.set_render_preset(state.preset);
        },
        Message::ToggleStats => {
            state.show_stats = !state.show_stats;
            state.stats = state.show_stats.then(|| state.player.media_stats());
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

    let controls = column![
        row![
            button(label).on_press(Message::TogglePause),
            button(text(quality)).on_press(Message::CyclePreset),
            button("stats").on_press(Message::ToggleStats),
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
    .spacing(12);

    let mut screen = column![].padding(24).spacing(12).height(iced::Length::Fill);
    if state.show_stats {
        screen = screen.push(stats_panel(state.stats.as_ref()));
    }
    // Push the controls to the bottom of the window.
    screen = screen.push(Space::new().height(iced::Length::Fill));
    screen.push(controls).into()
}

/// A monospaced "stats for nerds" panel summarising the active streams and the
/// render path.
fn stats_panel(stats: Option<&MediaStats>) -> iced::Element<'_, Message> {
    let body = match stats {
        Some(stats) => stats_text(stats),
        None => "collecting…".to_owned(),
    };
    let label = text(body)
        .font(iced::Font::MONOSPACE)
        .size(13)
        .color(iced::Color::WHITE);
    container(label)
        .padding(12)
        .style(|_| container::Style {
            background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
            border: iced::border::rounded(8),
            ..container::Style::default()
        })
        .into()
}

/// Render `stats` as grouped, labelled lines, using `—` for unknown fields.
fn stats_text(stats: &MediaStats) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    if let Some(video) = &stats.video {
        let _ = writeln!(out, "Video");
        let _ = writeln!(out, "  codec: {}", or_dash(&video.codec));
        let _ = writeln!(out, "  resolution: {}x{}", video.width, video.height);
        let framerate = video
            .framerate
            .map(|fps| format!("{fps:.3} fps"))
            .unwrap_or_else(dash);
        let _ = writeln!(out, "  framerate: {framerate}");
        let _ = writeln!(out, "  bitrate: {}", opt_bitrate(video.bitrate));
    }
    if let Some(audio) = &stats.audio {
        let _ = writeln!(out, "Audio");
        let _ = writeln!(out, "  codec: {}", or_dash(&audio.codec));
        let rate = audio
            .sample_rate
            .map(|hz| format!("{hz} Hz"))
            .unwrap_or_else(dash);
        let _ = writeln!(out, "  sample rate: {rate}");
        let channels = audio
            .channels
            .map(|count| count.to_string())
            .unwrap_or_else(dash);
        let _ = writeln!(out, "  channels: {channels}");
        let _ = writeln!(out, "  bitrate: {}", opt_bitrate(audio.bitrate));
        let _ = writeln!(out, "  language: {}", or_dash(&audio.language));
        let _ = writeln!(out, "  track: {}/{}", audio.track + 1, audio.track_count);
    }
    let subtitle = &stats.subtitle;
    let _ = writeln!(out, "Subtitle");
    if subtitle.track >= 0 {
        let _ = writeln!(
            out,
            "  track: {}/{}",
            subtitle.track + 1,
            subtitle.track_count
        );
        let _ = writeln!(out, "  language: {}", or_dash(&subtitle.language));
    } else {
        let _ = writeln!(out, "  none ({} available)", subtitle.track_count);
    }
    let _ = writeln!(out, "Render");
    match &stats.render {
        Some(render) => {
            let _ = writeln!(out, "  format: {}", render.format);
            let _ = writeln!(out, "  size: {}x{}", render.width, render.height);
            let path = if render.zero_copy {
                "zero-copy dmabuf"
            } else {
                "sysmem upload"
            };
            let _ = writeln!(out, "  path: {path}");
            let _ = writeln!(out, "  color: {}", or_dash(&render.color));
            let _ = writeln!(out, "  preset: {:?}", render.preset);
            let _ = writeln!(out, "  present fps: {:.1}", render.fps);
            let _ = writeln!(
                out,
                "  frames: {} presented / {} skipped",
                render.frames_presented, render.frames_skipped
            );
        },
        None => {
            let _ = writeln!(out, "  not started");
        },
    }
    out
}

fn dash() -> String {
    "—".to_owned()
}

fn or_dash(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(dash)
}

/// Format a bitrate in bits/s as kbps, or `—` when unknown.
fn opt_bitrate(bits_per_second: Option<u32>) -> String {
    bits_per_second
        .map(|bits| format!("{:.0} kbps", bits as f64 / 1000.0))
        .unwrap_or_else(dash)
}

fn subscription(_state: &App) -> iced::Subscription<Message> {
    iced::time::every(Duration::from_millis(250)).map(|_| Message::Tick)
}

/// Format a number of seconds as `m:ss`.
fn format_time(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
