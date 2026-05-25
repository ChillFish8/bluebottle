//! Tunable rendering quality, selecting between libplacebo's three curated
//! parameter sets. [`RenderPreset::HighQuality`] matches mpv's
//! `--profile=high-quality`.

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
    /// The libplacebo `pl_render_params` for this preset. These are copies of
    /// `extern const` sets whose inner pointers reference libplacebo's own
    /// static data, valid for the program's lifetime.
    pub(crate) fn to_params(self) -> pl::pl_render_params {
        unsafe {
            match self {
                RenderPreset::Fast => pl::pl_render_fast_params,
                RenderPreset::Standard => pl::pl_render_default_params,
                RenderPreset::HighQuality => pl::pl_render_high_quality_params,
            }
        }
    }
}
