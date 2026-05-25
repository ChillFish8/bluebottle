use placebo_sys as pl;

use super::log::Log;
use super::vulkan::Gpu;
use crate::error::{CreateSnafu, Error, OperationSnafu};

/// A libplacebo renderer: runs a `pl_frame` (the decoded image) through the
/// scaling / colour-management / dithering pipeline onto a target `pl_frame`
/// (the swapchain framebuffer).
///
/// Must be dropped before the [`super::Device`] whose [`Gpu`] it was created
/// with; the owning render context guarantees this by field order.
pub struct Renderer {
    raw: pl::pl_renderer,
}

impl Renderer {
    pub fn new(log: &Log, gpu: Gpu) -> Result<Self, Error> {
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
        unsafe { pl::pl_renderer_destroy(&mut self.raw) };
    }
}
