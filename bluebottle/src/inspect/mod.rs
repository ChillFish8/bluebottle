mod shader;

use std::fmt;
use std::sync::Arc;

use bluebottle_ui::color;
use iced::widget::shader::Shader;
use iced::{Color, Length};

use self::shader::ScrimPrimitive;

/// The colour the scrim settles the blurred scene toward, outside the panel
/// (the look's tint coverage scales its alpha). The app background, so the wash
/// reads as a darkened tint of the palette rather than a flat near-black —
/// the role `rgba(24,20,16,.6)` plays over the lighter reference canvas.
const SCRIM_TINT: Color = color::BACKGROUND;

/// A snapshot of the rendered scene, captured when the inspect modal opens and
/// blurred as the scrim's backdrop. Held as tightly packed RGBA8 for GPU upload.
///
/// The pixels are uploaded once (the pipeline caches the GPU texture by `Arc`
/// identity) but kept here for the modal's lifetime; for a transient overlay the
/// retained copy isn't worth the interior mutability needed to drop it early.
pub struct SnapshotImage {
    /// Row-major RGBA8 (sRGB) pixels, `width * height * 4` bytes long.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl SnapshotImage {
    /// Builds a snapshot from an [`iced::window::Screenshot`].
    pub fn from_screenshot(screenshot: &iced::window::Screenshot) -> Self {
        Self {
            rgba: screenshot.rgba.to_vec(),
            width: screenshot.size.width,
            height: screenshot.size.height,
        }
    }
}

impl fmt::Debug for SnapshotImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnapshotImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.rgba.len())
            .finish()
    }
}

/// A full-bleed scrim widget over a scene `snapshot` blurred by `blur` and faded
/// in by `factor` (its overall alpha): a dark wash everywhere, and a near-solid
/// frosted pane within a centred rounded rect of `panel_size` / `corner_radius`
/// (all in logical pixels). `scrim_tint` is the outside wash's coverage in
/// `[0, 1]`; the panel is the app background shifted toward the primary colour
/// by `panel_shift` and laid over the blur at `panel_opacity`.
#[allow(clippy::too_many_arguments)]
pub fn scrim<Message>(
    snapshot: Arc<SnapshotImage>,
    blur: f32,
    panel_blur: f32,
    saturate: f32,
    scrim_tint: f32,
    panel_shift: f32,
    panel_opacity: f32,
    panel_size: (f32, f32),
    corner_radius: f32,
    factor: f32,
) -> Shader<Message, ScrimProgram> {
    // Panel colour: the app background eased toward the primary by `panel_shift`.
    let lerp = |from: f32, to: f32| from + panel_shift * (to - from);
    let (bg, primary) = (color::BACKGROUND, color::PRIMARY);
    let program = ScrimProgram {
        snapshot,
        blur,
        panel_blur,
        saturate,
        scrim_tint: [SCRIM_TINT.r, SCRIM_TINT.g, SCRIM_TINT.b, scrim_tint],
        panel_tint: [
            lerp(bg.r, primary.r),
            lerp(bg.g, primary.g),
            lerp(bg.b, primary.b),
            panel_opacity,
        ],
        panel_size: [panel_size.0, panel_size.1],
        corner_radius,
        factor,
    };
    Shader::new(program)
        .width(Length::Fill)
        .height(Length::Fill)
}

/// The [`shader::Program`](iced::widget::shader::Program) driving the scrim.
pub struct ScrimProgram {
    snapshot: Arc<SnapshotImage>,
    blur: f32,
    panel_blur: f32,
    saturate: f32,
    scrim_tint: [f32; 4],
    panel_tint: [f32; 4],
    panel_size: [f32; 2],
    corner_radius: f32,
    factor: f32,
}

impl<Message> iced::widget::shader::Program<Message> for ScrimProgram {
    type State = ();
    type Primitive = ScrimPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: iced::mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        ScrimPrimitive {
            snapshot: Arc::clone(&self.snapshot),
            blur: self.blur,
            panel_blur: self.panel_blur,
            saturate: self.saturate,
            scrim_tint: self.scrim_tint,
            panel_tint: self.panel_tint,
            panel_size: self.panel_size,
            corner_radius: self.corner_radius,
            factor: self.factor,
        }
    }
}
