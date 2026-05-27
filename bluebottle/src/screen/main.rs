//! The main library surface.
//!
//! Renders the full-bleed background: a blurred backdrop image, or the
//! procedural gradient fallback, with a small overlay for dialling in the look
//! by eye.

use std::sync::Arc;

use bluebottle_ui::color;
use iced::widget::{column, container, slider, stack, text};
use iced::{Color, Element, Task, border};

use crate::background::{BackgroundSource, Look, background};

/// Messages emitted by the main screen.
#[derive(Debug, Clone)]
pub enum MainMsg {
    /// The blur-radius slider moved.
    Blur(f32),
    /// The saturation slider moved.
    Saturate(f32),
    /// The image start-opacity slider moved.
    ImageOpacityStart(f32),
    /// The image end-opacity slider moved.
    ImageOpacityEnd(f32),
    /// The background start-opacity slider moved.
    BgOpacityStart(f32),
    /// The background end-opacity slider moved.
    BgOpacityEnd(f32),
    /// The image-fade slider moved.
    ImageFade(f32),
    /// The background-fade start slider moved.
    BgStart(f32),
    /// The background-fade end slider moved.
    BgEnd(f32),
    /// The hard-solid-background slider moved.
    BgSolid(f32),
    /// The vertical-focus slider moved.
    Focus(f32),
    /// The zoom slider moved.
    Zoom(f32),
}

/// State for the main library surface.
pub struct MainScreen {
    source: Arc<BackgroundSource>,
    look: Look,
}

impl MainScreen {
    /// Builds the screen over `source`, with the default look.
    pub fn new(source: Arc<BackgroundSource>) -> Self {
        Self {
            source,
            look: Look::default(),
        }
    }

    pub fn update(&mut self, message: MainMsg) -> Task<MainMsg> {
        match message {
            MainMsg::Blur(blur) => self.look.blur = blur,
            MainMsg::Saturate(saturate) => self.look.saturate = saturate,
            MainMsg::ImageOpacityStart(value) => self.look.image_opacity_start = value,
            MainMsg::ImageOpacityEnd(value) => self.look.image_opacity_end = value,
            MainMsg::BgOpacityStart(value) => self.look.bg_opacity_start = value,
            MainMsg::BgOpacityEnd(value) => self.look.bg_opacity_end = value,
            MainMsg::ImageFade(image_fade) => self.look.image_fade = image_fade,
            MainMsg::BgStart(bg_start) => self.look.bg_start = bg_start,
            MainMsg::BgEnd(bg_end) => self.look.bg_end = bg_end,
            MainMsg::BgSolid(bg_solid) => self.look.bg_solid = bg_solid,
            MainMsg::Focus(focus) => self.look.focus = focus,
            MainMsg::Zoom(zoom) => self.look.zoom = zoom,
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, MainMsg> {
        let backdrop = background(Arc::clone(&self.source), self.look);
        stack![backdrop, self.tuning_panel()].into()
    }

    /// A small overlay for dialling in the background look by eye.
    fn tuning_panel(&self) -> Element<'_, MainMsg> {
        let label = |value: String| text(value).size(14).color(color::TEXT_DEFAULT);

        let panel = column![
            label(format!("blur: {:.0}", self.look.blur)),
            slider(0.0..=120.0, self.look.blur, MainMsg::Blur).step(1.0),
            label(format!("saturate: {:.2}", self.look.saturate)),
            slider(0.0..=3.0, self.look.saturate, MainMsg::Saturate).step(0.05),
            label(format!(
                "image opacity start: {:.2}",
                self.look.image_opacity_start
            )),
            slider(
                0.0..=1.0,
                self.look.image_opacity_start,
                MainMsg::ImageOpacityStart
            )
            .step(0.01),
            label(format!(
                "image opacity end: {:.2}",
                self.look.image_opacity_end
            )),
            slider(
                0.0..=1.0,
                self.look.image_opacity_end,
                MainMsg::ImageOpacityEnd
            )
            .step(0.01),
            label(format!(
                "bg opacity start: {:.2}",
                self.look.bg_opacity_start
            )),
            slider(
                0.0..=1.0,
                self.look.bg_opacity_start,
                MainMsg::BgOpacityStart
            )
            .step(0.01),
            label(format!("bg opacity end: {:.2}", self.look.bg_opacity_end)),
            slider(0.0..=1.0, self.look.bg_opacity_end, MainMsg::BgOpacityEnd)
                .step(0.01),
            label(format!("image fade: {:.2}", self.look.image_fade)),
            slider(0.0..=1.0, self.look.image_fade, MainMsg::ImageFade).step(0.01),
            label(format!("bg start: {:.2}", self.look.bg_start)),
            slider(0.0..=1.0, self.look.bg_start, MainMsg::BgStart).step(0.01),
            label(format!("bg end: {:.2}", self.look.bg_end)),
            slider(0.0..=1.0, self.look.bg_end, MainMsg::BgEnd).step(0.01),
            label(format!("bg solid: {:.2}", self.look.bg_solid)),
            slider(0.0..=1.0, self.look.bg_solid, MainMsg::BgSolid).step(0.01),
            label(format!("focus: {:.2}", self.look.focus)),
            slider(0.0..=1.0, self.look.focus, MainMsg::Focus).step(0.01),
            label(format!("zoom: {:.2}", self.look.zoom)),
            slider(1.0..=2.0, self.look.zoom, MainMsg::Zoom).step(0.01),
        ]
        .spacing(8)
        .width(280);

        container(panel)
            .padding(16)
            .style(|_theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                border: border::rounded(8),
                ..container::Style::default()
            })
            .into()
    }
}
