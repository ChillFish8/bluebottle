use std::path::{Path, PathBuf};

use directories::ProjectDirs as OsProjectDirs;
use snafu::{OptionExt, Whatever};

/// The cache, config, and data directories for the app.
pub struct ProjectDirs {
    cache: PathBuf,
    #[allow(dead_code)]
    config: PathBuf,
    #[allow(dead_code)]
    data: PathBuf,
}

impl ProjectDirs {
    /// Resolves the directories, preferring an explicit `root` over the OS
    /// conventions.
    pub fn resolve(root: Option<PathBuf>) -> Result<Self, Whatever> {
        match root {
            Some(root) => Ok(Self {
                cache: root.join("cache"),
                config: root.join("config"),
                data: root.join("data"),
            }),
            None => {
                let dirs = OsProjectDirs::from("com", "chillfish8", "Bluebottle")
                    .whatever_context("resolve the OS storage directories")?;
                Ok(Self {
                    cache: dirs.cache_dir().to_path_buf(),
                    config: dirs.config_dir().to_path_buf(),
                    data: dirs.data_dir().to_path_buf(),
                })
            },
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    #[allow(dead_code)]
    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    #[allow(dead_code)]
    pub fn data_dir(&self) -> &Path {
        &self.data
    }
}
