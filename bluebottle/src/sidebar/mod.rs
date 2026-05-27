use std::sync::Arc;

use iced::widget::shader::Shader;

use crate::background::{
    BackgroundLook,
    BackgroundSource,
    CompositeKind,
    CompositeProgram,
    composite,
};

/// Marker selecting the sidebar's own composite pipeline instance. It composites
/// with the *same* shader and uniform as the main background (`background.wgsl`)
/// — a blurred image easing into the base colour — so the drawer shares the
/// page's style; the distinct type just gives it separate pipeline storage,
/// since iced shares one pipeline across every primitive of a type and the main
/// background renders in the same frame.
#[derive(Debug, Clone, Copy)]
pub struct SidebarKind;

impl CompositeKind for SidebarKind {
    const LABEL: &'static str = "sidebar";
}

/// The drawer background widget, over `source` (the poster it shows, or a solid
/// fill) under `look`.
pub fn sidebar<Message>(
    source: Arc<BackgroundSource>,
    look: BackgroundLook,
) -> Shader<Message, CompositeProgram<SidebarKind>> {
    composite(source, look)
}
