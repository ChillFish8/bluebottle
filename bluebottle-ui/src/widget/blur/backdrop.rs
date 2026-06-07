//! A decoded RGBA image, kept in shared CPU storage so it can be uploaded to
//! the GPU by any shader widget that needs it.

use std::fmt;
use std::sync::Arc;

/// A decoded backdrop image, kept as packed RGBA8 for GPU upload.
///
/// Cloning shares the pixels, so a clone stays the same image to the widget
/// and does not trigger a crossfade.
#[derive(Clone)]
pub struct Backdrop {
    inner: Arc<Pixels>,
}

struct Pixels {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

impl Backdrop {
    /// Wraps `width` x `height` row-major RGBA8 pixels.
    pub fn new(rgba: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            inner: Arc::new(Pixels {
                rgba,
                width,
                height,
            }),
        }
    }

    pub(crate) fn rgba(&self) -> &[u8] {
        &self.inner.rgba
    }

    pub(crate) fn width(&self) -> u32 {
        self.inner.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.inner.height
    }

    /// Identity of the shared pixels, used to detect when the image changes.
    pub(crate) fn key(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }
}

impl fmt::Debug for Backdrop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Backdrop")
            .field("width", &self.inner.width)
            .field("height", &self.inner.height)
            .field("bytes", &self.inner.rgba.len())
            .finish()
    }
}
