BEGIN;
-- Backend libraries and their context required to be re-created.
CREATE TABLE IF NOT EXISTS backend_init_state (
    backend_id TEXT,
    kind TEXT,
    context TEXT
);

-- User interactions pre-saved to a local DBs in order to apply later on.
CREATE TABLE IF NOT EXISTS user_interaction_backlog (
    event_id TEXT,
    backend_id TEXT,
    event TEXT,
    created_at BIGINT
);
COMMIT;