use std::sync::Arc;

use iced::{Element, Task};

use crate::background::{BackgroundLook, BackgroundSource, background};

/// Messages emitted by the main screen.
///
/// The surface is currently passive, so there are none.
#[derive(Debug, Clone)]
pub enum MainMsg {}

/// State for the main library surface.
pub struct MainScreen {
    source: Arc<BackgroundSource>,
    look: BackgroundLook,
}

impl MainScreen {
    /// Builds the screen over `source`, with the default look.
    pub fn new(source: Arc<BackgroundSource>) -> Self {
        Self {
            source,
            look: BackgroundLook::default(),
        }
    }

    pub fn update(&mut self, message: MainMsg) -> Task<MainMsg> {
        match message {}
    }

    pub fn view(&self) -> Element<'_, MainMsg> {
        background(Arc::clone(&self.source), self.look).into()
    }
}
