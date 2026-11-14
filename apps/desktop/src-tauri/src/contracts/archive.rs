use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchivePlatform {
    pub id: String,
    pub name: String,
    pub manufacturer: String,
    pub year: Option<u32>,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveGame {
    pub id: String,
    pub title: String,
    pub year: Option<u32>,
    pub developer: String,
    pub publisher: String,
    pub genre: String,
    pub description: String,
    pub history: String,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveRelease {
    pub id: String,
    pub game_id: String,
    pub platform_id: String,
    pub region: String,
    pub languages: String,
    pub title_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveEdition {
    pub id: String,
    pub release_id: String,
    pub name: String,
    pub medium: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveContent {
    pub id: String,
    pub edition_id: String,
    pub kind: String,
    pub name: String,
    pub version: String,
    pub build: String,
    pub release_date: String,
    pub format: String,
    pub base_id: Option<String>,
    pub description: String,
    pub changelog: String,
    pub identifier: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveDependency {
    pub content_id: String,
    pub required_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveFile {
    pub id: String,
    pub content_id: String,
    pub name: String,
    pub format: String,
    pub size_bytes: String,
    pub sha256: String,
    pub item_id: Option<String>,
    pub file_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveSource {
    pub id: String,
    pub content_id: String,
    pub magnet: String,
    pub license: String,
    pub attribution: String,
    pub authorized: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveAsset {
    pub id: String,
    pub platform_id: Option<String>,
    pub game_id: Option<String>,
    pub content_id: Option<String>,
    pub role: String,
    pub mime: String,
    pub bytes: Vec<u8>,
    pub source: String,
    pub license: String,
    pub attribution: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveCatalog {
    pub id: String,
    pub name: String,
    pub digest: String,
    pub attribution: String,
    pub imported_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveCatalogEntry {
    pub catalog_id: String,
    pub game_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveCollection {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveCollectionEntry {
    pub collection_id: String,
    pub game_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchivePreference {
    pub game_id: String,
    pub favorite: bool,
    pub in_library: bool,
    pub visited_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveMetadataSource {
    pub id: String,
    pub game_id: String,
    pub source: String,
    pub attribution: String,
    pub note: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveData {
    pub platforms: Vec<ArchivePlatform>,
    pub games: Vec<ArchiveGame>,
    pub releases: Vec<ArchiveRelease>,
    pub editions: Vec<ArchiveEdition>,
    pub contents: Vec<ArchiveContent>,
    pub dependencies: Vec<ArchiveDependency>,
    pub preservation_files: Vec<ArchiveFile>,
    pub p2p_sources: Vec<ArchiveSource>,
    pub assets: Vec<ArchiveAsset>,
    pub catalogs: Vec<ArchiveCatalog>,
    pub catalog_entries: Vec<ArchiveCatalogEntry>,
    pub collections: Vec<ArchiveCollection>,
    pub collection_entries: Vec<ArchiveCollectionEntry>,
    pub preferences: Vec<ArchivePreference>,
    pub metadata_sources: Vec<ArchiveMetadataSource>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveSearch {
    pub search: String,
    pub platform_id: String,
    pub region: String,
    pub genre: String,
    pub language: String,
    pub edition: String,
    pub year: Option<u32>,
    pub availability: String,
    pub order: String,
    pub offset: u32,
    pub scope: String,
    pub collection_id: String,
    pub catalog_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveGameCard {
    pub game: ArchiveGame,
    pub platforms: String,
    pub region: String,
    pub cover_id: Option<String>,
    pub favorite: bool,
    pub in_library: bool,
    pub local_files: u32,
    pub source_count: u32,
    pub updates: u32,
    pub dlcs: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveStats {
    pub games: u32,
    pub platforms: u32,
    pub local_files: u32,
    pub sources: u32,
    pub catalogs: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchivePlatformCard {
    pub platform: ArchivePlatform,
    pub cover_id: Option<String>,
    pub games: u32,
    pub with_files: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveFileState {
    pub file_id: String,
    pub status: String,
    pub checked_at: Option<String>,
    pub location: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveAssetInfo {
    pub id: String,
    pub platform_id: Option<String>,
    pub game_id: Option<String>,
    pub content_id: Option<String>,
    pub role: String,
    pub source: String,
    pub license: String,
    pub attribution: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ArchiveRequest {
    Overview,
    Platforms,
    Games {
        query: ArchiveSearch,
    },
    Game {
        id: String,
    },
    Asset {
        id: String,
    },
    SavePlatform {
        value: ArchivePlatform,
    },
    SaveGame {
        value: ArchiveGame,
    },
    SaveRelease {
        value: ArchiveRelease,
    },
    SaveEdition {
        value: ArchiveEdition,
    },
    SaveContent {
        value: ArchiveContent,
    },
    SaveDependency {
        value: ArchiveDependency,
    },
    SaveSource {
        value: ArchiveSource,
    },
    SaveMetadataSource {
        value: ArchiveMetadataSource,
    },
    SaveCollection {
        value: ArchiveCollection,
    },
    Member {
        #[serde(rename = "collectionId")]
        collection_id: String,
        #[serde(rename = "gameId")]
        game_id: String,
        present: bool,
    },
    Preference {
        #[serde(rename = "gameId")]
        game_id: String,
        favorite: bool,
        #[serde(rename = "inLibrary")]
        in_library: bool,
    },
    Visit {
        #[serde(rename = "gameId")]
        game_id: String,
    },
    LinkItem {
        #[serde(rename = "contentId")]
        content_id: String,
        #[serde(rename = "itemId")]
        item_id: String,
    },
    AddAsset {
        #[serde(rename = "ownerKind")]
        owner_kind: String,
        #[serde(rename = "ownerId")]
        owner_id: String,
        role: String,
        source: String,
        license: String,
        attribution: String,
    },
    AttachEver {
        #[serde(rename = "contentId")]
        content_id: String,
        license: String,
        authorized: bool,
    },
    PrepareCatalog,
    ImportCatalog {
        token: String,
    },
    ExportCatalog {
        name: String,
        attribution: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ArchiveResponse {
    Overview {
        stats: ArchiveStats,
        platforms: Vec<ArchivePlatformCard>,
        recent: Vec<ArchiveGameCard>,
        updated: Vec<ArchiveGameCard>,
        collections: Vec<ArchiveCollection>,
        catalogs: Vec<ArchiveCatalog>,
    },
    Platforms {
        platforms: Vec<ArchivePlatformCard>,
    },
    Games {
        items: Vec<ArchiveGameCard>,
        total: u32,
        offset: u32,
    },
    Game {
        card: Box<ArchiveGameCard>,
        releases: Vec<ArchiveRelease>,
        editions: Vec<ArchiveEdition>,
        contents: Vec<ArchiveContent>,
        dependencies: Vec<ArchiveDependency>,
        files: Vec<ArchiveFile>,
        sources: Vec<ArchiveSource>,
        assets: Vec<ArchiveAssetInfo>,
        states: Vec<ArchiveFileState>,
        provenance: Vec<ArchiveMetadataSource>,
        catalogs: Vec<ArchiveCatalog>,
        collections: Vec<String>,
    },
    Asset {
        bytes: Vec<u8>,
        mime: String,
    },
    CatalogPreview {
        token: String,
        name: String,
        games: u32,
        platforms: u32,
        attribution: String,
    },
    Done {
        id: Option<String>,
        cancelled: bool,
    },
}
