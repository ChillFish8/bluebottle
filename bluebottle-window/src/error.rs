use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::client::ConnectError;
use smithay_client_toolkit::reexports::client::globals::{BindError, GlobalError};
use smithay_client_toolkit::shm::CreatePoolError;
use smithay_client_toolkit::shm::slot::CreateBufferError;
use snafu::Snafu;

/// An error that can occur while creating or running an overlay window.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    /// The background event loop thread could not be spawned.
    #[snafu(display("failed to spawn the overlay event loop thread"))]
    SpawnThread { source: std::io::Error },

    /// The event loop exited before the main surface was ready.
    #[snafu(display("the overlay event loop exited before reporting readiness"))]
    LoopExited,

    /// Connecting to the Wayland compositor failed.
    #[snafu(display("failed to connect to the Wayland compositor"))]
    Connect { source: ConnectError },

    /// Initialising the Wayland registry failed.
    #[snafu(display("failed to initialise the Wayland registry"))]
    Registry { source: GlobalError },

    /// A required Wayland global was not advertised by the compositor.
    #[snafu(display("required Wayland global unavailable: {what}"))]
    MissingGlobal {
        what: &'static str,
        source: BindError,
    },

    /// The shared-memory pool backing the overlay could not be created.
    #[snafu(display("failed to create the shared-memory pool"))]
    Pool { source: CreatePoolError },

    /// A shared-memory buffer could not be created.
    #[snafu(display("failed to create a shared-memory buffer"))]
    Buffer { source: CreateBufferError },

    /// The calloop event loop could not be created or failed while running.
    #[snafu(display("the overlay event loop failed"))]
    EventLoop { source: calloop::Error },

    /// The Wayland event source could not be registered with the event loop.
    #[snafu(display("failed to register the Wayland event source: {message}"))]
    EventLoopInsert { message: String },

    /// The Iced renderer/compositor could not be initialised on the surface.
    #[snafu(display("failed to initialise the Iced renderer: {message}"))]
    RendererInit { message: String },
}
