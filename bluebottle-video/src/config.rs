//! Tunable rendering quality.
//!
//! libplacebo ships three curated parameter sets that span the
//! quality/performance spectrum; [`RenderPreset`] selects between them. The
//! [`RenderPreset::HighQuality`] set matches mpv's `--profile=high-quality`
//! defaults (EWA Lanczos scaling, error-diffusion dithering, debanding), which
//! is what "matching mpv's rendering quality" means in practice. The
//! [`RenderPreset::Standard`] default already enables 10-bit dithering and
//! good scaling; [`RenderPreset::Fast`] trades quality for throughput.

use placebo_sys as pl;

/// A libplacebo render-quality preset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderPreset {
    /// Cheapest path: bilinear scaling, minimal post-processing.
    Fast,
    /// libplacebo's balanced defaults (10-bit dithering, decent scaling).
    #[default]
    Standard,
    /// Highest quality: EWA Lanczos, debanding, error-diffusion dither —
    /// equivalent to mpv's high-quality profile.
    HighQuality,
}

impl RenderPreset {
    /// The libplacebo `pl_render_params` for this preset.
    ///
    /// These are copies of libplacebo's `extern const` parameter sets; the
    /// pointers they contain reference libplacebo's own static data and remain
    /// valid for the program's lifetime.
    pub(crate) fn to_params(self) -> pl::pl_render_params {
        // SAFETY: reading libplacebo's `extern const` parameter globals (Copy).
        unsafe {
            match self {
                RenderPreset::Fast => pl::pl_render_fast_params,
                RenderPreset::Standard => pl::pl_render_default_params,
                RenderPreset::HighQuality => pl::pl_render_high_quality_params,
            }
        }
    }
}
