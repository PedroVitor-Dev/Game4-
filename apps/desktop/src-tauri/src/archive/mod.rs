//! Offline historical catalog. Only explicit existing transfer actions can start P2P.
use crate::contracts::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    sync::Mutex,
    time::Instant,
};
use uuid::Uuid;
mod records;
#[cfg(test)]
mod tests;
pub(crate) type Result<T> = std::result::Result<T, IpcError>;
pub(crate) fn invalid() -> IpcError {
    IpcError::new(ErrorCode::InvalidArgument)
}
fn sql_error(_: rusqlite::Error) -> IpcError {
    IpcError::new(ErrorCode::DatabaseUnavailable)
}
fn missing() -> IpcError {
    IpcError::new(ErrorCode::NotFound)
}
const MAX_CATALOG: usize = 32 * 1024 * 1024;
#[derive(Default)]
pub struct ArchiveState {
    pending: Mutex<HashMap<String, (Instant, CatalogDocument)>>,
}
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn text(s: &str, max: usize, required: bool) -> Result<()> {
    if s.len() > max
        || (required && s.trim().is_empty())
        || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid());
    }
    Ok(())
}
fn valid_id(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 180
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return Err(invalid());
    }
    Ok(())
}
fn year(y: Option<u32>) -> Result<()> {
    if y.is_some_and(|v| !(1950..=2200).contains(&v)) {
        return Err(invalid());
    }
    Ok(())
}
fn exists(c: &Connection, table: &str, value: &str) -> Result<()> {
    valid_id(value)?;
    if !c
        .query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM archive_{table} WHERE id=?)"),
            [value],
            |r| r.get::<_, bool>(0),
        )
        .map_err(sql_error)?
    {
        return Err(missing());
    }
    Ok(())
}
fn revision(c: &Connection, game: &str, note: &str) -> Result<()> {
    c.execute(
        "UPDATE archive_games SET updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?",
        [game],
    )
    .map_err(sql_error)?;
    ArchiveMetadataSource {
        id: id(),
        game_id: game.into(),
        source: "Local catalog".into(),
        attribution: String::new(),
        note: note.into(),
        created_at: now(c)?,
    }
    .put(c)
}
fn now(c: &Connection) -> Result<String> {
    c.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ','now')", [], |r| {
        r.get(0)
    })
    .map_err(sql_error)
}
fn content_game(c: &Connection, content: &str) -> Result<String> {
    c.query_row("SELECT r.game_id FROM archive_contents x JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE x.id=?",[content],|r|r.get(0)).optional().map_err(sql_error)?.ok_or_else(missing)
}
fn validate_content(c: &Connection, v: &ArchiveContent) -> Result<()> {
    valid_id(&v.id)?;
    exists(c, "editions", &v.edition_id)?;
    text(&v.name, 512, true)?;
    for s in [
        &v.version,
        &v.build,
        &v.release_date,
        &v.format,
        &v.identifier,
    ] {
        text(s, 256, false)?;
    }
    text(&v.description, 20000, false)?;
    text(&v.changelog, 20000, false)?;
    if !["version", "update", "dlc"].contains(&v.kind.as_str()) {
        return Err(invalid());
    }
    match (&v.kind[..], &v.base_id) {
        ("version", None) => {}
        ("update" | "dlc", Some(base)) => {
            let relation: Option<(String, String)> = c
                .query_row(
                    "SELECT edition_id,kind FROM archive_contents WHERE id=?",
                    [base],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(sql_error)?;
            if base == &v.id || relation != Some((v.edition_id.clone(), "version".into())) {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }
    // Existing compatibility links prevent changing a base revision into DLC/update.
    let dependents: u32 = c
        .query_row(
            "SELECT count(*) FROM archive_contents WHERE base_id=?",
            [&v.id],
            |r| r.get(0),
        )
        .map_err(sql_error)?;
    if dependents > 0 && v.kind != "version" {
        return Err(invalid());
    }
    let old: Option<String> = c
        .query_row(
            "SELECT edition_id FROM archive_contents WHERE id=?",
            [&v.id],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql_error)?;
    if old.is_some_and(|old| old != v.edition_id) {
        return Err(invalid());
    }
    Ok(())
}
fn dependency(c: &Connection, v: &ArchiveDependency) -> Result<()> {
    if v.content_id == v.required_id
        || content_game(c, &v.content_id)? != content_game(c, &v.required_id)?
    {
        return Err(invalid());
    }
    let cycle:bool=c.query_row("WITH RECURSIVE reach(id) AS (SELECT required_id FROM archive_dependencies WHERE content_id=?1 UNION SELECT d.required_id FROM archive_dependencies d JOIN reach r ON d.content_id=r.id) SELECT EXISTS(SELECT 1 FROM reach WHERE id=?2)",params![v.required_id,v.content_id],|r|r.get(0)).map_err(sql_error)?;
    if cycle {
        return Err(invalid());
    }
    v.put(c)
}
fn validate_source(v: &ArchiveSource) -> Result<()> {
    valid_id(&v.id)?;
    valid_id(&v.content_id)?;
    text(&v.license, 2000, true)?;
    text(&v.attribution, 2000, false)?;
    crate::torrents::normalize_magnet(&v.magnet)?;
    Ok(())
}
fn validate_file(v: &ArchiveFile) -> Result<()> {
    valid_id(&v.id)?;
    valid_id(&v.content_id)?;
    text(&v.name, 4096, true)?;
    text(&v.format, 256, false)?;
    if !v
        .size_bytes
        .parse::<u64>()
        .is_ok_and(|n| n.to_string() == v.size_bytes)
        || (!v.sha256.is_empty()
            && (v.sha256.len() != 64 || !v.sha256.bytes().all(|b| b.is_ascii_hexdigit())))
    {
        return Err(invalid());
    }
    Ok(())
}
fn validate_game(v: &ArchiveGame) -> Result<()> {
    valid_id(&v.id)?;
    text(&v.title, 512, true)?;
    year(v.year)?;
    for s in [&v.developer, &v.publisher, &v.genre] {
        text(s, 512, false)?;
    }
    text(&v.description, 20000, false)?;
    text(&v.history, 20000, false)?;
    Ok(())
}
fn validate_platform(v: &ArchivePlatform) -> Result<()> {
    valid_id(&v.id)?;
    text(&v.name, 512, true)?;
    text(&v.manufacturer, 512, true)?;
    year(v.year)?;
    text(&v.description, 20000, false)
}
fn validate_release(v: &ArchiveRelease) -> Result<()> {
    valid_id(&v.id)?;
    valid_id(&v.game_id)?;
    valid_id(&v.platform_id)?;
    for s in [&v.region, &v.languages, &v.title_id] {
        text(s, 512, false)?
    }
    Ok(())
}
fn validate_edition(v: &ArchiveEdition) -> Result<()> {
    valid_id(&v.id)?;
    valid_id(&v.release_id)?;
    text(&v.name, 512, true)?;
    text(&v.medium, 256, false)
}
const CARD_SELECT:&str="SELECT g.id,g.title,g.year,g.developer,g.publisher,g.genre,g.description,g.history,g.created_at,g.updated_at,
 COALESCE((SELECT group_concat(DISTINCT p.name) FROM archive_releases r JOIN archive_platforms p ON p.id=r.platform_id WHERE r.game_id=g.id),''),
 COALESCE((SELECT group_concat(DISTINCT r.region) FROM archive_releases r WHERE r.game_id=g.id),''),
 (SELECT id FROM archive_assets WHERE game_id=g.id AND role='cover' ORDER BY rowid DESC LIMIT 1),
 COALESCE((SELECT favorite FROM archive_preferences WHERE game_id=g.id),0),COALESCE((SELECT in_library FROM archive_preferences WHERE game_id=g.id),0),
 (SELECT count(*) FROM archive_preservation_files f JOIN archive_contents x ON x.id=f.content_id JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=g.id AND f.item_id IS NOT NULL),
 (SELECT count(*) FROM archive_p2p_sources s JOIN archive_contents x ON x.id=s.content_id JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=g.id),
 (SELECT count(*) FROM archive_contents x JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=g.id AND x.kind='update'),
 (SELECT count(*) FROM archive_contents x JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=g.id AND x.kind='dlc') FROM archive_games g";
fn card(r: &rusqlite::Row<'_>) -> rusqlite::Result<ArchiveGameCard> {
    Ok(ArchiveGameCard {
        game: ArchiveGame {
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
        },
        platforms: r.get(10)?,
        region: r.get(11)?,
        cover_id: r.get(12)?,
        favorite: r.get(13)?,
        in_library: r.get(14)?,
        local_files: r.get(15)?,
        source_count: r.get(16)?,
        updates: r.get(17)?,
        dlcs: r.get(18)?,
    })
}
fn game_cards(c: &Connection, query: &ArchiveSearch) -> Result<(Vec<ArchiveGameCard>, u32)> {
    text(&query.search, 512, false)?;
    if query.offset > 1_000_000 {
        return Err(invalid());
    }
    for s in [
        &query.region,
        &query.genre,
        &query.language,
        &query.edition,
        &query.platform_id,
        &query.collection_id,
        &query.catalog_id,
    ] {
        text(s, 512, false)?;
    }
    let filter=" WHERE (?1='' OR instr(lower(g.title),lower(?1))>0 OR instr(lower(g.developer),lower(?1))>0)
 AND (?2 IS NULL OR g.year=?2) AND (?3='' OR instr(lower(g.genre),lower(?3))>0)
 AND ((?4='' AND ?5='' AND ?6='' AND ?7='') OR EXISTS(SELECT 1 FROM archive_releases r WHERE r.game_id=g.id AND (?4='' OR r.platform_id=?4) AND (?5='' OR instr(lower(r.region),lower(?5))>0) AND (?6='' OR instr(lower(r.languages),lower(?6))>0) AND (?7='' OR EXISTS(SELECT 1 FROM archive_editions e WHERE e.release_id=r.id AND instr(lower(e.name),lower(?7))>0))))
 AND (?8<>'favorites' OR EXISTS(SELECT 1 FROM archive_preferences WHERE game_id=g.id AND favorite=1))
 AND (?8<>'recent' OR EXISTS(SELECT 1 FROM archive_preferences WHERE game_id=g.id AND visited_at IS NOT NULL))
 AND (?8<>'personal' OR EXISTS(SELECT 1 FROM archive_preferences WHERE game_id=g.id AND in_library=1))
 AND (?9='' OR EXISTS(SELECT 1 FROM archive_collection_entries WHERE game_id=g.id AND collection_id=?9))
 AND (?10='' OR EXISTS(SELECT 1 FROM archive_catalog_entries WHERE game_id=g.id AND catalog_id=?10))
 AND (?11='' OR (?11='local' AND EXISTS(SELECT 1 FROM archive_preservation_files f JOIN archive_contents x ON x.id=f.content_id JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=g.id AND f.item_id IS NOT NULL)) OR (?11='reference' AND EXISTS(SELECT 1 FROM archive_p2p_sources s JOIN archive_contents x ON x.id=s.content_id JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=g.id)))";
    let order = match query.order.as_str() {
        "year" => "g.year DESC,g.title COLLATE NOCASE,g.id",
        "recent" => "g.created_at DESC,g.id",
        "updated" => "g.updated_at DESC,g.id",
        "visited" => "(SELECT visited_at FROM archive_preferences WHERE game_id=g.id) DESC,g.id",
        _ => "g.title COLLATE NOCASE,g.id",
    };
    let values = params![
        query.search,
        query.year,
        query.genre,
        query.platform_id,
        query.region,
        query.language,
        query.edition,
        query.scope,
        query.collection_id,
        query.catalog_id,
        query.availability
    ];
    let total = c
        .query_row(
            &format!("SELECT count(*) FROM archive_games g{filter}"),
            values,
            |r| r.get(0),
        )
        .map_err(sql_error)?;
    let mut q = c
        .prepare(&format!(
            "{CARD_SELECT}{filter} ORDER BY {order} LIMIT 48 OFFSET {}",
            query.offset
        ))
        .map_err(sql_error)?;
    let rows = q
        .query_map(values, card)
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    Ok((rows, total))
}
fn platforms(c: &Connection) -> Result<Vec<ArchivePlatformCard>> {
    let mut q=c.prepare("SELECT p.id,p.name,p.manufacturer,p.year,p.description,(SELECT id FROM archive_assets WHERE platform_id=p.id ORDER BY rowid DESC LIMIT 1),
 (SELECT count(DISTINCT game_id) FROM archive_releases WHERE platform_id=p.id),
 (SELECT count(DISTINCT r.game_id) FROM archive_releases r JOIN archive_editions e ON e.release_id=r.id JOIN archive_contents x ON x.edition_id=e.id JOIN archive_preservation_files f ON f.content_id=x.id WHERE r.platform_id=p.id AND f.item_id IS NOT NULL) FROM archive_platforms p ORDER BY p.manufacturer,p.name LIMIT 501").map_err(sql_error)?;
    q.query_map([], |r| {
        Ok(ArchivePlatformCard {
            platform: ArchivePlatform {
                id: r.get(0)?,
                name: r.get(1)?,
                manufacturer: r.get(2)?,
                year: r.get(3)?,
                description: r.get(4)?,
            },
            cover_id: r.get(5)?,
            games: r.get(6)?,
            with_files: r.get(7)?,
        })
    })
    .map_err(sql_error)?
    .collect::<std::result::Result<Vec<_>, _>>()
    .map_err(sql_error)
}
fn detail(c: &Connection, id: &str) -> Result<ArchiveResponse> {
    valid_id(id)?;
    let card = c
        .query_row(&format!("{CARD_SELECT} WHERE g.id=?"), [id], card)
        .optional()
        .map_err(sql_error)?
        .ok_or_else(missing)?;
    // Bounded per-game queries do not scan/load the entire catalog.
    let releases = read_selected::<ArchiveRelease>(c, "releases", "game_id=?", id)?;
    let editions = read_selected::<ArchiveEdition>(
        c,
        "editions",
        "release_id IN (SELECT id FROM archive_releases WHERE game_id=?)",
        id,
    )?;
    let content_filter = "edition_id IN (SELECT e.id FROM archive_editions e JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=?)";
    let contents = read_selected::<ArchiveContent>(c, "contents", content_filter, id)?;
    let nested = "content_id IN (SELECT x.id FROM archive_contents x JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=?)";
    let files = read_selected::<ArchiveFile>(c, "preservation_files", nested, id)?;
    let sources = read_selected::<ArchiveSource>(c, "p2p_sources", nested, id)?;
    let dependencies = read_selected::<ArchiveDependency>(c, "dependencies", nested, id)?;
    let mut assets_q=c.prepare("SELECT id,platform_id,game_id,content_id,role,source,license,attribution FROM archive_assets WHERE game_id=?1 OR content_id IN (SELECT x.id FROM archive_contents x JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=?1) LIMIT 100").map_err(sql_error)?;
    let assets = assets_q
        .query_map([id], |r| {
            Ok(ArchiveAssetInfo {
                id: r.get(0)?,
                platform_id: r.get(1)?,
                game_id: r.get(2)?,
                content_id: r.get(3)?,
                role: r.get(4)?,
                source: r.get(5)?,
                license: r.get(6)?,
                attribution: r.get(7)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    let mut q=c.prepare("SELECT af.id,l.verification_status,l.last_verified_at,l.path FROM archive_preservation_files af JOIN file_locations l ON l.file_id=af.file_id JOIN collection_item_files cf ON cf.file_id=af.file_id AND cf.item_id=af.item_id WHERE af.content_id IN (SELECT x.id FROM archive_contents x JOIN archive_editions e ON e.id=x.edition_id JOIN archive_releases r ON r.id=e.release_id WHERE r.game_id=?) GROUP BY af.id,l.id LIMIT 1024").map_err(sql_error)?;
    let states = q
        .query_map([id], |r| {
            Ok(ArchiveFileState {
                file_id: r.get(0)?,
                status: r.get(1)?,
                checked_at: r.get(2)?,
                location: r.get(3)?,
            })
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    let provenance =
        read_selected::<ArchiveMetadataSource>(c, "metadata_sources", "game_id=?", id)?;
    let catalogs = read_selected::<ArchiveCatalog>(
        c,
        "catalogs",
        "id IN (SELECT catalog_id FROM archive_catalog_entries WHERE game_id=?)",
        id,
    )?;
    let mut q = c
        .prepare("SELECT collection_id FROM archive_collection_entries WHERE game_id=? LIMIT 500")
        .map_err(sql_error)?;
    let collections = q
        .query_map([id], |r| r.get(0))
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    Ok(ArchiveResponse::Game {
        card: Box::new(card),
        releases,
        editions,
        contents,
        dependencies,
        files,
        sources,
        assets,
        states,
        provenance,
        catalogs,
        collections,
    })
}
// Serialize SQL rows through a fixed column map, preserving decimal identifiers in IPC.
fn read_selected<T: serde::de::DeserializeOwned>(
    c: &Connection,
    table: &str,
    filter: &str,
    id: &str,
) -> Result<Vec<T>> {
    let mut q = c
        .prepare(&format!(
            "SELECT * FROM archive_{table} WHERE {filter} LIMIT 1024"
        ))
        .map_err(sql_error)?;
    let columns = q.column_names().map(|s| s.to_string()).collect::<Vec<_>>();
    let rows = q
        .query_map([id], |r| {
            let mut map = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                let key = col
                    .split('_')
                    .enumerate()
                    .map(|(i, s)| {
                        if i == 0 {
                            s.into()
                        } else {
                            let mut c = s.chars();
                            c.next().unwrap().to_uppercase().to_string() + c.as_str()
                        }
                    })
                    .collect::<String>();
                let value = match r.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(n) if col == "authorized" => {
                        serde_json::json!(n != 0)
                    }
                    rusqlite::types::ValueRef::Integer(n)
                        if col == "item_id" || col == "file_id" =>
                    {
                        serde_json::json!(n.to_string())
                    }
                    rusqlite::types::ValueRef::Integer(n) => serde_json::json!(n),
                    rusqlite::types::ValueRef::Text(s) => {
                        serde_json::json!(String::from_utf8_lossy(s))
                    }
                    rusqlite::types::ValueRef::Blob(b) => serde_json::json!(b),
                    _ => serde_json::Value::Null,
                };
                map.insert(key, value);
            }
            Ok(serde_json::Value::Object(map))
        })
        .map_err(sql_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    rows.into_iter()
        .map(|v| serde_json::from_value(v).map_err(|_| invalid()))
        .collect()
}
fn done(id: Option<String>) -> ArchiveResponse {
    ArchiveResponse::Done {
        id,
        cancelled: false,
    }
}
pub fn execute(c: &mut Connection, request: ArchiveRequest) -> Result<ArchiveResponse> {
    match request {
        ArchiveRequest::Overview => {
            let stats=c.query_row("SELECT (SELECT count(*) FROM archive_games),(SELECT count(*) FROM archive_platforms),(SELECT count(*) FROM archive_preservation_files WHERE item_id IS NOT NULL),(SELECT count(*) FROM archive_p2p_sources),(SELECT count(*) FROM archive_catalogs)",[],|r|Ok(ArchiveStats{games:r.get(0)?,platforms:r.get(1)?,local_files:r.get(2)?,sources:r.get(3)?,catalogs:r.get(4)?})).map_err(sql_error)?;
            let (mut recent, _) = game_cards(
                c,
                &ArchiveSearch {
                    order: "recent".into(),
                    ..Default::default()
                },
            )?;
            recent.truncate(8);
            let (mut updated, _) = game_cards(
                c,
                &ArchiveSearch {
                    order: "updated".into(),
                    ..Default::default()
                },
            )?;
            updated.truncate(8);
            Ok(ArchiveResponse::Overview {
                stats,
                platforms: platforms(c)?,
                recent,
                updated,
                collections: ArchiveCollection::read(c)?,
                catalogs: ArchiveCatalog::read(c)?,
            })
        }
        ArchiveRequest::Platforms => Ok(ArchiveResponse::Platforms {
            platforms: platforms(c)?,
        }),
        ArchiveRequest::Games { query } => {
            let (items, total) = game_cards(c, &query)?;
            Ok(ArchiveResponse::Games {
                items,
                total,
                offset: query.offset,
            })
        }
        ArchiveRequest::Game { id } => detail(c, &id),
        ArchiveRequest::Asset { id } => {
            valid_id(&id)?;
            c.query_row(
                "SELECT bytes,mime FROM archive_assets WHERE id=?",
                [id],
                |r| {
                    Ok(ArchiveResponse::Asset {
                        bytes: r.get(0)?,
                        mime: r.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(sql_error)?
            .ok_or_else(missing)
        }
        request => {
            let tx = c.transaction().map_err(sql_error)?;
            let result = match request {
                ArchiveRequest::SavePlatform { value: v } => {
                    validate_platform(&v)?;
                    if platforms(&tx)?.len() >= 500 && exists(&tx, "platforms", &v.id).is_err() {
                        return Err(invalid());
                    }
                    v.put(&tx)?;
                    done(Some(v.id))
                }
                ArchiveRequest::SaveGame { value: mut v } => {
                    validate_game(&v)?;
                    let previous: Option<String> = tx
                        .query_row(
                            "SELECT created_at FROM archive_games WHERE id=?",
                            [&v.id],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(sql_error)?;
                    v.created_at = previous.unwrap_or(now(&tx)?);
                    v.updated_at = now(&tx)?;
                    v.put(&tx)?;
                    revision(&tx, &v.id, "Game metadata saved")?;
                    done(Some(v.id))
                }
                ArchiveRequest::SaveRelease { value: v } => {
                    validate_release(&v)?;
                    exists(&tx, "games", &v.game_id)?;
                    exists(&tx, "platforms", &v.platform_id)?;
                    let old: Option<(String, String)> = tx
                        .query_row(
                            "SELECT game_id,platform_id FROM archive_releases WHERE id=?",
                            [&v.id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .optional()
                        .map_err(sql_error)?;
                    if old.is_some_and(|o| o != (v.game_id.clone(), v.platform_id.clone())) {
                        return Err(invalid());
                    }
                    v.put(&tx)?;
                    revision(&tx, &v.game_id, "Platform release saved")?;
                    done(Some(v.id))
                }
                ArchiveRequest::SaveEdition { value: v } => {
                    validate_edition(&v)?;
                    exists(&tx, "releases", &v.release_id)?;
                    let old: Option<String> = tx
                        .query_row(
                            "SELECT release_id FROM archive_editions WHERE id=?",
                            [&v.id],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(sql_error)?;
                    if old.is_some_and(|o| o != v.release_id) {
                        return Err(invalid());
                    }
                    v.put(&tx)?;
                    done(Some(v.id))
                }
                ArchiveRequest::SaveContent { value: v } => {
                    validate_content(&tx, &v)?;
                    v.put(&tx)?;
                    revision(&tx, &content_game(&tx, &v.id)?, "Content revision saved")?;
                    done(Some(v.id))
                }
                ArchiveRequest::SaveDependency { value: v } => {
                    dependency(&tx, &v)?;
                    done(None)
                }
                ArchiveRequest::SaveSource { value: mut v } => {
                    validate_source(&v)?;
                    exists(&tx, "contents", &v.content_id)?;
                    v.magnet = crate::torrents::normalize_magnet(&v.magnet)?.magnet;
                    v.put(&tx)?;
                    revision(
                        &tx,
                        &content_game(&tx, &v.content_id)?,
                        "Distribution reference saved; availability unknown",
                    )?;
                    done(Some(v.id))
                }
                ArchiveRequest::SaveMetadataSource { value: mut v } => {
                    valid_id(&v.id)?;
                    exists(&tx, "games", &v.game_id)?;
                    text(&v.source, 2000, true)?;
                    text(&v.attribution, 2000, false)?;
                    text(&v.note, 20000, false)?;
                    v.created_at = now(&tx)?;
                    v.put(&tx)?;
                    done(Some(v.id))
                }
                ArchiveRequest::SaveCollection { value: v } => {
                    valid_id(&v.id)?;
                    text(&v.name, 512, true)?;
                    v.put(&tx)?;
                    done(Some(v.id))
                }
                ArchiveRequest::Member {
                    collection_id,
                    game_id,
                    present,
                } => {
                    exists(&tx, "collections", &collection_id)?;
                    exists(&tx, "games", &game_id)?;
                    if present {
                        ArchiveCollectionEntry {
                            collection_id,
                            game_id,
                        }
                        .put(&tx)?;
                    } else {
                        tx.execute("DELETE FROM archive_collection_entries WHERE collection_id=? AND game_id=?",params![collection_id,game_id]).map_err(sql_error)?;
                    }
                    done(None)
                }
                ArchiveRequest::Preference {
                    game_id,
                    favorite,
                    in_library,
                } => {
                    exists(&tx, "games", &game_id)?;
                    tx.execute("INSERT INTO archive_preferences(game_id,favorite,in_library) VALUES (?1,?2,?3) ON CONFLICT(game_id) DO UPDATE SET favorite=?2,in_library=?3",params![game_id,favorite,in_library]).map_err(sql_error)?;
                    done(None)
                }
                ArchiveRequest::Visit { game_id } => {
                    exists(&tx, "games", &game_id)?;
                    tx.execute("INSERT INTO archive_preferences(game_id,visited_at) VALUES (?1,strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(game_id) DO UPDATE SET visited_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')",[game_id]).map_err(sql_error)?;
                    done(None)
                }
                ArchiveRequest::LinkItem {
                    content_id,
                    item_id,
                } => {
                    exists(&tx, "contents", &content_id)?;
                    let item = crate::manifests::item_id(&item_id)?;
                    let rows = {
                        let mut q=tx.prepare("SELECT f.id,cf.relative_path,COALESCE(i.format_id,''),f.size_bytes,f.sha256 FROM collection_item_files cf JOIN files f ON f.id=cf.file_id LEFT JOIN file_identifications i ON i.file_id=f.id WHERE cf.item_id=? LIMIT 1025").map_err(sql_error)?;
                        q.query_map([item], |r| {
                            Ok((
                                r.get::<_, i64>(0)?,
                                r.get::<_, String>(1)?,
                                r.get::<_, String>(2)?,
                                r.get::<_, i64>(3)?,
                                r.get::<_, String>(4)?,
                            ))
                        })
                        .map_err(sql_error)?
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .map_err(sql_error)?
                    };
                    if rows.is_empty() || rows.len() > 1024 {
                        return Err(invalid());
                    }
                    for (f, name, format, size, hash) in rows {
                        let key = format!("local:{content_id}:{item}:{f}");
                        ArchiveFile {
                            id: key,
                            content_id: content_id.clone(),
                            name,
                            format,
                            size_bytes: size.to_string(),
                            sha256: hash,
                            item_id: Some(item_id.clone()),
                            file_id: Some(f.to_string()),
                        }
                        .put(&tx)?;
                    }
                    revision(
                        &tx,
                        &content_game(&tx, &content_id)?,
                        "Existing local files linked without copying originals",
                    )?;
                    done(None)
                }
                _ => return Err(invalid()),
            };
            tx.commit().map_err(sql_error)?;
            Ok(result)
        }
    }
}
fn image_mime(b: &[u8]) -> Result<&'static str> {
    if b.len() > 4 * 1024 * 1024 || b.len() < 16 {
        return Err(invalid());
    }
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        if b.len() < 24 {
            return Err(invalid());
        }
        let w = u32::from_be_bytes(b[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(b[20..24].try_into().unwrap());
        if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 32_000_000 {
            return Err(invalid());
        }
        Ok("image/png")
    } else if b.starts_with(b"\xff\xd8\xff") {
        Ok("image/jpeg")
    } else if b.starts_with(b"RIFF") && &b[8..12] == b"WEBP" {
        Ok("image/webp")
    } else {
        Err(invalid())
    }
}
fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| IpcError::new(ErrorCode::SourceUnavailable))?
        .take((max + 1) as u64)
        .read_to_end(&mut b)
        .map_err(|_| invalid())?;
    if b.len() > max {
        return Err(invalid());
    }
    Ok(b)
}
pub fn add_asset(
    c: &mut Connection,
    request: ArchiveRequest,
    bytes: Vec<u8>,
) -> Result<ArchiveResponse> {
    let ArchiveRequest::AddAsset {
        owner_kind,
        owner_id,
        role,
        source,
        license,
        attribution,
    } = request
    else {
        return Err(invalid());
    };
    for s in [&source, &license] {
        text(s, 2000, true)?
    }
    text(&attribution, 2000, false)?;
    if !["cover", "banner", "screenshot", "console"].contains(&role.as_str()) {
        return Err(invalid());
    }
    let table = match owner_kind.as_str() {
        "platform" => "platforms",
        "game" => "games",
        "content" => "contents",
        _ => return Err(invalid()),
    };
    exists(c, table, &owner_id)?;
    let count:u32=c.query_row("SELECT count(*) FROM archive_assets WHERE platform_id=?1 OR game_id=?1 OR content_id=?1",[&owner_id],|r|r.get(0)).map_err(sql_error)?;
    if count >= 50 {
        return Err(invalid());
    }
    let mime = image_mime(&bytes)?.to_string();
    let asset = ArchiveAsset {
        id: id(),
        platform_id: (table == "platforms").then(|| owner_id.clone()),
        game_id: (table == "games").then(|| owner_id.clone()),
        content_id: (table == "contents").then(|| owner_id.clone()),
        role,
        mime,
        bytes,
        source,
        license,
        attribution,
    };
    let tx = c.transaction().map_err(sql_error)?;
    asset.put(&tx)?;
    if table == "games" {
        revision(&tx, &owner_id, "Authorized artwork cached locally")?;
    }
    tx.commit().map_err(sql_error)?;
    Ok(done(Some(asset.id)))
}
pub fn attach_ever(
    c: &mut Connection,
    content_id: &str,
    license: &str,
    authorized: bool,
    bytes: &[u8],
) -> Result<ArchiveResponse> {
    exists(c, "contents", content_id)?;
    text(license, 2000, true)?;
    let doc = crate::ever::parse(bytes)?;
    let tx = c.transaction().map_err(sql_error)?;
    for t in doc.torrents {
        let source = ArchiveSource {
            id: id(),
            content_id: content_id.into(),
            magnet: crate::torrents::normalize_magnet(&t.magnet)?.magnet,
            license: license.into(),
            attribution: doc
                .manifest
                .collection
                .declared_source
                .clone()
                .unwrap_or_default(),
            authorized,
        };
        validate_source(&source)?;
        source.put(&tx)?;
    }
    for f in doc.manifest.files {
        let record = ArchiveFile {
            id: id(),
            content_id: content_id.into(),
            name: f.path,
            format: "original".into(),
            size_bytes: f.size_bytes,
            sha256: f.sha256,
            item_id: None,
            file_id: None,
        };
        validate_file(&record)?;
        record.put(&tx)?;
    }
    revision(
        &tx,
        &content_game(&tx, content_id)?,
        "Game4 Ever references attached offline",
    )?;
    tx.commit().map_err(sql_error)?;
    Ok(done(None))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogDocument {
    pub format: String,
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub attribution: String,
    pub data: ArchiveData,
}
impl ArchiveData {
    pub(crate) fn validate(&self, c: &Connection) -> Result<()> {
        let bytes = serde_json::to_vec(self).map_err(|_| invalid())?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(invalid());
        }
        if self.platforms.len() > 500
            || self.games.len() > 100_000
            || self.contents.len() > 100_000
            || self.assets.len() > 10000
        {
            return Err(invalid());
        }
        // Duplicate identities are invalid, rather than silently collapsed by SQL upserts.
        let encoded = serde_json::to_value(self).map_err(|_| invalid())?;
        for rows in encoded.as_object().ok_or_else(invalid)?.values() {
            let mut seen = std::collections::HashSet::new();
            for row in rows.as_array().ok_or_else(invalid)? {
                let object = row.as_object().ok_or_else(invalid)?;
                let key = object.get("id").cloned().unwrap_or_else(|| {
                    serde_json::json!([
                        object.get("contentId"),
                        object.get("requiredId"),
                        object.get("catalogId"),
                        object.get("collectionId"),
                        object.get("gameId")
                    ])
                });
                if !seen.insert(key.to_string()) {
                    return Err(invalid());
                }
            }
        }
        for p in &self.platforms {
            validate_platform(p)?
        }
        for g in &self.games {
            validate_game(g)?
        }
        for r in &self.releases {
            validate_release(r)?
        }
        for e in &self.editions {
            validate_edition(e)?
        }
        for v in &self.contents {
            validate_content(c, v)?
        }
        for d in &self.dependencies {
            dependency(c, d)?
        }
        for f in &self.preservation_files {
            validate_file(f)?;
            if f.item_id.is_some() != f.file_id.is_some() {
                return Err(invalid());
            }
            if let (Some(item), Some(file)) = (&f.item_id, &f.file_id) {
                let valid:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM collection_item_files cf JOIN files f ON f.id=cf.file_id WHERE cf.item_id=?1 AND f.id=?2 AND f.sha256=?3 AND CAST(f.size_bytes AS TEXT)=?4)",params![item,file,f.sha256,f.size_bytes],|r|r.get(0)).map_err(sql_error)?;
                if !valid {
                    return Err(invalid());
                }
            }
        }
        for s in &self.p2p_sources {
            validate_source(s)?
        }
        for a in &self.assets {
            valid_id(&a.id)?;
            if image_mime(&a.bytes)? != a.mime {
                return Err(invalid());
            }
            text(&a.source, 2000, true)?;
            text(&a.license, 2000, true)?;
            text(&a.attribution, 2000, false)?;
        }
        for m in &self.metadata_sources {
            valid_id(&m.id)?;
            text(&m.source, 2000, true)?;
            text(&m.note, 20000, false)?;
        }
        Ok(())
    }
}
fn validate_document(doc: &CatalogDocument) -> Result<()> {
    if doc.format != "game4-community-catalog" || doc.schema_version != 1 {
        return Err(invalid());
    }
    valid_id(&doc.id)?;
    text(&doc.name, 512, true)?;
    text(&doc.attribution, 2000, false)?;
    // Public catalogs cannot carry local paths, private IDs or personal preferences.
    if !doc.data.catalogs.is_empty()
        || !doc.data.catalog_entries.is_empty()
        || !doc.data.collections.is_empty()
        || !doc.data.collection_entries.is_empty()
        || !doc.data.preferences.is_empty()
        || doc
            .data
            .preservation_files
            .iter()
            .any(|f| f.item_id.is_some() || f.file_id.is_some())
    {
        return Err(invalid());
    }
    let mut c = Connection::open_in_memory().map_err(sql_error)?;
    c.pragma_update(None, "foreign_keys", true)
        .map_err(sql_error)?;
    crate::database::migrations::apply(&mut c).map_err(|_| invalid())?;
    let tx = c.transaction().map_err(sql_error)?;
    tx.pragma_update(None, "defer_foreign_keys", true)
        .map_err(sql_error)?;
    ArchiveData::clear(&tx)?;
    doc.data.insert(&tx)?;
    doc.data.validate(&tx)?;
    let violations: u32 = tx
        .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .map_err(sql_error)?;
    if violations > 0 {
        return Err(invalid());
    }
    tx.commit().map_err(sql_error)?;
    Ok(())
}
pub fn parse_catalog(bytes: &[u8]) -> Result<CatalogDocument> {
    if bytes.len() > MAX_CATALOG {
        return Err(invalid());
    }
    let doc: CatalogDocument = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if serde_json::to_vec(&doc).map_err(|_| invalid())?.len() > MAX_CATALOG {
        return Err(invalid());
    }
    validate_document(&doc)?;
    Ok(doc)
}
pub fn import_catalog(c: &mut Connection, doc: &CatalogDocument) -> Result<ArchiveResponse> {
    validate_document(doc)?;
    let bytes = serde_json::to_vec(doc).map_err(|_| invalid())?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let existing: Option<String> = c
        .query_row(
            "SELECT id FROM archive_catalogs WHERE digest=?",
            [&digest],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql_error)?;
    if let Some(existing) = existing {
        return Ok(done(Some(existing)));
    }
    // Snapshot-scoped IDs make reimports idempotent and conflicting catalogs non-destructive.
    let prefix = format!("catalog-{}", &digest[..24]);
    let mut data = doc.data.clone();
    let remap = |s: &mut String| {
        let hash = format!("{:x}", Sha256::digest(s.as_bytes()));
        *s = format!("{prefix}-{}", &hash[..32]);
    };
    for p in &mut data.platforms {
        remap(&mut p.id)
    }
    for g in &mut data.games {
        remap(&mut g.id)
    }
    for r in &mut data.releases {
        remap(&mut r.id);
        remap(&mut r.game_id);
        remap(&mut r.platform_id)
    }
    for e in &mut data.editions {
        remap(&mut e.id);
        remap(&mut e.release_id)
    }
    for x in &mut data.contents {
        remap(&mut x.id);
        remap(&mut x.edition_id);
        if let Some(base) = &mut x.base_id {
            remap(base)
        }
    }
    for d in &mut data.dependencies {
        remap(&mut d.content_id);
        remap(&mut d.required_id)
    }
    for f in &mut data.preservation_files {
        remap(&mut f.id);
        remap(&mut f.content_id)
    }
    for s in &mut data.p2p_sources {
        remap(&mut s.id);
        remap(&mut s.content_id);
        s.authorized = false;
        s.magnet = crate::torrents::normalize_magnet(&s.magnet)?.magnet;
    }
    for a in &mut data.assets {
        remap(&mut a.id);
        for s in [&mut a.game_id, &mut a.platform_id, &mut a.content_id]
            .into_iter()
            .flatten()
        {
            remap(s)
        }
    }
    for m in &mut data.metadata_sources {
        remap(&mut m.id);
        remap(&mut m.game_id)
    }
    let tx = c.transaction().map_err(sql_error)?;
    tx.pragma_update(None, "defer_foreign_keys", true)
        .map_err(sql_error)?;
    let platform_count: u32 = tx
        .query_row("SELECT count(*) FROM archive_platforms", [], |r| r.get(0))
        .map_err(sql_error)?;
    if platform_count as usize + data.platforms.len() > 500 {
        return Err(invalid());
    }
    data.insert(&tx)?;
    ArchiveCatalog {
        id: prefix.clone(),
        name: doc.name.clone(),
        digest,
        attribution: doc.attribution.clone(),
        imported_at: now(&tx)?,
    }
    .put(&tx)?;
    for g in &data.games {
        ArchiveCatalogEntry {
            catalog_id: prefix.clone(),
            game_id: g.id.clone(),
        }
        .put(&tx)?;
    }
    tx.commit().map_err(sql_error)?;
    Ok(done(Some(prefix)))
}
pub fn export_catalog(c: &Connection, name: &str, attribution: &str) -> Result<Vec<u8>> {
    text(name, 512, true)?;
    text(attribution, 2000, false)?;
    let mut data = ArchiveData::read(c)?;
    for f in &mut data.preservation_files {
        f.item_id = None;
        f.file_id = None;
    }
    for s in &mut data.p2p_sources {
        s.authorized = false;
    }
    data.preferences.clear();
    data.collections.clear();
    data.collection_entries.clear();
    data.catalogs.clear();
    data.catalog_entries.clear();
    let doc = CatalogDocument {
        format: "game4-community-catalog".into(),
        schema_version: 1,
        id: id(),
        name: name.into(),
        attribution: attribution.into(),
        data,
    };
    validate_document(&doc)?;
    let bytes = serde_json::to_vec(&doc).map_err(|_| invalid())?;
    if bytes.len() > MAX_CATALOG {
        return Err(invalid());
    }
    Ok(bytes)
}
// No arbitrary path is accepted from JavaScript; all file actions use native pickers.
pub fn dialog_request(
    c: &mut Connection,
    state: &ArchiveState,
    window: &tauri::WebviewWindow,
    request: ArchiveRequest,
) -> Result<ArchiveResponse> {
    match request {
        r @ ArchiveRequest::AddAsset { .. } => {
            let path = rfd::FileDialog::new()
                .set_parent(window)
                .add_filter("Authorized artwork", &["png", "jpg", "jpeg", "webp"])
                .pick_file();
            match path {
                Some(p) => add_asset(c, r, read_bounded(&p, 4 * 1024 * 1024)?),
                None => Ok(ArchiveResponse::Done {
                    id: None,
                    cancelled: true,
                }),
            }
        }
        ArchiveRequest::AttachEver {
            content_id,
            license,
            authorized,
        } => {
            let path = rfd::FileDialog::new()
                .set_parent(window)
                .add_filter("Game4 Ever Manifest", &["g4e"])
                .pick_file();
            match path {
                Some(p) => attach_ever(
                    c,
                    &content_id,
                    &license,
                    authorized,
                    &read_bounded(&p, crate::ever::MAX_BYTES)?,
                ),
                None => Ok(ArchiveResponse::Done {
                    id: None,
                    cancelled: true,
                }),
            }
        }
        ArchiveRequest::PrepareCatalog => {
            let path = rfd::FileDialog::new()
                .set_parent(window)
                .add_filter("Game4 community catalog", &["g4lib"])
                .pick_file();
            let Some(p) = path else {
                return Ok(ArchiveResponse::Done {
                    id: None,
                    cancelled: true,
                });
            };
            let doc = parse_catalog(&read_bounded(&p, MAX_CATALOG)?)?;
            let token = id();
            let result = ArchiveResponse::CatalogPreview {
                token: token.clone(),
                name: doc.name.clone(),
                games: doc.data.games.len() as u32,
                platforms: doc.data.platforms.len() as u32,
                attribution: doc.attribution.clone(),
            };
            let mut pending = state.pending.lock().map_err(|_| invalid())?;
            pending.retain(|_, (time, _)| time.elapsed().as_secs() < 300);
            if pending.len() >= 4 {
                return Err(invalid());
            }
            pending.insert(token, (Instant::now(), doc));
            Ok(result)
        }
        ArchiveRequest::ImportCatalog { token } => {
            let doc = state
                .pending
                .lock()
                .map_err(|_| invalid())?
                .remove(&token)
                .filter(|(time, _)| time.elapsed().as_secs() < 300)
                .ok_or_else(invalid)?
                .1;
            import_catalog(c, &doc)
        }
        ArchiveRequest::ExportCatalog { name, attribution } => {
            let bytes = export_catalog(c, &name, &attribution)?;
            let path = rfd::FileDialog::new()
                .set_parent(window)
                .add_filter("Game4 community catalog", &["g4lib"])
                .set_file_name("game4-archive.g4lib")
                .save_file();
            let Some(p) = path else {
                return Ok(ArchiveResponse::Done {
                    id: None,
                    cancelled: true,
                });
            };
            if !p
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("g4lib"))
            {
                return Err(invalid());
            }
            let mut file = tempfile::NamedTempFile::new_in(p.parent().ok_or_else(invalid)?)
                .map_err(|_| invalid())?;
            file.write_all(&bytes).map_err(|_| invalid())?;
            file.as_file().sync_all().map_err(|_| invalid())?;
            file.persist_noclobber(&p)
                .map_err(|_| IpcError::new(ErrorCode::ManifestWriteFailed))?;
            Ok(done(None))
        }
        r => execute(c, r),
    }
}
