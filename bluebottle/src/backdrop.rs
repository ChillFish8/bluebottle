use std::path::Path;

use bluebottle_ui::splash_background::Backdrop;

use crate::project_dirs::ProjectDirs;

/// Resolves the backdrop image: the cached spotlight if present, otherwise the
/// default backdrop from the config directory.
pub fn resolve(dirs: &ProjectDirs) -> Option<Backdrop> {
    load_spotlight(dirs.cache_dir()).or_else(|| load_default(dirs.config_dir()))
}

/// Loads the cached spotlight thumbnail from `cache`, if present.
///
/// Looks for `spotlight/thumbnail.png` then `spotlight/thumbnail.jpg`.
fn load_spotlight(cache: &Path) -> Option<Backdrop> {
    let dir = cache.join("spotlight");
    let path = ["thumbnail.png", "thumbnail.jpg", "thumbnail.jpeg"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.exists())?;
    load(&path, "spotlight")
}

/// Loads the default backdrop from `<config>/default-backdrop.jpeg`.
fn load_default(config: &Path) -> Option<Backdrop> {
    let path = config.join("default-backdrop.jpeg");
    if !path.exists() {
        tracing::warn!(
            path = %path.display(),
            "default backdrop missing; falling back to the glow",
        );
        return None;
    }
    load(&path, "default backdrop")
}

/// Decodes `path` into a backdrop, logging the outcome.
fn load(path: &Path, kind: &str) -> Option<Backdrop> {
    match decode(path) {
        Ok(backdrop) => {
            tracing::info!(kind, path = %path.display(), "loaded backdrop image");
            Some(backdrop)
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

/// Decodes encoded image `bytes` into a backdrop, guessing the format from their
/// contents; logs and returns `None` on failure.
pub fn decode_bytes(bytes: &[u8]) -> Option<Backdrop> {
    match image::load_from_memory(bytes) {
        Ok(image) => Some(into_backdrop(image)),
        Err(error) => {
            tracing::warn!(%error, "failed to decode embedded image");
            None
        },
    }
}

/// Decodes `path` into a backdrop, guessing the format from its contents.
fn decode(path: &Path) -> image::ImageResult<Backdrop> {
    let image = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?;
    Ok(into_backdrop(image))
}

/// Packs a decoded image into a [`Backdrop`] as RGBA8.
fn into_backdrop(image: image::DynamicImage) -> Backdrop {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Backdrop::new(rgba.into_raw(), width, height)
}
