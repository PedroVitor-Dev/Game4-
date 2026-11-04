-- Durable import request deduplication and restart recovery.
ALTER TABLE tasks ADD COLUMN request_id TEXT;
ALTER TABLE tasks ADD COLUMN selection_token TEXT;
ALTER TABLE tasks ADD COLUMN revision INTEGER NOT NULL DEFAULT 0 CHECK (revision >= 0);
ALTER TABLE tasks ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
UPDATE tasks SET updated_at = created_at;
CREATE UNIQUE INDEX tasks_request_id ON tasks(request_id) WHERE request_id IS NOT NULL;
CREATE TABLE import_sources (
    source_key TEXT PRIMARY KEY,
    item_id INTEGER NOT NULL UNIQUE REFERENCES collection_items(id) ON DELETE CASCADE
) STRICT;
