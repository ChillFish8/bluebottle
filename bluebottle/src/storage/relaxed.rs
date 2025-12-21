use std::path::Path;

use snafu::ResultExt;

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
