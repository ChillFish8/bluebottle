//! The cached "spotlight" image that seeds the main screen background.
//!
//! The trending/featured artwork is cached on disk under the storage root at
//! `spotlight/thumbnail.{png,jpg}`. When present it is decoded to packed RGBA8
//! for upload to the background shader; when absent the UI falls back to the
//! procedural gradient.

use std::fmt;
use std::path::Path;

/// A decoded spotlight image, kept as tightly packed RGBA8 for GPU upload.
pub struct SpotlightImage {
    /// Row-major RGBA8 pixels, `width * height * 4` bytes long.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl fmt::Debug for SpotlightImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpotlightImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.rgba.len())
            .finish()
    }
}

/// Loads the cached spotlight thumbnail from the storage `root`, if present.
///
/// Looks for `spotlight/thumbnail.png` then `spotlight/thumbnail.jpg`, decoding
/// the first that exists into packed RGBA8. A missing file or a decode failure
/// yields `None`, which the UI treats as "fall back to the procedural gradient".
pub fn load(root: &Path) -> Option<SpotlightImage> {
    let dir = root.join("spotlight");
    let path = ["thumbnail.png", "thumbnail.jpg"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.exists())?;

    match decode(&path) {
        Ok(image) => {
            tracing::info!(path = %path.display(), width = image.width, height = image.height, "loaded spotlight image");
            Some(image)
        },
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "failed to decode spotlight image");
            None
        },
    }
}

/// Decodes `path` into packed RGBA8, guessing the format from its contents.
fn decode(path: &Path) -> image::ImageResult<SpotlightImage> {
    let rgba = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(SpotlightImage {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}
