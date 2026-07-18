CREATE TABLE IF NOT EXISTS app_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO app_metadata (key, value)
VALUES ('schema_baseline', '1')
ON CONFLICT (key) DO NOTHING;
