use serde_json::Value;

mod http;
pub mod jellyfin;

/// The backend provides access to media information required by the Bluebottle UI.
pub trait Backend {}

/// The backend trait for initialising the backend from a persisted state.
pub trait BackendInit: Sized {
    /// Load the backend from some persisted context state.
    fn from_context(context: Value) -> Result<Self, snafu::Whatever>;
}
