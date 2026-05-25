//! A snapshot of what the player is currently playing and rendering, for a
//! "stats for nerds"-style debug overlay.
//!
//! [`MediaStats`] combines two views: per-track container/decoder info pulled
//! from `playbin` ([`VideoStats`], [`AudioStats`], [`SubtitleStats`]) and the
//! actual render path reported by the sink ([`RenderStats`]). Fields the source
//! does not report are left `None`.

use crate::config::RenderPreset;

/// A full snapshot of the active streams and the render pipeline.
#[derive(Clone, Debug, Default)]
pub struct MediaStats {
    pub video: Option<VideoStats>,
    pub audio: Option<AudioStats>,
    pub subtitle: SubtitleStats,
    pub render: Option<RenderStats>,
}

/// The selected video stream as `playbin` sees it (container/decoder view).
#[derive(Clone, Debug)]
pub struct VideoStats {
    pub codec: Option<String>,
    pub width: u32,
    pub height: u32,
    pub framerate: Option<f64>,
    pub bitrate: Option<u32>,
}

/// The selected audio stream as `playbin` sees it.
#[derive(Clone, Debug)]
pub struct AudioStats {
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub bitrate: Option<u32>,
    pub language: Option<String>,
    /// Index of the selected audio track, and how many are available.
    pub track: i32,
    pub track_count: i32,
}

/// The subtitle/text tracks. `track` is `-1` when none is active.
#[derive(Clone, Debug)]
pub struct SubtitleStats {
    pub track: i32,
    pub track_count: i32,
    pub language: Option<String>,
}

impl Default for SubtitleStats {
    fn default() -> Self {
        Self {
            track: -1,
            track_count: 0,
            language: None,
        }
    }
}

/// What the libplacebo sink is actually doing with the frames.
#[derive(Clone, Debug)]
pub struct RenderStats {
    /// Negotiated pixel format, e.g. `"P01010le"`.
    pub format: String,
    pub width: u32,
    pub height: u32,
    /// Whether frames take the zero-copy dmabuf import path (vs a sysmem upload).
    pub zero_copy: bool,
    /// Colour summary with an HDR flag, e.g. `"BT.2020 / PQ (HDR)"`.
    pub color: Option<String>,
    pub preset: RenderPreset,
    /// Frames presented to the swapchain so far.
    pub frames_presented: u64,
    /// Frames skipped because the surface was unavailable (not upstream QoS
    /// drops).
    pub frames_skipped: u64,
    /// Measured present rate, capped by the FIFO swapchain / display refresh.
    pub fps: f64,
}
