use std::path::PathBuf;

use snafu::ResultExt;

mod cache;
mod directory;
mod durable;
mod relaxed;
mod state;

pub use self::state::{with_durable_state, with_relaxed_state};

/// Initialise the app storage system.
pub fn init_storage(base_path: Option<PathBuf>) -> Result<(), snafu::Whatever> {
    directory::init_paths(base_path).whatever_context("init storage paths")?;
    state::init_state().whatever_context("init storage state")?;
    Ok(())
}
