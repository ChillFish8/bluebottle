use placebo_sys as pl;

use super::log::Log;
use super::vulkan::Gpu;
use crate::error::{CreateSnafu, Error, OperationSnafu};

/// A libplacebo renderer: the object that runs a `pl_frame` (the decoded image)
/// through the scaling / colour-management / dithering pipeline onto a target
/// `pl_frame` (the swapchain framebuffer).
///
/// Must be dropped before the [`super::Device`] whose [`Gpu`] it was created
/// with; the owning render context guarantees this by field order.
pub struct Renderer {
    raw: pl::pl_renderer,
}

impl Renderer {
    /// Create a renderer for `gpu`.
    pub fn new(log: &Log, gpu: Gpu) -> Result<Self, Error> {
        // SAFETY: `log`/`gpu` outlive the renderer (enforced by the owning
        // context's field order).
        let raw = unsafe { pl::pl_renderer_create(log.raw(), gpu.raw()) };
        snafu::ensure!(
            !raw.is_null(),
            CreateSnafu {
                what: "pl_renderer"
            }
        );
        Ok(Self { raw })
    }

    /// Render `image` onto `target` with `params`.
    ///
    /// The frames are passed as raw pointers because they reference textures
    /// and colour metadata assembled by the caller for this single call.
    ///
    /// # Safety
    /// `image`, `target` and `params` must be valid `pl_frame`/`pl_render_params`
    /// for the duration of the call, and their textures must belong to the same
    /// `gpu` this renderer was created with.
    pub unsafe fn render(
        &self,
        image: *const pl::pl_frame,
        target: *const pl::pl_frame,
        params: *const pl::pl_render_params,
    ) -> Result<(), Error> {
        // SAFETY: upheld by the caller per the contract above.
        let ok = unsafe { pl::pl_render_image(self.raw, image, target, params) };
        snafu::ensure!(
            ok,
            OperationSnafu {
                what: "pl_render_image"
            }
        );
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // SAFETY: created by `pl_renderer_create`, destroyed once, before the
        // `gpu`/device it references.
        unsafe { pl::pl_renderer_destroy(&mut self.raw) };
    }
}
