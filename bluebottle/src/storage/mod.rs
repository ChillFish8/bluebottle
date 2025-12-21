use std::path::PathBuf;
use snafu::ResultExt;

mod cache;
mod state;
mod directory;

/// Initialise the app storage system.
pub fn init_storage(base_path: Option<PathBuf>) -> Result<(), snafu::Whatever> {
    directory::init_paths(base_path)
        .whatever_context("init storage paths")?;
    state::init_state()
        .whatever_context("init storage state")?;
    Ok(())
}

