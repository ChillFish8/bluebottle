use std::ffi::{CStr, c_void};
use std::os::raw::c_char;

use placebo_sys as pl;

use crate::error::{CreateSnafu, Error};

/// A libplacebo logger that forwards messages to `tracing`.
///
/// Almost every libplacebo object is created against a `pl_log`; this owns one
/// for the lifetime of the render path and routes libplacebo's diagnostics into
/// the host's tracing subscriber.
pub struct Log {
    raw: pl::pl_log,
}

impl Log {
    /// Create a logger forwarding at up to `PL_LOG_DEBUG` to `tracing`.
    pub fn new() -> Result<Self, Error> {
        let params = pl::pl_log_params {
            log_cb: Some(log_callback),
            log_priv: std::ptr::null_mut(),
            log_level: pl::pl_log_level_PL_LOG_DEBUG,
        };
        // SAFETY: `PL_API_VER` matches the bound headers; `params` outlives the
        // call (libplacebo copies it).
        let raw = unsafe { pl::pl_log_create(pl::PL_API_VER as i32, &params) };
        snafu::ensure!(!raw.is_null(), CreateSnafu { what: "pl_log" });
        Ok(Self { raw })
    }

    /// The raw `pl_log` handle, borrowed for the lifetime of `self`.
    pub fn raw(&self) -> pl::pl_log {
        self.raw
    }
}

impl Drop for Log {
    fn drop(&mut self) {
        // SAFETY: `raw` was created by `pl_log_create` and is destroyed once.
        unsafe { pl::pl_log_destroy(&mut self.raw) };
    }
}

/// libplacebo log callback: map the level and emit through `tracing`.
extern "C" fn log_callback(
    _log_priv: *mut c_void,
    level: pl::pl_log_level,
    msg: *const c_char,
) {
    if msg.is_null() {
        return;
    }
    // SAFETY: libplacebo passes a NUL-terminated, non-owning C string valid for
    // the duration of the call.
    let text = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    match level {
        pl::pl_log_level_PL_LOG_FATAL | pl::pl_log_level_PL_LOG_ERR => {
            tracing::error!(target: "libplacebo", "{text}");
        },
        pl::pl_log_level_PL_LOG_WARN => tracing::warn!(target: "libplacebo", "{text}"),
        pl::pl_log_level_PL_LOG_INFO => tracing::info!(target: "libplacebo", "{text}"),
        pl::pl_log_level_PL_LOG_DEBUG => tracing::debug!(target: "libplacebo", "{text}"),
        _ => tracing::trace!(target: "libplacebo", "{text}"),
    }
}
