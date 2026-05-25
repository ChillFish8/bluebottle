use snafu::Snafu;

/// Errors produced while building or driving the libplacebo render path.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    /// A libplacebo object could not be created (it returned NULL). `what`
    /// names the object, e.g. `"pl_vulkan"` or `"pl_renderer"`.
    #[snafu(display("failed to create libplacebo {what}"))]
    Create { what: &'static str },

    /// A libplacebo call that returns success/failure reported failure.
    #[snafu(display("libplacebo operation failed: {what}"))]
    Operation { what: &'static str },

    /// Vulkan surface creation failed for the current platform.
    #[snafu(display("failed to create a Vulkan surface: {message}"))]
    Surface { message: String },

    /// A frame arrived in a pixel format the renderer cannot import.
    #[snafu(display("unsupported frame format: {message}"))]
    UnsupportedFormat { message: String },

    /// The running platform has no implemented render backend.
    #[snafu(display("video rendering is not supported on this platform"))]
    UnsupportedPlatform,

    /// A GStreamer operation failed.
    #[snafu(display("gstreamer error: {message}"))]
    Gstreamer { message: String },
}
