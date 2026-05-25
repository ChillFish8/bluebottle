//! Raw FFI bindings to the system [libplacebo](https://libplacebo.org/) (API
//! v7 / `PL_API_VER` 360), generated with bindgen at build time.
//!
//! This crate is the unsafe foundation under `bluebottle-video`'s safe
//! `placebo` wrapper; application code should not use it directly. Vulkan
//! handle types (`VkInstance`, `VkSurfaceKHR`, ...) are emitted as transitive
//! dependencies of the libplacebo Vulkan signatures and are layout-compatible
//! with the corresponding `ash` handles.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// `pl_log_create` is defined as an ABI-versioned glue macro
/// (`pl_log_create_<PL_API_VER>`), so the preprocessor — and therefore bindgen
/// — emits the symbol under its versioned name. Re-export it under the stable
/// name callers expect.
pub use self::pl_log_create_360 as pl_log_create;

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: proves the crate links against libplacebo and the ABI of the
    /// versioned entry point matches by creating and destroying a logger.
    #[test]
    fn create_and_destroy_log() {
        // SAFETY: `PL_API_VER` matches the headers we bound; a NULL params
        // pointer selects `pl_log_default_params`, and `pl_log_destroy` takes a
        // pointer to the handle, nulling it.
        unsafe {
            let mut log = pl_log_create(PL_API_VER as i32, std::ptr::null());
            assert!(!log.is_null(), "pl_log_create returned NULL");
            pl_log_destroy(&mut log);
            assert!(log.is_null(), "pl_log_destroy did not clear the handle");
        }
    }
}
