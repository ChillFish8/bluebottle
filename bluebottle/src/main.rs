use anyhow::Context;
use clap::Parser;

mod app;
mod components;
mod screen;
mod view;
mod backends;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    /// Enable debugging logging.
    debug: bool,
}

fn main() -> anyhow::Result<()> {
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

    app::run_app().context("run iced UI")?;

    tracing::info!("system exit complete");

    Ok(())
}
