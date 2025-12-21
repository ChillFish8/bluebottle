use std::path::PathBuf;
use clap::Parser;
use snafu::ResultExt;

mod app;
mod backends;
mod components;
mod screen;
mod storage;
mod view;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    /// Enable debugging logging.
    debug: bool,
    #[arg(long, env = "BLUEBOTTLE_STORAGE_PATH")]
    /// The explicit folder path to store app state.
    ///
    /// If this is not set, it will use the conventional OS paths.
    storage_path: Option<PathBuf>,
}

fn main() -> Result<(), snafu::Whatever> {
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

    storage::init_storage(args.storage_path)
        .whatever_context("init app storage")?;
    
    tracing::info!("starting Bluebottle");

    app::run_app()?;

    tracing::info!("system exit complete");

    Ok(())
}
