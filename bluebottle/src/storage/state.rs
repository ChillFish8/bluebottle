use std::sync::OnceLock;

use snafu::ResultExt;
use tokio::sync::{mpsc, oneshot};

use super::durable::DurableStateStorage;
use super::relaxed::RelaxedStateStorage;

type StateOp<S> = Box<dyn FnOnce(&S) + Send>;

static DURABLE_STATE: OnceLock<mpsc::Sender<StateOp<DurableStateStorage>>> =
    OnceLock::new();
static RELAXED_STATE: OnceLock<mpsc::Sender<StateOp<RelaxedStateStorage>>> =
    OnceLock::new();

/// Initialises the global app state using the [DirectoryPaths](super::directory::DirectoryPaths)
/// configured.
pub fn init_state() -> Result<(), snafu::Whatever> {
    let durable = DurableStateStorage::open()
        .whatever_context("open SQLite durable storage state")?;
    let relaxed = RelaxedStateStorage::open()
        .whatever_context("open SQLite relaxed storage state")?;

    let (durable_tx, durable_rx) = mpsc::channel(500);
    let (relaxed_tx, relaxed_rx) = mpsc::channel(500);

    std::thread::Builder::new()
        .name("bluebottle-durable-state-actor".into())
        .spawn(move || state_runner_thread(durable_rx, durable))
        .expect("spawn state actor thread");

    std::thread::Builder::new()
        .name("bluebottle-relaxed-state-actor".into())
        .spawn(move || state_runner_thread(relaxed_rx, relaxed))
        .expect("spawn state actor thread");

    DURABLE_STATE
        .set(durable_tx)
        .expect("state should not already be init");

    RELAXED_STATE
        .set(relaxed_tx)
        .expect("state should not already be init");

    Ok(())
}

/// Gets a static reference to the global durable app state.
pub fn with_durable_state<F, T>(op: F) -> T
where
    F: for<'a> FnOnce(&'a DurableStateStorage) -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel();

    let op = move |state: &DurableStateStorage| {
        let result = op(state);
        let _ = tx.send(result);
    };

    let sender = DURABLE_STATE
        .get()
        .expect("state actor should be initialised");

    sender
        .blocking_send(Box::new(op))
        .expect("state actor shutdown panicked");

    rx.blocking_recv().expect("op panicked")
}

/// Gets a static reference to the global relaxed app state.
pub fn with_relaxed_state<F, T>(op: F) -> T
where
    F: for<'a> FnOnce(&'a RelaxedStateStorage) -> T + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel();

    let op = move |state: &RelaxedStateStorage| {
        let result = op(state);
        let _ = tx.send(result);
    };

    let sender = RELAXED_STATE
        .get()
        .expect("state actor should be initialised");

    sender
        .blocking_send(Box::new(op))
        .expect("state actor shutdown panicked");

    rx.blocking_recv().expect("op panicked")
}

fn state_runner_thread<S>(mut ops: mpsc::Receiver<StateOp<S>>, state: S) {
    tracing::info!("state actor started");

    while let Some(access) = ops.blocking_recv() {
        access(&state)
    }

    tracing::warn!("state actor shut down");
}
