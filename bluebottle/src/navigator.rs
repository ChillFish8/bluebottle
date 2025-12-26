use std::sync::atomic::{AtomicU32, Ordering};

use crate::storage;

static SCREEN: AtomicU32 = AtomicU32::new(0);
static NAVIGATOR_STATE_KEY: &str = "navigator_screen";

/// Returns the currently active screen.
pub fn active() -> ActiveScreen {
    let id = SCREEN.load(Ordering::Relaxed);
    ActiveScreen::from_id(id).unwrap_or_default()
}

/// Attempt to load the last navigation screen from the persisted state.
pub fn load_from_state() {
    let screen_bytes = storage::with_relaxed_state(move |state| {
        state
            .get_key_value(NAVIGATOR_STATE_KEY)
            .inspect_err(|err| tracing::error!(error = %err, "failed to fetch navigator key state"))
            .ok()
    });
    dbg!(&screen_bytes);

    let screen: ActiveScreen = screen_bytes
        .and_then(|bytes| Some(u32::from_le_bytes(bytes.try_into().ok()?)))
        .and_then(ActiveScreen::from_id)
        .unwrap_or_default();

    SCREEN.store(screen as u32, Ordering::Relaxed);
}

/// Navigate to a new screen.
pub fn navigate(screen: ActiveScreen) {
    SCREEN.store(screen as u32, Ordering::Relaxed);

    let bytes = (screen as u32).to_le_bytes();
    dbg!(&bytes);
    storage::submit_relaxed_state(move |state| {
        if let Err(err) = state.set_key_value(NAVIGATOR_STATE_KEY, &bytes) {
            tracing::error!(error = %err, "failed to set navigator state");
        }
    });
}

#[repr(u32)]
#[derive(
    Default, Copy, Clone, Debug, serde_derive::Serialize, serde_derive::Deserialize,
)]
/// What UI screen the app should be displaying.
pub enum ActiveScreen {
    #[default]
    /// View an existing media library.
    LibraryView = 0,
    /// The library being requested is still being prepared, show
    /// the user a loading screen for now.
    Loading = 1,
    /// The user has no libraries available, we should onboard
    /// them with the setup screen.
    Setup = 2,
    /// Select an existing media library (or add a new one.)
    LibrarySelect = 3,
    /// View the app settings.
    Settings = 4,
}

impl ActiveScreen {
    fn from_id(id: u32) -> Option<ActiveScreen> {
        match id {
            0 => Some(ActiveScreen::LibraryView),
            1 => Some(ActiveScreen::Loading),
            2 => Some(ActiveScreen::Setup),
            3 => Some(ActiveScreen::LibrarySelect),
            4 => Some(ActiveScreen::Settings),
            _ => None,
        }
    }
}
