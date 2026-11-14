-- Historical metadata is independent from imported originals and network sessions.
CREATE TABLE archive_platforms (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    manufacturer TEXT NOT NULL,
    year INTEGER,
    description TEXT NOT NULL
) STRICT;
CREATE TABLE archive_games (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    year INTEGER,
    developer TEXT NOT NULL,
    publisher TEXT NOT NULL,
    genre TEXT NOT NULL,
    description TEXT NOT NULL,
    history TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE TABLE archive_releases (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES archive_games(id),
    platform_id TEXT NOT NULL REFERENCES archive_platforms(id),
    region TEXT NOT NULL,
    languages TEXT NOT NULL,
    title_id TEXT NOT NULL
) STRICT;
CREATE INDEX archive_releases_game_id ON archive_releases(game_id);
CREATE INDEX archive_releases_platform_id ON archive_releases(platform_id);
CREATE TABLE archive_editions (
    id TEXT PRIMARY KEY,
    release_id TEXT NOT NULL REFERENCES archive_releases(id),
    name TEXT NOT NULL,
    medium TEXT NOT NULL
) STRICT;
CREATE INDEX archive_editions_release_id ON archive_editions(release_id);
CREATE TABLE archive_contents (
    id TEXT PRIMARY KEY,
    edition_id TEXT NOT NULL REFERENCES archive_editions(id),
    kind TEXT NOT NULL CHECK(kind IN ('version','update','dlc')),
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    build TEXT NOT NULL,
    release_date TEXT NOT NULL,
    format TEXT NOT NULL,
    base_id TEXT REFERENCES archive_contents(id),
    description TEXT NOT NULL,
    changelog TEXT NOT NULL,
    identifier TEXT NOT NULL ,
    CHECK ((kind='version' AND base_id IS NULL) OR (kind IN ('update','dlc') AND base_id IS NOT NULL)),
    CHECK(base_id IS NULL OR base_id <> id)
) STRICT;
CREATE INDEX archive_contents_edition_id ON archive_contents(edition_id);
CREATE INDEX archive_contents_base_id ON archive_contents(base_id);
CREATE TABLE archive_dependencies (
    content_id TEXT NOT NULL REFERENCES archive_contents(id),
    required_id TEXT NOT NULL REFERENCES archive_contents(id) ,
    PRIMARY KEY (content_id,required_id)
) STRICT;
CREATE INDEX archive_dependencies_content_id ON archive_dependencies(content_id);
CREATE INDEX archive_dependencies_required_id ON archive_dependencies(required_id);
CREATE TABLE archive_preservation_files (
    id TEXT PRIMARY KEY,
    content_id TEXT NOT NULL REFERENCES archive_contents(id),
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    size_bytes TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    item_id INTEGER REFERENCES collection_items(id) ON DELETE SET NULL,
    file_id INTEGER REFERENCES files(id) ON DELETE SET NULL
) STRICT;
CREATE INDEX archive_preservation_files_content_id ON archive_preservation_files(content_id);
CREATE INDEX archive_preservation_files_item_id ON archive_preservation_files(item_id);
CREATE INDEX archive_preservation_files_file_id ON archive_preservation_files(file_id);
CREATE TABLE archive_p2p_sources (
    id TEXT PRIMARY KEY,
    content_id TEXT NOT NULL REFERENCES archive_contents(id),
    magnet TEXT NOT NULL,
    license TEXT NOT NULL,
    attribution TEXT NOT NULL,
    authorized INTEGER NOT NULL CHECK(authorized IN (0,1))
) STRICT;
CREATE INDEX archive_p2p_sources_content_id ON archive_p2p_sources(content_id);
CREATE TABLE archive_assets (
    id TEXT PRIMARY KEY,
    platform_id TEXT REFERENCES archive_platforms(id),
    game_id TEXT REFERENCES archive_games(id),
    content_id TEXT REFERENCES archive_contents(id),
    role TEXT NOT NULL CHECK(role IN ('cover','banner','screenshot','console')),
    mime TEXT NOT NULL,
    bytes BLOB NOT NULL,
    source TEXT NOT NULL,
    license TEXT NOT NULL,
    attribution TEXT NOT NULL ,
    CHECK ((platform_id IS NOT NULL)+(game_id IS NOT NULL)+(content_id IS NOT NULL)=1)
) STRICT;
CREATE INDEX archive_assets_platform_id ON archive_assets(platform_id);
CREATE INDEX archive_assets_game_id ON archive_assets(game_id);
CREATE INDEX archive_assets_content_id ON archive_assets(content_id);
CREATE TABLE archive_catalogs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    digest TEXT NOT NULL UNIQUE,
    attribution TEXT NOT NULL,
    imported_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE TABLE archive_catalog_entries (
    catalog_id TEXT NOT NULL REFERENCES archive_catalogs(id),
    game_id TEXT NOT NULL REFERENCES archive_games(id) ,
    PRIMARY KEY (catalog_id,game_id)
) STRICT;
CREATE INDEX archive_catalog_entries_catalog_id ON archive_catalog_entries(catalog_id);
CREATE INDEX archive_catalog_entries_game_id ON archive_catalog_entries(game_id);
CREATE TABLE archive_collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL
) STRICT;
CREATE TABLE archive_collection_entries (
    collection_id TEXT NOT NULL REFERENCES archive_collections(id),
    game_id TEXT NOT NULL REFERENCES archive_games(id) ,
    PRIMARY KEY (collection_id,game_id)
) STRICT;
CREATE INDEX archive_collection_entries_collection_id ON archive_collection_entries(collection_id);
CREATE INDEX archive_collection_entries_game_id ON archive_collection_entries(game_id);
CREATE TABLE archive_preferences (
    game_id TEXT PRIMARY KEY REFERENCES archive_games(id),
    favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)),
    in_library INTEGER NOT NULL DEFAULT 0 CHECK(in_library IN (0,1)),
    visited_at TEXT
) STRICT;
CREATE INDEX archive_preferences_game_id ON archive_preferences(game_id);
CREATE TABLE archive_metadata_sources (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES archive_games(id),
    source TEXT NOT NULL,
    attribution TEXT NOT NULL,
    note TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX archive_metadata_sources_game_id ON archive_metadata_sources(game_id);
CREATE INDEX archive_games_title ON archive_games(title COLLATE NOCASE);
CREATE INDEX archive_games_year ON archive_games(year);
CREATE INDEX archive_games_updated ON archive_games(updated_at);
CREATE INDEX archive_releases_platform_game ON archive_releases(platform_id,game_id);
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-1','Nintendo Entertainment System','Nintendo','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-2','Super Nintendo','Nintendo','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-3','Nintendo 64','Nintendo','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-4','GameCube','Nintendo','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-5','Wii','Nintendo','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-6','Wii U','Nintendo','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-7','Nintendo Switch','Nintendo','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-8','PlayStation','Sony','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-9','PlayStation 2','Sony','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-10','PlayStation 3','Sony','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-11','PlayStation 4','Sony','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-12','PlayStation 5','Sony','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-13','Xbox','Microsoft','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-14','Xbox 360','Microsoft','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-15','Xbox One','Microsoft','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-16','Xbox Series X|S','Microsoft','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-17','Mega Drive','Sega','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-18','Saturn','Sega','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-19','Dreamcast','Sega','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-20','Windows','PC','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-21','Linux','PC','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-22','macOS','PC','');
INSERT INTO archive_platforms(id,name,manufacturer,description) VALUES ('platform-23','Retro & Others','Other','');
