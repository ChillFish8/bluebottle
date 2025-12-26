use std::path::PathBuf;

use snafu::ResultExt;

mod asset_cache;
mod content_cache;
mod directory;
mod durable;
mod relaxed;
mod state;

pub use self::state::{submit_relaxed_state, with_durable_state, with_relaxed_state};

/// Initialise the app storage system.
pub fn init_storage(base_path: Option<PathBuf>) -> Result<(), snafu::Whatever> {
    directory::init_paths(base_path).whatever_context("init storage paths")?;
    state::init_state().whatever_context("init storage state")?;
    Ok(())
}

/// Returns a new timestamp in milliseconds.
pub(super) fn now() -> i64 {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as i64
}

#[cfg(test)]
pub mod test_utils {

    #[rstest::fixture]
    /// Creates a new temporary directory with storage initialised.
    pub fn temp_storage() -> tempfile::TempDir {
        let temp_dir = tempfile::tempdir().unwrap();
        super::init_storage(Some(temp_dir.path().to_path_buf())).unwrap();
        temp_dir
    }
}
