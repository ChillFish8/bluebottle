use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use snafu::ResultExt;

static PATHS: OnceLock<DirectoryPaths> = OnceLock::new();

/// Init the directory paths used by the app.
pub fn init_paths(base_path: Option<PathBuf>) -> Result<(), snafu::Whatever> {
    let paths = PATHS.get_or_init(|| {
        if let Some(base_path) = base_path {
            DirectoryPaths::from_explicit_path(base_path)
        } else {
            DirectoryPaths::from_sniffed()
        }
    });
    
    paths.ensure_created()?;
    
    tracing::info!(
        config = %paths.config_dir().display(),
        cache = %paths.cache_dir().display(),
        data = %paths.data_dir().display(),
        "directory paths have been initialised"
    );
    
    Ok(())
}

/// Get the paths used to use in the app.
pub fn paths() -> &'static DirectoryPaths {
    PATHS.get().expect("paths was not initialized")
}


/// Manages file paths for app data storage.
pub struct DirectoryPaths {
    config_dir: PathBuf,
    cache_dir: PathBuf,
    data_dir: PathBuf,
}

impl DirectoryPaths {
    fn from_explicit_path(path: PathBuf) -> Self {
        Self {
            config_dir: path.join("config"),
            cache_dir: path.join("cache"),
            data_dir: path.join("data"),
        }
    }
    
    fn from_sniffed() -> Self {
        let paths = directories::ProjectDirs::from("com", "chillfish8", "Bluebottle")
            .expect("Could not determine project directories");
        Self {
            config_dir: paths.config_dir().to_path_buf(),
            cache_dir: paths.cache_dir().to_path_buf(),
            data_dir: paths.data_dir().to_path_buf(),
        }
    }

    fn ensure_created(&self) -> Result<(), snafu::Whatever> {
        std::fs::create_dir_all(&self.config_dir)
            .with_whatever_context(|_| format!("create directory {}", self.config_dir.display()))?;
        std::fs::create_dir_all(&self.cache_dir)
            .with_whatever_context(|_| format!("create directory {}", self.cache_dir.display()))?;
        std::fs::create_dir_all(&self.data_dir)
            .with_whatever_context(|_| format!("create directory {}", self.data_dir.display()))?;
        Ok(())
    }
    
    /// Returns the directory that should hold configuration files.
    pub fn config_dir(&self) -> &Path {
        self.config_dir.as_path()
    }

    /// Returns the directory that should hold cache & temporary files.
    pub fn cache_dir(&self) -> &Path {
        self.cache_dir.as_path()
    }

    /// Returns the directory that should hold data files.
    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }
    
}
