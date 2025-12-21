BEGIN;
CREATE TABLE IF NOT EXISTS backend_context (
    backend_id TEXT,
    kind TEXT,
    context TEXT
);
COMMIT;