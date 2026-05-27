//! The backdrop image that seeds the main screen background.
//!
//! Resolved in order: the cached trending "spotlight" image under the cache
//! directory, otherwise the `default-backdrop.jpeg` shipped in the config
//! directory. Both are decoded to packed RGBA8 for upload to the background
//! shader; only if neither is available does the UI fall back to the procedural
//! gradient.

use std::fmt;
use std::path::Path;

use crate::project_dirs::ProjectDirs;

/// A decoded backdrop image, kept as tightly packed RGBA8 for GPU upload.
pub struct BackdropImage {
    /// Row-major RGBA8 pixels, `width * height * 4` bytes long.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl fmt::Debug for BackdropImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackdropImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.rgba.len())
            .finish()
    }
}

/// Resolves the backdrop image: the cached spotlight if present, otherwise the
/// default backdrop from the config directory.
pub fn resolve(dirs: &ProjectDirs) -> Option<BackdropImage> {
    load_spotlight(dirs.cache_dir()).or_else(|| load_default(dirs.config_dir()))
}

/// Loads the cached spotlight thumbnail from `cache`, if present.
///
/// Looks for `spotlight/thumbnail.png` then `spotlight/thumbnail.jpg`.
fn load_spotlight(cache: &Path) -> Option<BackdropImage> {
    let dir = cache.join("spotlight");
    let path = ["thumbnail.png", "thumbnail.jpg", "thumbnail.jpeg"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.exists())?;
    load(&path, "spotlight")
}

/// Loads the default backdrop from `<config>/default-backdrop.jpeg`.
fn load_default(config: &Path) -> Option<BackdropImage> {
    let path = config.join("default-backdrop.jpeg");
    if !path.exists() {
        tracing::warn!(
            path = %path.display(),
            "default backdrop missing; falling back to the gradient",
        );
        return None;
    }
    load(&path, "default backdrop")
}

/// Decodes `path` into packed RGBA8, logging the outcome.
fn load(path: &Path, kind: &str) -> Option<BackdropImage> {
    match decode(path) {
        Ok(image) => {
            tracing::info!(
                kind,
                path = %path.display(),
                width = image.width,
                height = image.height,
                "loaded backdrop image",
            );
            Some(image)
        },
        Err(error) => {
            tracing::warn!(
                kind,
                path = %path.display(),
                %error,
                "failed to decode backdrop image",
            );
            None
        },
    }
}

/// Decodes `path` into packed RGBA8, guessing the format from its contents.
fn decode(path: &Path) -> image::ImageResult<BackdropImage> {
    let rgba = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(BackdropImage {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}
