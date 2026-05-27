mod app;
mod backdrop;
mod background;
mod gpu;
mod project_dirs;
mod screen;
mod sidebar;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bluebottle_video::Player;
use clap::Parser;
use gstreamer as gst;
use gstreamer::prelude::*;
use snafu::{ResultExt, Whatever};

use crate::app::App;
use crate::background::BackgroundSource;
use crate::project_dirs::ProjectDirs;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    /// Enable debugging logging.
    debug: bool,
    #[arg(long, env = "BLUEBOTTLE_STORAGE_PATH")]
    /// The explicit root folder for app state, holding `cache/`, `config/`, and
    /// `data/` directly.
    ///
    /// If this is not set, it will use the conventional OS paths.
    storage_path: Option<PathBuf>,
}

#[snafu::report]
fn main() -> Result<(), Whatever> {
    let args = Args::parse();

    if std::env::var("RUST_LOG").is_err() {
        let directive = if args.debug {
            "debug,wgpu=warn,naga=warn,cosmic_text=info"
        } else {
            "info"
        };
        unsafe { std::env::set_var("RUST_LOG", directive) };
    }

    if std::env::var("WGPU_POWER_PREF").is_err() {
        tracing::info!("setting GPU power preference to low");
        unsafe { std::env::set_var("WGPU_POWER_PREF", "low") };
    }

    tracing_subscriber::fmt::init();

    tracing::info!("starting Bluebottle");

    let dirs = ProjectDirs::resolve(args.storage_path)?;
    let source = Arc::new(BackgroundSource::new(backdrop::resolve(&dirs)));

    gst::init().whatever_context("initialise GStreamer")?;

    // The player is built before the window so an `Arc` can be moved into the
    // overlay's build closure; it is bound to the content surface afterwards.
    let player =
        Arc::new(Player::test_pattern().whatever_context("build the video player")?);

    let ui_player = Arc::clone(&player);
    let window = bluebottle_window::create_video_overlay(move || {
        let mut application = iced::application(
            {
                let player = Arc::clone(&ui_player);
                let source = Arc::clone(&source);
                move || App::new(Arc::clone(&player), Arc::clone(&source))
            },
            App::update,
            App::view,
        )
        .title("BlueBottle")
        .subscription(App::subscription)
        .default_font(bluebottle_ui::font::regular())
        .theme(|_state: &App| bluebottle_ui::color::theme());

        for font in bluebottle_ui::font::required_fonts() {
            application = application.font(font);
        }

        application
    })
    .whatever_context("create the application window")?;

    player.bind_window(&window);

    // Track the content surface to the window straight from the event-loop
    // thread, avoiding the latency of routing the resize through the UI.
    let resize_player = Arc::clone(&player);
    window.on_resize(move |width, height| resize_player.set_render_size(width, height));

    // The player only renders while the player screen is active; keep the
    // process alive and surface pipeline errors until the window closes.
    run_bus(&player, &window);

    // Stop the pipeline before the window tears down the Wayland connection the
    // libplacebo swapchain presents onto.
    player.stop();
    window.request_close();
    window
        .join()
        .whatever_context("overlay loop exited cleanly")?;

    tracing::info!("system exit complete");

    Ok(())
}

/// Pumps the pipeline bus, keeping the process alive until the window closes and
/// logging any pipeline error; loops playback on end-of-stream.
fn run_bus(player: &Player, window: &bluebottle_window::Window) {
    let Some(bus) = player.bus() else {
        return;
    };
    while window.is_open() {
        let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else {
            continue;
        };
        match message.view() {
            gst::MessageView::Error(error) => {
                tracing::error!(
                    "pipeline error from {:?}: {}",
                    error.src().map(|source| source.path_string()),
                    error.error()
                );
                break;
            },
            gst::MessageView::Eos(_) => player.seek(Duration::ZERO),
            _ => {},
        }
    }
}
