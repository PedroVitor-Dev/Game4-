//! Portable catalog snapshots. Parsing never opens referenced files or URLs.
use crate::{
    contracts::{ErrorCode, ErrorField, IpcError},
    database::Database,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::Write,
    path::Path,
};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;
pub const SCHEMA_ID: &str = "urn:game4:manifest:1";
pub const MAX_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_FILES: usize = 100_000;
type Result<T> = std::result::Result<T, IpcError>;
fn invalid() -> IpcError {
    IpcError::new(ErrorCode::ManifestInvalid)
}
fn database_error(_: rusqlite::Error) -> IpcError {
    IpcError::new(ErrorCode::DatabaseUnavailable)
}
fn required_nullable<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Option::deserialize(d)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format: String,
    pub schema_version: u32,
    pub collection: Collection,
    pub files: Vec<ManifestFile>,
    pub extensions: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Collection {
    pub id: String,
    pub title: String,
    #[serde(deserialize_with = "required_nullable")]
    pub platform: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub edition: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub language: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub declared_source: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub declared_license: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestFile {
    pub path: String,
    pub size_bytes: String,
    pub sha256: String,
}

fn valid_text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}
pub fn valid_path(path: &str) -> bool {
    if path.is_empty()
        || path.chars().count() > 4096
        || path
            .chars()
            .any(|c| c.is_control() || "\\:*?\"<>|".contains(c))
    {
        return false;
    }
    path.split('/').all(|part| {
        if part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' ']) {
            return false;
        }
        let stem = part.split('.').next().unwrap().to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str()) {
            return false;
        }
        !["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|s| {
                matches!(
                    s,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
    })
}
pub fn validate(manifest: &Manifest) -> Result<()> {
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(IpcError::new(ErrorCode::ManifestVersionUnsupported));
    }
    if manifest.schema != SCHEMA_ID || manifest.format != "game4-manifest" {
        return Err(invalid());
    }
    let c = &manifest.collection;
    if !Uuid::parse_str(&c.id).is_ok_and(|id| {
        id.get_version_num() == 4
            && id.get_variant() == uuid::Variant::RFC4122
            && id.to_string() == c.id
    }) || !valid_text(&c.title, 512)
    {
        return Err(invalid());
    }
    for (value, max) in [
        (&c.platform, 200),
        (&c.edition, 200),
        (&c.language, 100),
        (&c.declared_source, 4096),
        (&c.declared_license, 4096),
    ] {
        if value.as_ref().is_some_and(|s| !valid_text(s, max)) {
            return Err(invalid());
        }
    }
    if manifest.files.is_empty() || manifest.files.len() > MAX_FILES {
        return Err(invalid());
    }
    let mut paths = HashSet::new();
    for f in &manifest.files {
        if !valid_path(&f.path)
            || !paths.insert(f.path.as_str())
            || f.sha256.len() != 64
            || !f
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        if !f
            .size_bytes
            .parse::<u64>()
            .is_ok_and(|n| n.to_string() == f.size_bytes)
        {
            return Err(invalid());
        }
    }
    for path in &paths {
        for (index, _) in path.match_indices('/') {
            if paths.contains(&path[..index]) {
                return Err(invalid());
            }
        }
    }
    if manifest.extensions.keys().any(|key| {
        let parts: Vec<_> = key.split('.').collect();
        parts.len() < 2
            || key.len() > 200
            || parts.iter().any(|p| {
                p.is_empty()
                    || !p.as_bytes()[0].is_ascii_lowercase()
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
    }) {
        return Err(invalid());
    }
    Ok(())
}
pub fn parse(bytes: &[u8]) -> Result<Manifest> {
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    let envelope: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if envelope
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .is_some_and(|n| n != u64::from(SCHEMA_VERSION))
    {
        return Err(IpcError::new(ErrorCode::ManifestVersionUnsupported));
    }
    // Deserialize from original bytes so duplicate struct fields are rejected.
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    validate(&manifest)?;
    Ok(manifest)
}
pub fn serialize(manifest: &Manifest) -> Result<Vec<u8>> {
    validate(manifest)?;
    let mut ordered = manifest.clone();
    ordered.files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut bytes = serde_json::to_vec_pretty(&ordered).map_err(|_| invalid())?;
    bytes.push(b'\n');
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
pub fn item_id(value: &str) -> Result<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == value)
        .ok_or(IpcError {
            code: ErrorCode::InvalidArgument,
            retryable: false,
            field: Some(ErrorField::ItemId),
        })
}
pub fn from_catalog(db: &Database, id: &str) -> Result<Manifest> {
    let id = item_id(id)?;
    let mut c = db
        .connection
        .lock()
        .map_err(|_| IpcError::new(ErrorCode::DatabaseUnavailable))?;
    let tx = c.transaction().map_err(database_error)?;
    let mut collection=tx.query_row("SELECT title,platform,edition,language,declared_source,declared_license FROM collection_items WHERE id=?1",[id],|r|Ok(Collection{id:String::new(),title:r.get(0)?,platform:r.get(1)?,edition:r.get(2)?,language:r.get(3)?,declared_source:r.get(4)?,declared_license:r.get(5)?})).optional().map_err(database_error)?.ok_or_else(||IpcError::new(ErrorCode::NotFound))?;
    tx.execute("INSERT INTO collection_manifest_ids(item_id,manifest_id) VALUES (?1,?2) ON CONFLICT(item_id) DO NOTHING",params![id,Uuid::new_v4().to_string()]).map_err(database_error)?;
    collection.id = tx
        .query_row(
            "SELECT manifest_id FROM collection_manifest_ids WHERE item_id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(database_error)?;
    let files = {
        let mut stmt=tx.prepare("SELECT cf.relative_path,f.size_bytes,f.sha256 FROM collection_item_files cf JOIN files f ON f.id=cf.file_id WHERE cf.item_id=?1 ORDER BY cf.relative_path LIMIT 100001").map_err(database_error)?;
        stmt.query_map([id], |r| {
            Ok(ManifestFile {
                path: r.get(0)?,
                size_bytes: r.get::<_, i64>(1)?.to_string(),
                sha256: r.get(2)?,
            })
        })
        .map_err(database_error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(database_error)?
    };
    let manifest = Manifest {
        schema: SCHEMA_ID.into(),
        format: "game4-manifest".into(),
        schema_version: SCHEMA_VERSION,
        collection,
        files,
        extensions: BTreeMap::new(),
    };
    validate(&manifest)?;
    tx.commit().map_err(database_error)?;
    Ok(manifest)
}
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let fail = || IpcError::new(ErrorCode::ManifestWriteFailed);
    if !path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("json"))
    {
        return Err(fail());
    }
    // Validate the complete output before creating any filesystem entry.
    parse(bytes)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| fail())?;
    temporary.write_all(bytes).map_err(|_| fail())?;
    temporary.as_file().sync_all().map_err(|_| fail())?;
    // Never overwrite a source, catalog, or existing manifest, even after a race.
    temporary.persist_noclobber(path).map_err(|_| fail())?;
    Ok(())
}

#[cfg(test)]
mod tests;
