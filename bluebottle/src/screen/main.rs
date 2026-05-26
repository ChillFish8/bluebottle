//! The main library surface.
//!
//! Renders the full-bleed background: a blurred spotlight image, or the
//! procedural gradient fallback.

use std::sync::Arc;

use iced::{Element, Task};

use crate::background::{self, BackgroundSource, background};

/// Messages emitted by the main screen.
#[derive(Debug, Clone)]
pub enum MainMsg {}

/// State for the main library surface.
pub struct MainScreen {
    source: Arc<BackgroundSource>,
    blur: f32,
    saturate: f32,
}

impl MainScreen {
    /// Builds the screen over `source`, with the default blur and saturation.
    pub fn new(source: Arc<BackgroundSource>) -> Self {
        Self {
            source,
            blur: background::DEFAULT_BLUR,
            saturate: background::DEFAULT_SATURATE,
        }
    }

    pub fn update(&mut self, message: MainMsg) -> Task<MainMsg> {
        match message {}
    }

    pub fn view(&self) -> Element<'_, MainMsg> {
        background(Arc::clone(&self.source), self.blur, self.saturate).into()
    }
}
