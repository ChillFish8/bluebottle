use std::ffi::c_void;

use placebo_sys as pl;

use super::vulkan::Gpu;
use crate::error::{Error, OperationSnafu, UnsupportedFormatSnafu};

/// An owned `pl_tex`, destroyed on drop. Used for dmabuf-imported planes.
pub struct Texture {
    tex: pl::pl_tex,
    gpu: pl::pl_gpu,
}

impl Texture {
    pub fn raw(&self) -> pl::pl_tex {
        self.tex
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { pl::pl_tex_destroy(self.gpu, &mut self.tex) };
    }
}

/// Persistent destination texture for system-memory plane uploads.
///
/// `pl_upload_plane` recreates the texture as the frame geometry/format
/// changes, so we keep one per source plane and re-upload into it each frame.
pub struct SysmemUploader {
    tex: pl::pl_tex,
    gpu: pl::pl_gpu,
}

impl SysmemUploader {
    pub fn new(gpu: Gpu) -> Self {
        Self {
            tex: std::ptr::null(),
            gpu: gpu.raw(),
        }
    }

    /// Upload `data` into the persistent texture and return the resulting plane,
    /// which borrows this uploader's texture until the next `upload`.
    pub fn upload(&mut self, data: &pl::pl_plane_data) -> Result<pl::pl_plane, Error> {
        let mut plane = pl::pl_plane::default();
        let ok =
            unsafe { pl::pl_upload_plane(self.gpu, &mut plane, &mut self.tex, data) };
        snafu::ensure!(
            ok,
            OperationSnafu {
                what: "pl_upload_plane"
            }
        );
        Ok(plane)
    }
}

impl Drop for SysmemUploader {
    fn drop(&mut self) {
        if !self.tex.is_null() {
            unsafe { pl::pl_tex_destroy(self.gpu, &mut self.tex) };
        }
    }
}

/// One dmabuf plane to import zero-copy.
pub struct DmabufPlane {
    pub fd: i32,
    pub offset: usize,
    pub stride: usize,
}

/// Import a single dmabuf plane as a sampleable `pl_tex`, zero-copy.
///
/// libplacebo does not take ownership of `plane.fd`; the caller must keep the
/// backing `gst::Buffer` alive for the texture's lifetime.
pub fn import_dmabuf(
    gpu: Gpu,
    fourcc: u32,
    modifier: u64,
    width: i32,
    height: i32,
    plane: &DmabufPlane,
) -> Result<Texture, Error> {
    let gpu = gpu.raw();
    let format = unsafe { pl::pl_find_fourcc(gpu, fourcc) };
    snafu::ensure!(
        !format.is_null(),
        UnsupportedFormatSnafu {
            message: format!("no libplacebo format for DRM fourcc {fourcc:#010x}"),
        }
    );

    let params = pl::pl_tex_params {
        w: width,
        h: height,
        format,
        sampleable: true,
        import_handle: pl::pl_handle_type_PL_HANDLE_DMA_BUF,
        shared_mem: pl::pl_shared_mem {
            handle: pl::pl_handle { fd: plane.fd },
            size: 0,
            offset: plane.offset,
            drm_format_mod: modifier,
            stride_w: plane.stride,
            stride_h: 0,
            plane: 0,
        },
        ..Default::default()
    };

    let tex = unsafe { pl::pl_tex_create(gpu, &params) };
    snafu::ensure!(
        !tex.is_null(),
        OperationSnafu {
            what: "pl_tex_create (dmabuf import)",
        }
    );

    Ok(Texture { tex, gpu })
}

/// Build a `pl_plane_data` for strided packed 8-bit data. `component_map` gives
/// the semantic (RGBA) index of each stored component in memory order, e.g.
/// BGRA-stored data maps to `[2, 1, 0, 3]`.
pub fn packed8_plane_data(
    width: i32,
    height: i32,
    components: usize,
    component_map: [i32; 4],
    pixel_stride: usize,
    row_stride: usize,
    pixels: *const c_void,
) -> pl::pl_plane_data {
    let mut size = [0; 4];
    let mut map = [0; 4];
    for index in 0..components {
        size[index] = 8;
        map[index] = component_map[index];
    }

    pl::pl_plane_data {
        type_: pl::pl_fmt_type_PL_FMT_UNORM,
        width,
        height,
        component_size: size,
        component_pad: [0; 4],
        component_map: map,
        pixel_stride,
        row_stride,
        pixels,
        ..Default::default()
    }
}
