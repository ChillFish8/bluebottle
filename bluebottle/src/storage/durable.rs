use snafu::ResultExt;

use crate::backends::BackendInitState;

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

    /// Persist the provided backend init state to storage.
    pub fn save_backend_init_state(
        &self,
        state: BackendInitState,
    ) -> Result<(), snafu::Whatever> {
        self.conn
            .execute("INSERT INTO backend_init_state (backend_id, kind, context) VALUES (?, ?, ?);", (state.id, state.kind, state.context))
            .with_whatever_context(|_| {
                format!("insert backend ({}) context", state.id)
            })?;

        Ok(())
    }

    /// Retrieves all persisted backend init state from the storage.
    pub fn read_all_backend_init_state(
        &self,
    ) -> Result<Vec<BackendInitState>, snafu::Whatever> {
        let mut stmt = self
            .conn
            .prepare("SELECT backend_id, kind, context FROM backend_init_state;")
            .whatever_context("prepare context read")?;

        stmt.query_map((), |row| {
            Ok(BackendInitState {
                id: row.get(0)?,
                kind: row.get(1)?,
                context: row.get(2)?,
            })
        })
        .whatever_context("retrieve context rows")?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .whatever_context("deserialize context rows")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::backends::BackendKind;

    #[test]
    fn test_single_backend_save_retrieve() {
        let input_init_state = BackendInitState {
            id: uuid::Uuid::now_v7(),
            kind: BackendKind::Jellyfin,
            context: json!({"test": 1234}),
        };
        let id = input_init_state.id;

        let storage = DurableStateStorage::open().unwrap();
        storage.save_backend_init_state(input_init_state).unwrap();

        let states = storage.read_all_backend_init_state().unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].id, id);
        assert_eq!(states[0].kind, BackendKind::Jellyfin);
        assert_eq!(states[0].context, json!({"test": 1234}));
    }

    #[test]
    fn test_multiple_backend_save_retrieve() {
        let input_init_state1 = BackendInitState {
            id: uuid::Uuid::now_v7(),
            kind: BackendKind::Jellyfin,
            context: json!({"test": 1234}),
        };
        let input_init_state2 = BackendInitState {
            id: uuid::Uuid::now_v7(),
            kind: BackendKind::Jellyfin,
            context: json!({"test": "other"}),
        };

        let storage = DurableStateStorage::open().unwrap();
        storage.save_backend_init_state(input_init_state1).unwrap();
        storage.save_backend_init_state(input_init_state2).unwrap();

        let states = storage.read_all_backend_init_state().unwrap();
        assert_eq!(states.len(), 2);
    }
}
