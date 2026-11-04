-- Portable collection identity is independent of local SQLite row identifiers.
CREATE TABLE collection_manifest_ids (
    item_id INTEGER PRIMARY KEY REFERENCES collection_items(id) ON DELETE CASCADE,
    manifest_id TEXT NOT NULL UNIQUE
) STRICT;
