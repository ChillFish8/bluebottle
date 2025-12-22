//! Caches content responses from backends to minimise network requests.

use std::time::Duration;

use crate::backends::BackendId;

const DEFAULT_TTL: Duration = Duration::from_secs(7 * 24 * 3600); // 7 days

/// A retrieved cache entry.
pub struct CacheEntry<T> {
    /// The cached value.
    pub value: T,
    /// How long until the entry is expired.
    pub expires_in: Duration,
}

/// Attempt to get a cached response from the cache.
pub fn try_get<T>(backend_id: BackendId, path: &str) -> Option<CacheEntry<T>>
where
    T: serde::de::DeserializeOwned,
{
    let cache_key = blake3::hash(path.as_bytes()).to_hex();
    let content = super::with_relaxed_state(move |state| {
        state.get_content_cache_entry(backend_id, &cache_key).ok()
    });

    content.and_then(|(buffer, expires_in)| {
        let value = rmp_serde::from_slice(&buffer).ok()?;
        Some(CacheEntry { value, expires_in })
    })
}

/// Insert a new content response into the cache.
pub fn insert<T>(backend_id: BackendId, path: &str, content: &T, ttl: Option<Duration>)
where
    T: serde::Serialize,
{
    let ttl = ttl.unwrap_or(DEFAULT_TTL);
    let content = rmp_serde::to_vec(content).unwrap();
    let cache_key = blake3::hash(path.as_bytes()).to_hex();
    super::with_relaxed_state(move |state| {
        if let Err(err) =
            state.add_content_cache_entry(backend_id, &cache_key, content, ttl)
        {
            tracing::error!(error = %err, "failed to content cache");
        }
    });
}

/// Prune any cache entries that have expired past their TTL.
///
/// Returns the number of rows pruned.
pub fn prune() -> usize {
    super::with_relaxed_state(move |state| {
        state.prune_content_cache().unwrap_or_else(|err| {
            tracing::error!(error = %err, "failed to prune content cache");
            0
        })
    })
}

/// Delete all content in the cache.
pub fn purge() {
    super::with_relaxed_state(move |state| {
        state.purge_content_cache().unwrap_or_else(|err| {
            tracing::error!(error = %err, "failed to purge content cache");
        })
    })
}
