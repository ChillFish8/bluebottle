//! A `wp_viewport` on the main surface that scales a 1×1 opaque-black backdrop
//! to the window size.
//!
//! In video mode the main surface is only the letterbox backdrop behind the
//! content subsurface. Reallocating a full-window shm buffer on every resize
//! made the slot pool grow (doubling each time) until the compositor rejected
//! it; stretching a single 1×1 buffer with a viewport avoids any per-resize
//! allocation. Both viewporter interfaces are eventless, so the handlers below
//! are empty — they exist only to satisfy the `Dispatch` bounds on binding the
//! global and creating the viewport.

use smithay_client_toolkit::globals::GlobalData;
use smithay_client_toolkit::reexports::client::{Connection, Dispatch, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::{
    self,
    WpViewport,
};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::{
    self,
    WpViewporter,
};

use super::state::State;

impl Dispatch<WpViewporter, GlobalData> for State {
    fn event(
        _state: &mut Self,
        _viewporter: &WpViewporter,
        _event: wp_viewporter::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wp_viewporter has no events.
    }
}

impl Dispatch<WpViewport, GlobalData> for State {
    fn event(
        _state: &mut Self,
        _viewport: &WpViewport,
        _event: wp_viewport::Event,
        _data: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wp_viewport has no events.
    }
}
