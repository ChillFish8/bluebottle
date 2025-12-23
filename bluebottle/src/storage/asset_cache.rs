use std::path::PathBuf;
use std::str::FromStr;
use std::time::UNIX_EPOCH;

use arrayvec::ArrayString;
use snafu::ResultExt;

type AssetId = ArrayString<64>;

/// Try retrieve a cached entry with the given path.
pub fn try_get(path: &str) -> Option<Vec<u8>> {
    let asset_id: AssetId = blake3::hash(path.as_bytes()).to_hex();
    let cache_path = super::directory::paths().asset_cache_dir();
    let path = cache_path.join(asset_id);
    std::fs::read(&path).ok()
}

/// Try insert a new asset into the cache directory.
pub fn insert(path: &str, data: &[u8]) {
    let asset_id: AssetId = blake3::hash(path.as_bytes()).to_hex();
    let cache_path = super::directory::paths().asset_cache_dir();
    let path = cache_path.join(asset_id);
    if let Err(e) = std::fs::write(path, data) {
        tracing::warn!(error = %e, "failed to write asset to cache");
    }
}

/// Prune the cache to be up to the given capacity.
///
/// Returns the number of entries removed.
pub fn prune_to(target_size: u64) -> Result<usize, snafu::Whatever> {
    let mut all_entries = list_all_entries()?;
    all_entries.sort_unstable_by_key(|entry| access_time(&entry.metadata));

    let total_size: u64 = all_entries.iter().map(|entry| entry.metadata.len()).sum();
    let delta_size = total_size.saturating_sub(target_size);
    if delta_size == 0 {
        return Ok(0);
    }

    let mut removed = 0;
    let mut bytes_freed = 0;
    for entry in all_entries {
        if bytes_freed > delta_size {
            break;
        }

        tracing::debug!(asset_id = %entry.id, "removing cached asset");

        match std::fs::remove_file(&entry.path) {
            Ok(_) => {
                removed += 1;
                bytes_freed += entry.metadata.len();
            },
            Err(err) => {
                tracing::warn!(error = %entry.id, err = %err, "failed to remove cached asset");
            },
        }
    }

    Ok(removed)
}

/// Returns the total size of the cached entries.
pub fn usage() -> Result<u64, snafu::Whatever> {
    let total = list_all_entries()?
        .into_iter()
        .map(|entry| entry.metadata.len())
        .sum();
    Ok(total)
}

/// List all entries within the cache along with their metadata.
fn list_all_entries() -> Result<Vec<AssetEntry>, snafu::Whatever> {
    let cache_directory = super::directory::paths().asset_cache_dir();
    let entries = std::fs::read_dir(cache_directory)
        .whatever_context("failed to read cache directory")?;

    let mut all_entries = Vec::new();
    for file in entries {
        let entry = match file {
            Ok(entry) => entry,
            Err(err) => {
                tracing::warn!(error = %err, "cannot get entry info");
                continue;
            },
        };

        let path = entry.path();
        let file_name = entry.file_name();
        let id = match AssetId::from_str(&file_name.to_string_lossy()) {
            Ok(asset_id) => asset_id,
            Err(_) => {
                tracing::warn!(file_name = ?file_name, "invalid cache file name found");
                continue;
            },
        };

        let metadata = entry
            .metadata()
            .whatever_context("failed to get cache entry metadata")?;

        if metadata.is_dir() {
            continue;
        }

        all_entries.push(AssetEntry { id, metadata, path });
    }

    Ok(all_entries)
}

struct AssetEntry {
    id: AssetId,
    metadata: std::fs::Metadata,
    path: PathBuf,
}

fn access_time(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .accessed()
        .map(|time| time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let tmp_dir = tempfile::tempdir().unwrap();
        crate::storage::directory::init_paths(Some(tmp_dir.path().to_path_buf()))
            .unwrap();

        insert("test1.txt", b"hello world 1");
        insert("test2.txt", b"hello world 2");

        let content1 = try_get("test1.txt").unwrap();
        let content2 = try_get("test2.txt").unwrap();
        assert_eq!(content1, b"hello world 1");
        assert_eq!(content2, b"hello world 2");
    }

    #[test]
    fn test_size_count() {
        let tmp_dir = tempfile::tempdir().unwrap();
        crate::storage::directory::init_paths(Some(tmp_dir.path().to_path_buf()))
            .unwrap();

        insert("test1.txt", b"hello world 1");
        insert("test2.txt", b"hello world 2");

        let total_size = usage().unwrap();
        assert_eq!(total_size, 26);
    }

    #[test]
    fn test_prune_to() {
        let tmp_dir = tempfile::tempdir().unwrap();
        crate::storage::directory::init_paths(Some(tmp_dir.path().to_path_buf()))
            .unwrap();

        insert("test1.txt", b"hello world 1");
        std::thread::sleep(std::time::Duration::from_millis(200));
        insert("test2.txt", b"hello world 2");

        let total_size = usage().unwrap();
        assert_eq!(total_size, 26);

        let n_removed = prune_to(14).unwrap();
        assert_eq!(n_removed, 1);

        assert!(try_get("test1.txt").is_none());
        assert!(try_get("test2.txt").is_some());
    }
}
