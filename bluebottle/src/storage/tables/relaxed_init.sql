BEGIN;
-- Holds onto RPC calls of backends with responses that
-- can be cached, this reduces wait time and traffic on the network.
CREATE TABLE IF NOT EXISTS backend_content_cache (
    backend_id TEXT,
    cache_key TEXT,
    content BLOB,
    updated_at BIGINT,
    expires_at BIGINT,
    PRIMARY KEY (backend_id, cache_key)
);
COMMIT;