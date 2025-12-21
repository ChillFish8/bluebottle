use serde::Serialize;
use snafu::ResultExt;

use crate::backends::BackendKind;

/// System state storage backed by an SQLite database.
pub struct DurableStateStorage {
    conn: rusqlite::Connection,
}

impl DurableStateStorage {
    /// Creates a new [DurableStateStorage] instance located within the data directory.
    pub(super) fn open() -> Result<Self, snafu::Whatever> {
        let conn = if cfg!(test) {
            rusqlite::Connection::open_in_memory()
                .whatever_context("open durable SQLite database")?
        } else {
            let paths = super::directory::paths();
            let durable_path = paths.data_dir().join("durable.sqlite");
            rusqlite::Connection::open(durable_path)
                .whatever_context("open durable SQLite database")?
        };

        conn.pragma_update(None, "journal_mode", "WAL")
            .whatever_context("update durable journal_mode pragma")?;
        conn.pragma_update(None, "synchronous", "FULL")
            .whatever_context("update durable synchronous pragma")?;

        let this = Self { conn };
        this.init_databases()?;
        Ok(this)
    }

    fn init_databases(&self) -> Result<(), snafu::Whatever> {
        static DURABLE_INIT_SQL: &str = include_str!("tables/durable_init.sql");

        tracing::info!("initializing database");

        self.conn
            .execute_batch(DURABLE_INIT_SQL)
            .whatever_context("initializing durable database")?;

        Ok(())
    }

    /// Persist the provided backend context to the state DB.
    pub fn save_backend_context(
        &self,
        backend_id: uuid::Uuid,
        backend: BackendKind,
        context: &impl Serialize,
    ) -> Result<(), snafu::Whatever> {
        let context =
            serde_json::to_value(context).whatever_context("Serialize context")?;

        self.conn
            .execute("INSERT INTO ", (backend_id, backend, context))
            .with_whatever_context(|_| {
                format!("insert backend ({backend_id}) context")
            })?;

        Ok(())
    }

    /// Retrieves all persisted backend context from the state.
    pub fn read_all_backend_context(
        &self,
    ) -> Result<Vec<(uuid::Uuid, BackendKind, serde_json::Value)>, snafu::Whatever> {
        let mut stmt = self
            .conn
            .prepare("SELECT backend_id, kind, context FROM backend_context;")
            .whatever_context("prepare context read")?;

        stmt.query_map((), |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .whatever_context("retrieve context rows")?
            .collect::<Result<Vec<_>, rusqlite::Error>>()
            .whatever_context("deserialize context rows")
    }
}
