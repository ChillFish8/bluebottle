use std::path::Path;
use std::time::Duration;

use rusqlite::params;
use snafu::ResultExt;

use crate::backends::BackendId;

/// System state storage backed by an SQLite database.
pub struct RelaxedStateStorage {
    conn: rusqlite::Connection,
}

impl RelaxedStateStorage {
    /// Creates a new [RelaxedStateStorage] instance located within the data directory.
    pub(super) fn open() -> Result<Self, snafu::Whatever> {
        let paths = super::directory::paths();
        let relaxed_path = paths.data_dir().join("relaxed.sqlite");

        let conn = open_sqlite_connection(&relaxed_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .whatever_context("update relaxed journal_mode pragma")?;
        conn.pragma_update(None, "synchronous", "OFF")
            .whatever_context("update relaxed synchronous pragma")?;

        let this = Self { conn };
        this.init_databases()?;
        Ok(this)
    }

    fn init_databases(&self) -> Result<(), snafu::Whatever> {
        static RELAXED_INIT_SQL: &str = include_str!("tables/relaxed_init.sql");

        tracing::info!("initializing database");

        self.conn
            .execute_batch(RELAXED_INIT_SQL)
            .whatever_context("initializing relaxed database")?;

        Ok(())
    }

    /// Retrieve an existing cached content entry.
    pub(super) fn get_content_cache_entry(
        &self,
        backend_id: BackendId,
        path: &str,
    ) -> Result<(Vec<u8>, Duration), snafu::Whatever> {
        let sql = r#"
            SELECT content, expires_at
            FROM backend_content_cache
            WHERE backend_id = ? AND cache_key = ?;
        "#;

        let mut stmt = self
            .conn
            .prepare_cached(sql)
            .whatever_context("prepared backend content")?;

        let (content, expires_at) = stmt
            .query_row(params![backend_id, path], |row| {
                Ok((row.get(0)?, row.get::<_, i64>(1)?))
            })
            .whatever_context("get backend content")?;

        let expires_in = (super::now() - expires_at) as u64;

        Ok((content, Duration::from_millis(expires_in)))
    }

    /// Add a new content cache entry.
    pub(super) fn add_content_cache_entry(
        &self,
        backend_id: BackendId,
        path: &str,
        content: Vec<u8>,
        ttl: Duration,
    ) -> Result<(), snafu::Whatever> {
        let now = super::now();
        let expires_at = now + ttl.as_millis() as i64;

        let sql = r#"
            INSERT INTO backend_content_cache (
                backend_id,
                cache_key,
                content,
                updated_at,
                expires_at
            ) VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (backend_id, cache_key)
            DO UPDATE SET
                content = excluded.content,
                updated_at = excluded.updated_at,
                expires_at = excluded.expires_at;
        "#;

        let mut stmt = self
            .conn
            .prepare_cached(sql)
            .whatever_context("prepared backend content")?;

        stmt.insert(params![backend_id, path, content, now, expires_at])
            .whatever_context("insert backend content")?;

        Ok(())
    }

    /// Prune the content cache of any expired entries.
    pub(super) fn prune_content_cache(&self) -> Result<usize, snafu::Whatever> {
        let now = super::now();

        let mut stmt = self
            .conn
            .prepare_cached("DELETE FROM backend_content_cache WHERE expires_at <= ?;")
            .whatever_context("prepared backend content")?;

        let n = stmt
            .execute(params![now])
            .whatever_context("prune purge query")?;

        Ok(n)
    }

    /// Purge the content cache of all entries
    pub(super) fn purge_content_cache(&self) -> Result<(), snafu::Whatever> {
        let mut stmt = self
            .conn
            .prepare_cached("DELETE FROM backend_content_cache?;")
            .whatever_context("prepared backend content")?;

        stmt.execute(params![])
            .whatever_context("execute purge query")?;

        Ok(())
    }
}

fn open_sqlite_connection(
    relaxed_path: &Path,
) -> Result<rusqlite::Connection, snafu::Whatever> {
    if cfg!(test) {
        return rusqlite::Connection::open_in_memory()
            .whatever_context("open relaxed SQLite database");
    };

    match rusqlite::Connection::open(relaxed_path) {
        Ok(conn) => Ok(conn),
        Err(err) => {
            tracing::warn!(error = %err, "relaxed state could not be opened, truncating and retrying...");
            let _ = std::fs::remove_file(relaxed_path);
            rusqlite::Connection::open(relaxed_path)
                .whatever_context("failed to open relaxed state storage")
        },
    }
}
