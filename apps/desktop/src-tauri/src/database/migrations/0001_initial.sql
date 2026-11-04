-- Content identity is independent of file locations and editorial metadata.
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    sha256 TEXT NOT NULL CHECK (length(sha256) = 64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (sha256, size_bytes)
) STRICT;

-- Verification belongs to a physical copy: two copies can have different health.
CREATE TABLE file_locations (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE RESTRICT,
    path TEXT NOT NULL CHECK (length(trim(path)) > 0),
    path_key TEXT NOT NULL UNIQUE CHECK (length(path_key) > 0),
    verification_status TEXT NOT NULL DEFAULT 'unverified'
        CHECK (verification_status IN ('unverified', 'verified', 'changed', 'missing', 'inaccessible')),
    last_verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
CREATE INDEX file_locations_file_id ON file_locations(file_id);

CREATE TABLE collection_items (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    platform TEXT,
    edition TEXT,
    language TEXT,
    declared_source TEXT,
    declared_license TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE collection_item_files (
    item_id INTEGER NOT NULL REFERENCES collection_items(id) ON DELETE CASCADE,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE RESTRICT,
    relative_path TEXT NOT NULL CHECK (length(trim(relative_path)) > 0),
    PRIMARY KEY (item_id, relative_path)
) STRICT;
CREATE INDEX collection_item_files_file_id ON collection_item_files(file_id);

CREATE TABLE tasks (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('import', 'verify')),
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled', 'interrupted')),
    processed_count INTEGER NOT NULL DEFAULT 0 CHECK (processed_count >= 0),
    total_count INTEGER CHECK (total_count IS NULL OR total_count >= processed_count),
    error_code TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    started_at TEXT,
    finished_at TEXT
) STRICT;
CREATE INDEX tasks_status ON tasks(status);
