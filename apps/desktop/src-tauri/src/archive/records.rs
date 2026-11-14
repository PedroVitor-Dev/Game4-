use super::*;
use rusqlite::{Connection, params};
impl ArchivePlatform {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_platforms", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q = c
            .prepare(
                "SELECT id,name,manufacturer,year,description FROM archive_platforms LIMIT 100001",
            )
            .map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                name: r.get(1)?,
                manufacturer: r.get(2)?,
                year: r.get(3)?,
                description: r.get(4)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_platforms(id,name,manufacturer,year,description) VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,manufacturer=excluded.manufacturer,year=excluded.year,description=excluded.description",params![self.id,self.name,self.manufacturer,self.year,self.description,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveGame {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_games", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT id,title,year,developer,publisher,genre,description,history,created_at,updated_at FROM archive_games LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                title: r.get(1)?,
                year: r.get(2)?,
                developer: r.get(3)?,
                publisher: r.get(4)?,
                genre: r.get(5)?,
                description: r.get(6)?,
                history: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_games(id,title,year,developer,publisher,genre,description,history,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET title=excluded.title,year=excluded.year,developer=excluded.developer,publisher=excluded.publisher,genre=excluded.genre,description=excluded.description,history=excluded.history,created_at=excluded.created_at,updated_at=excluded.updated_at",params![self.id,self.title,self.year,self.developer,self.publisher,self.genre,self.description,self.history,self.created_at,self.updated_at,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveRelease {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_releases", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT id,game_id,platform_id,region,languages,title_id FROM archive_releases LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                game_id: r.get(1)?,
                platform_id: r.get(2)?,
                region: r.get(3)?,
                languages: r.get(4)?,
                title_id: r.get(5)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_releases(id,game_id,platform_id,region,languages,title_id) VALUES (?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET game_id=excluded.game_id,platform_id=excluded.platform_id,region=excluded.region,languages=excluded.languages,title_id=excluded.title_id",params![self.id,self.game_id,self.platform_id,self.region,self.languages,self.title_id,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveEdition {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_editions", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q = c
            .prepare("SELECT id,release_id,name,medium FROM archive_editions LIMIT 100001")
            .map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                release_id: r.get(1)?,
                name: r.get(2)?,
                medium: r.get(3)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_editions(id,release_id,name,medium) VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET release_id=excluded.release_id,name=excluded.name,medium=excluded.medium",params![self.id,self.release_id,self.name,self.medium,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveContent {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_contents", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT id,edition_id,kind,name,version,build,release_date,format,base_id,description,changelog,identifier FROM archive_contents LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                edition_id: r.get(1)?,
                kind: r.get(2)?,
                name: r.get(3)?,
                version: r.get(4)?,
                build: r.get(5)?,
                release_date: r.get(6)?,
                format: r.get(7)?,
                base_id: r.get(8)?,
                description: r.get(9)?,
                changelog: r.get(10)?,
                identifier: r.get(11)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_contents(id,edition_id,kind,name,version,build,release_date,format,base_id,description,changelog,identifier) VALUES (?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET edition_id=excluded.edition_id,kind=excluded.kind,name=excluded.name,version=excluded.version,build=excluded.build,release_date=excluded.release_date,format=excluded.format,base_id=excluded.base_id,description=excluded.description,changelog=excluded.changelog,identifier=excluded.identifier",params![self.id,self.edition_id,self.kind,self.name,self.version,self.build,self.release_date,self.format,self.base_id,self.description,self.changelog,self.identifier,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveDependency {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_dependencies", [], |r| {
                r.get(0)
            })
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q = c
            .prepare("SELECT content_id,required_id FROM archive_dependencies LIMIT 100001")
            .map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                content_id: r.get(0)?,
                required_id: r.get(1)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_dependencies(content_id,required_id) VALUES (?,?) ON CONFLICT(content_id,required_id) DO NOTHING",params![self.content_id,self.required_id,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveFile {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_preservation_files", [], |r| {
                r.get(0)
            })
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT id,content_id,name,format,size_bytes,sha256,item_id,file_id FROM archive_preservation_files LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                content_id: r.get(1)?,
                name: r.get(2)?,
                format: r.get(3)?,
                size_bytes: r.get(4)?,
                sha256: r.get(5)?,
                item_id: r.get::<_, Option<i64>>(6)?.map(|v| v.to_string()),
                file_id: r.get::<_, Option<i64>>(7)?.map(|v| v.to_string()),
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_preservation_files(id,content_id,name,format,size_bytes,sha256,item_id,file_id) VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET content_id=excluded.content_id,name=excluded.name,format=excluded.format,size_bytes=excluded.size_bytes,sha256=excluded.sha256,item_id=excluded.item_id,file_id=excluded.file_id",params![self.id,self.content_id,self.name,self.format,self.size_bytes,self.sha256,self.item_id.as_ref().map(|v|v.parse::<i64>()).transpose().map_err(|_|invalid())?,self.file_id.as_ref().map(|v|v.parse::<i64>()).transpose().map_err(|_|invalid())?,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveSource {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_p2p_sources", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT id,content_id,magnet,license,attribution,authorized FROM archive_p2p_sources LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                content_id: r.get(1)?,
                magnet: r.get(2)?,
                license: r.get(3)?,
                attribution: r.get(4)?,
                authorized: r.get(5)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_p2p_sources(id,content_id,magnet,license,attribution,authorized) VALUES (?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET content_id=excluded.content_id,magnet=excluded.magnet,license=excluded.license,attribution=excluded.attribution,authorized=excluded.authorized",params![self.id,self.content_id,self.magnet,self.license,self.attribution,self.authorized,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveAsset {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_assets", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT id,platform_id,game_id,content_id,role,mime,bytes,source,license,attribution FROM archive_assets LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                platform_id: r.get(1)?,
                game_id: r.get(2)?,
                content_id: r.get(3)?,
                role: r.get(4)?,
                mime: r.get(5)?,
                bytes: r.get(6)?,
                source: r.get(7)?,
                license: r.get(8)?,
                attribution: r.get(9)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_assets(id,platform_id,game_id,content_id,role,mime,bytes,source,license,attribution) VALUES (?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET platform_id=excluded.platform_id,game_id=excluded.game_id,content_id=excluded.content_id,role=excluded.role,mime=excluded.mime,bytes=excluded.bytes,source=excluded.source,license=excluded.license,attribution=excluded.attribution",params![self.id,self.platform_id,self.game_id,self.content_id,self.role,self.mime,self.bytes,self.source,self.license,self.attribution,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveCatalog {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_catalogs", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q = c
            .prepare(
                "SELECT id,name,digest,attribution,imported_at FROM archive_catalogs LIMIT 100001",
            )
            .map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                name: r.get(1)?,
                digest: r.get(2)?,
                attribution: r.get(3)?,
                imported_at: r.get(4)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_catalogs(id,name,digest,attribution,imported_at) VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,digest=excluded.digest,attribution=excluded.attribution,imported_at=excluded.imported_at",params![self.id,self.name,self.digest,self.attribution,self.imported_at,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveCatalogEntry {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_catalog_entries", [], |r| {
                r.get(0)
            })
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q = c
            .prepare("SELECT catalog_id,game_id FROM archive_catalog_entries LIMIT 100001")
            .map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                catalog_id: r.get(0)?,
                game_id: r.get(1)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_catalog_entries(catalog_id,game_id) VALUES (?,?) ON CONFLICT(catalog_id,game_id) DO NOTHING",params![self.catalog_id,self.game_id,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveCollection {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_collections", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q = c
            .prepare("SELECT id,name FROM archive_collections LIMIT 100001")
            .map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_collections(id,name) VALUES (?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name",params![self.id,self.name,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveCollectionEntry {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_collection_entries", [], |r| {
                r.get(0)
            })
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q = c
            .prepare("SELECT collection_id,game_id FROM archive_collection_entries LIMIT 100001")
            .map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                collection_id: r.get(0)?,
                game_id: r.get(1)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_collection_entries(collection_id,game_id) VALUES (?,?) ON CONFLICT(collection_id,game_id) DO NOTHING",params![self.collection_id,self.game_id,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchivePreference {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_preferences", [], |r| r.get(0))
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT game_id,favorite,in_library,visited_at FROM archive_preferences LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                game_id: r.get(0)?,
                favorite: r.get(1)?,
                in_library: r.get(2)?,
                visited_at: r.get(3)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_preferences(game_id,favorite,in_library,visited_at) VALUES (?,?,?,?) ON CONFLICT(game_id) DO UPDATE SET favorite=excluded.favorite,in_library=excluded.in_library,visited_at=excluded.visited_at",params![self.game_id,self.favorite,self.in_library,self.visited_at,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveMetadataSource {
    pub(super) fn read(c: &Connection) -> Result<Vec<Self>> {
        let count: u32 = c
            .query_row("SELECT count(*) FROM archive_metadata_sources", [], |r| {
                r.get(0)
            })
            .map_err(sql_error)?;
        if count > 100_000 {
            return Err(invalid());
        }
        let mut q=c.prepare("SELECT id,game_id,source,attribution,note,created_at FROM archive_metadata_sources LIMIT 100001").map_err(sql_error)?;
        q.query_map([], |r| {
            Ok(Self {
                id: r.get(0)?,
                game_id: r.get(1)?,
                source: r.get(2)?,
                attribution: r.get(3)?,
                note: r.get(4)?,
                created_at: r.get(5)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)
    }
    pub(super) fn put(&self, c: &Connection) -> Result<()> {
        c.execute("INSERT INTO archive_metadata_sources(id,game_id,source,attribution,note,created_at) VALUES (?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET game_id=excluded.game_id,source=excluded.source,attribution=excluded.attribution,note=excluded.note,created_at=excluded.created_at",params![self.id,self.game_id,self.source,self.attribution,self.note,self.created_at,]).map_err(sql_error)?;
        Ok(())
    }
}
impl ArchiveData {
    pub(crate) fn read(c: &Connection) -> Result<Self> {
        Ok(Self {
            platforms: ArchivePlatform::read(c)?,
            games: ArchiveGame::read(c)?,
            releases: ArchiveRelease::read(c)?,
            editions: ArchiveEdition::read(c)?,
            contents: ArchiveContent::read(c)?,
            dependencies: ArchiveDependency::read(c)?,
            preservation_files: ArchiveFile::read(c)?,
            p2p_sources: ArchiveSource::read(c)?,
            assets: ArchiveAsset::read(c)?,
            catalogs: ArchiveCatalog::read(c)?,
            catalog_entries: ArchiveCatalogEntry::read(c)?,
            collections: ArchiveCollection::read(c)?,
            collection_entries: ArchiveCollectionEntry::read(c)?,
            preferences: ArchivePreference::read(c)?,
            metadata_sources: ArchiveMetadataSource::read(c)?,
        })
    }
    pub(crate) fn insert(&self, c: &Connection) -> Result<()> {
        for row in &self.platforms {
            row.put(c)?;
        }
        for row in &self.games {
            row.put(c)?;
        }
        for row in &self.releases {
            row.put(c)?;
        }
        for row in &self.editions {
            row.put(c)?;
        }
        for row in &self.contents {
            row.put(c)?;
        }
        for row in &self.dependencies {
            row.put(c)?;
        }
        for row in &self.preservation_files {
            row.put(c)?;
        }
        for row in &self.p2p_sources {
            row.put(c)?;
        }
        for row in &self.assets {
            row.put(c)?;
        }
        for row in &self.catalogs {
            row.put(c)?;
        }
        for row in &self.catalog_entries {
            row.put(c)?;
        }
        for row in &self.collections {
            row.put(c)?;
        }
        for row in &self.collection_entries {
            row.put(c)?;
        }
        for row in &self.preferences {
            row.put(c)?;
        }
        for row in &self.metadata_sources {
            row.put(c)?;
        }
        Ok(())
    }
    pub(crate) fn clear(c: &Connection) -> Result<()> {
        c.execute("DELETE FROM archive_metadata_sources", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_preferences", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_collection_entries", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_collections", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_catalog_entries", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_catalogs", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_assets", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_p2p_sources", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_preservation_files", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_dependencies", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_contents", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_editions", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_releases", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_games", [])
            .map_err(sql_error)?;
        c.execute("DELETE FROM archive_platforms", [])
            .map_err(sql_error)?;
        Ok(())
    }
}
