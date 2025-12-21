use std::sync::OnceLock;

use serde::Serialize;
use snafu::ResultExt;
use tokio::sync::{mpsc, oneshot};

use crate::backends::BackendKind;

type StateOp = Box<dyn FnOnce(&StateStorage) + Send>;
static STATE: OnceLock<mpsc::Sender<StateOp>> = OnceLock::new();

/// Initialises the global app state using the [DirectoryPaths](super::directory::DirectoryPaths)
/// configured.
pub fn init_state() -> Result<(), snafu::Whatever> {
    let storage = StateStorage::open().whatever_context("open SQLite storage state")?;

    let (tx, rx) = mpsc::channel(500);
    std::thread::Builder::new()
        .name("bluebottle-state-actor".into())
        .spawn(move || state_runner_thread(rx, storage))
        .expect("spawn state actor thread");

    STATE.set(tx).expect("state should not already be init");

    Ok(())
}

/// Gets a static reference to the global app state.
pub fn with_state<F, T>(op: F) -> T
where
    F: for<'a> FnOnce(&'a StateStorage) -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel();

    let op = move |state: &StateStorage| {
        let result = op(state);
        let _ = tx.send(result);
    };

    let sender = STATE.get().expect("state actor should be initialised");

    sender
        .blocking_send(Box::new(op))
        .expect("state actor shutdown panicked");

    rx.blocking_recv().expect("op panicked")
}

fn state_runner_thread(mut ops: mpsc::Receiver<StateOp>, state: StateStorage) {
    tracing::info!("state actor started");

    while let Some(access) = ops.blocking_recv() {
        access(&state)
    }

    tracing::warn!("state actor shut down");
}

/// System state storage backed by an SQLite database.
pub struct StateStorage {
    durable_conn: rusqlite::Connection,
    relaxed_conn: rusqlite::Connection,
}

impl StateStorage {
    /// Creates a new [StateStorage] instance located within the data directory.
    pub fn open() -> Result<Self, snafu::Whatever> {
        let paths = super::directory::paths();

        let durable_path = paths.data_dir().join("durable.sqlite");
        let relaxed_path = paths.data_dir().join("relaxed.sqlite");

        let durable_conn = rusqlite::Connection::open(durable_path)
            .whatever_context("open durable SQLite database")?;
        durable_conn
            .pragma_update(None, "journal_mode", "WAL")
            .whatever_context("update durable journal_mode pragma")?;
        durable_conn
            .pragma_update(None, "synchronous", "FULL")
            .whatever_context("update durable synchronous pragma")?;

        let relaxed_conn = match rusqlite::Connection::open(&relaxed_path) {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!(error = %err, "relaxed state could not be opened, truncating and retrying...");
                let _ = std::fs::remove_file(&relaxed_path);
                rusqlite::Connection::open(&relaxed_path)
                    .whatever_context("failed to open relaxed state storage")?
            },
        };

        relaxed_conn
            .pragma_update(None, "journal_mode", "WAL")
            .whatever_context("update relaxed journal_mode pragma")?;
        relaxed_conn
            .pragma_update(None, "synchronous", "OFF")
            .whatever_context("update relaxed synchronous pragma")?;

        let this = Self {
            durable_conn,
            relaxed_conn,
        };
        this.init_databases()?;
        Ok(this)
    }

    fn init_databases(&self) -> Result<(), snafu::Whatever> {
        static DURABLE_INIT_SQL: &str = include_str!("tables/durable_init.sql");
        static RELAXED_INIT_SQL: &str = include_str!("tables/relaxed_init.sql");

        tracing::info!("initializing databases");

        self.durable_conn
            .execute_batch(DURABLE_INIT_SQL)
            .whatever_context("initializing durable database")?;

        self.relaxed_conn
            .execute_batch(RELAXED_INIT_SQL)
            .whatever_context("initializing relaxed database")?;

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

        self.durable_conn
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
            .durable_conn
            .prepare("SELECT backend_id, kind, context FROM backend_context;")
            .whatever_context("prepare context read")?;

        stmt.query_map((), |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .whatever_context("retrieve context rows")?
            .collect::<Result<Vec<_>, rusqlite::Error>>()
            .whatever_context("deserialize context rows")
    }
}
