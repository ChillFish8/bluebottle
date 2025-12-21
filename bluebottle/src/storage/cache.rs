use std::path::PathBuf;

/// Cache assets and images locally on disk rather than fetching from the server.
pub struct AssetProxyCache {
    cache_directory: PathBuf,
}

impl AssetProxyCache {
    /// Try retrieve a cached entry with the given path.
    pub fn try_get_path(&self, path: &str) -> Option<Vec<u8>> {
        let digest = blake3::hash(path.as_bytes()).to_hex();
        let path = self.cache_directory.join(digest);
        std::fs::read(&path).ok()
    }

    /// Try insert a new asset into the cache directory.
    pub fn insert(&self, path: &str, data: &[u8]) {
        let digest = blake3::hash(path.as_bytes()).to_hex();
        let path = self.cache_directory.join(digest);
        if let Err(e) = std::fs::write(path, data) {
            tracing::warn!(error = %e, "failed to write asset to cache");
            return;
        }
    }
}
