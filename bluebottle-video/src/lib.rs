//! A libplacebo-rendered GStreamer video player toolkit for `bluebottle-window`.
//!
//! Decoded frames are run through libplacebo (mpv's `vo=gpu-next` engine) for
//! high-quality scaling, debanding, dithering and colour management, then
//! presented via a Vulkan swapchain onto the content surface beneath a
//! `bluebottle-window` overlay. Hardware-decoded frames are imported zero-copy
//! from dmabufs; everything else falls back to a system-memory upload.
//!
//! Linux/Wayland is implemented today; the [`platform`] seam isolates the
//! surface-creation differences so other platforms can be added later.

mod config;
mod error;
pub mod placebo;
pub mod platform;
mod player;
mod render;
pub mod sink;

pub use config::RenderPreset;
pub use error::Error;
pub use player::Player;
pub use sink::PlaceboSink;

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use placebo_sys as pl;

    use crate::placebo::{Device, Log, Renderer, SysmemUploader, packed8_plane_data};

    /// Headless smoke test: upload a synthetic RGBA frame and render it onto an
    /// offscreen target, exercising the device, renderer and sysmem upload path
    /// end to end. (The on-screen swapchain path is covered by the `player`
    /// example.)
    #[test]
    fn headless_render_pipeline() {
        let log = Log::new().expect("create log");
        let device = Device::headless(&log).expect("create headless device");
        let gpu = device.gpu();
        let renderer = Renderer::new(&log, gpu).expect("create renderer");

        const W: i32 = 64;
        const H: i32 = 64;
        let pixels = vec![0x80u8; (W * H * 4) as usize];

        let mut uploader = SysmemUploader::new(gpu);
        let data = packed8_plane_data(
            W,
            H,
            4,
            [0, 1, 2, 3],
            4,
            (W * 4) as usize,
            pixels.as_ptr() as *const c_void,
        );
        let plane = uploader.upload(&data).expect("upload plane");

        // Offscreen render target: a renderable RGBA8 texture.
        let caps =
            pl::pl_fmt_caps_PL_FMT_CAP_RENDERABLE | pl::pl_fmt_caps_PL_FMT_CAP_BLITTABLE;
        let format = unsafe {
            pl::pl_find_fmt(gpu.raw(), pl::pl_fmt_type_PL_FMT_UNORM, 4, 8, 0, caps)
        };
        assert!(!format.is_null(), "no renderable RGBA8 format");
        let target_params = pl::pl_tex_params {
            w: W,
            h: H,
            format,
            renderable: true,
            blit_dst: true,
            ..Default::default()
        };
        let mut target_tex = unsafe { pl::pl_tex_create(gpu.raw(), &target_params) };
        assert!(!target_tex.is_null(), "create target texture");

        let rgb = pl::pl_color_repr {
            sys: pl::pl_color_system_PL_COLOR_SYSTEM_RGB,
            ..Default::default()
        };

        let mut image = pl::pl_frame {
            num_planes: 1,
            repr: rgb,
            ..Default::default()
        };
        image.planes[0] = plane;

        let target_plane = pl::pl_plane {
            texture: target_tex,
            components: 4,
            component_mapping: [0, 1, 2, 3],
            ..Default::default()
        };
        let mut target = pl::pl_frame {
            num_planes: 1,
            repr: rgb,
            ..Default::default()
        };
        target.planes[0] = target_plane;

        // SAFETY: both frames and their textures belong to `gpu` and outlive the
        // call; default params are a valid `pl_render_params`.
        unsafe {
            let params = pl::pl_render_default_params;
            renderer
                .render(&image, &target, &params)
                .expect("render image");
            pl::pl_gpu_finish(gpu.raw());
            pl::pl_tex_destroy(gpu.raw(), &mut target_tex);
        }
    }
}
