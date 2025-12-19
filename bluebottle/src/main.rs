use anyhow::Context;

mod app;
mod components;
mod screen;
mod view;

fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        unsafe { std::env::set_var("RUST_LOG", "info") };
    }

    tracing_subscriber::fmt::init();

    tracing::info!("starting Bluebottle");

    app::run_app().context("run iced UI")?;

    tracing::info!("system exit complete");

    Ok(())
}
