//! Public .g4 v1: stored ZIP/ZIP64 with a descriptor, manifest and exact payload.
//! Reading verifies all bytes without extracting, executing or changing a catalog.
use crate::{
    contracts::{ErrorCode, IpcError},
    database::Database,
    library,
    manifests::{self, Manifest},
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const VERSION: u32 = 1;
pub const SCHEMA_ID: &str = "urn:game4:package:1";
pub const MAX_PAYLOAD: u64 = 1 << 40;
pub const MAX_ARCHIVE: u64 = MAX_PAYLOAD + (512 << 20);
const MAX_DIRECTORY: u64 = 256 << 20;
const MAX_DESCRIPTOR: u64 = 16 * 1024;
type Result<T> = std::result::Result<T, IpcError>;
fn error(code: ErrorCode) -> IpcError {
    IpcError::new(code)
}
fn invalid() -> IpcError {
    error(ErrorCode::PackageInvalid)
}
fn sql(_: rusqlite::Error) -> IpcError {
    error(ErrorCode::DatabaseUnavailable)
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Descriptor {
    #[serde(rename = "$schema")]
    schema: String,
    format: String,
    package_version: u32,
    manifest: String,
    payload_prefix: String,
}
fn descriptor() -> Descriptor {
    Descriptor {
        schema: SCHEMA_ID.into(),
        format: "game4-package".into(),
        package_version: VERSION,
        manifest: "manifest.json".into(),
        payload_prefix: "data/".into(),
    }
}
fn parse_descriptor(bytes: &[u8]) -> Result<()> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if value
        .get("packageVersion")
        .and_then(|v| v.as_u64())
        .is_some_and(|v| v != u64::from(VERSION))
    {
        return Err(error(ErrorCode::PackageVersionUnsupported));
    }
    let d: Descriptor = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if d.schema != SCHEMA_ID
        || d.format != "game4-package"
        || d.package_version != VERSION
        || d.manifest != "manifest.json"
        || d.payload_prefix != "data/"
    {
        return Err(invalid());
    }
    Ok(())
}
/// Stricter package profile: bounded names and case-insensitive leaf/prefix safety.
pub fn validate_manifest(manifest: &Manifest) -> Result<u64> {
    manifests::validate(manifest)?;
    let mut paths = HashSet::new();
    let mut total = 0u64;
    for file in &manifest.files {
        if file.path.len() > 1024
            || file.path.split('/').count() > 128
            || !paths.insert(file.path.to_lowercase())
        {
            return Err(invalid());
        }
        total = total
            .checked_add(file.size_bytes.parse::<u64>().map_err(|_| invalid())?)
            .filter(|n| *n <= MAX_PAYLOAD)
            .ok_or_else(|| error(ErrorCode::PackageLimitExceeded))?;
    }
    for path in &paths {
        for (i, _) in path.match_indices('/') {
            if paths.contains(&path[..i]) {
                return Err(invalid());
            }
        }
    }
    Ok(total)
}

fn u16le(b: &[u8]) -> u16 {
    u16::from_le_bytes(b[..2].try_into().unwrap())
}
fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes(b[..4].try_into().unwrap())
}
fn u64le(b: &[u8]) -> u64 {
    u64::from_le_bytes(b[..8].try_into().unwrap())
}
/// Check bounded central-directory counts/size before the ZIP library allocates
/// its index. Reject split archives, appended data and ZIP64 extensible sectors.
fn preflight(file: &mut File) -> Result<()> {
    let length = file.metadata().map_err(|_| invalid())?.len();
    if length > MAX_ARCHIVE {
        return Err(error(ErrorCode::PackageLimitExceeded));
    }
    // Empty archive comments pin the exact EOCD used by preflight and the
    // ZIP indexer, avoiding fallback to unbounded alternative footer candidates.
    let end = length.checked_sub(22).ok_or_else(invalid)?;
    file.seek(SeekFrom::Start(end)).map_err(|_| invalid())?;
    let mut e = [0u8; 22];
    file.read_exact(&mut e).map_err(|_| invalid())?;
    if e[..4] != *b"PK\x05\x06" || u16le(&e[20..]) != 0 {
        return Err(invalid());
    }
    if u16le(&e[4..]) != 0 || u16le(&e[6..]) != 0 || u16le(&e[8..]) != u16le(&e[10..]) {
        return Err(invalid());
    }
    let mut count = u64::from(u16le(&e[10..]));
    let mut size = u64::from(u32le(&e[12..]));
    let mut offset = u64::from(u32le(&e[16..]));
    let mut directory_end = end;
    // A ZIP64 locator can be present even when all EOCD values fit ZIP32.
    let mut locator = [0u8; 20];
    if end >= 20 {
        file.seek(SeekFrom::Start(end - 20))
            .map_err(|_| invalid())?;
        file.read_exact(&mut locator).map_err(|_| invalid())?;
    }
    if locator[..4] == *b"PK\x06\x07" {
        if u32le(&locator[4..]) != 0 || u32le(&locator[16..]) != 1 {
            return Err(invalid());
        }
        let at = u64le(&locator[8..]);
        if at.checked_add(56) != Some(end - 20) {
            return Err(invalid());
        }
        file.seek(SeekFrom::Start(at)).map_err(|_| invalid())?;
        let mut z = [0u8; 56];
        file.read_exact(&mut z).map_err(|_| invalid())?;
        if z[..4] != *b"PK\x06\x06"
            || u64le(&z[4..]) != 44
            || u32le(&z[16..]) != 0
            || u32le(&z[20..]) != 0
            || u64le(&z[24..]) != u64le(&z[32..])
        {
            return Err(invalid());
        }
        let nc = u64le(&z[32..]);
        let ns = u64le(&z[40..]);
        let no = u64le(&z[48..]);
        if (count != 65535 && count != nc)
            || (size != u64::from(u32::MAX) && size != ns)
            || (offset != u64::from(u32::MAX) && offset != no)
        {
            return Err(invalid());
        }
        count = nc;
        size = ns;
        offset = no;
        directory_end = at;
    } else if count == 65535 || size == u64::from(u32::MAX) || offset == u64::from(u32::MAX) {
        return Err(invalid());
    }
    if count < 3 || count > manifests::MAX_FILES as u64 + 2 || size > MAX_DIRECTORY {
        return Err(error(ErrorCode::PackageLimitExceeded));
    }
    if offset.checked_add(size) != Some(directory_end) || count > size / 46 {
        return Err(invalid());
    }
    file.rewind().map_err(|_| invalid())?;
    Ok(())
}
fn metadata<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>> {
    let mut entry = archive.by_name(name).map_err(|_| invalid())?;
    if entry.size() > limit {
        return Err(error(ErrorCode::PackageLimitExceeded));
    }
    let mut bytes = Vec::new();
    entry
        .by_ref()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() as u64 > limit {
        return Err(error(ErrorCode::PackageLimitExceeded));
    }
    Ok(bytes)
}
#[derive(Debug)]
pub struct VerifiedPackage {
    pub manifest: Manifest,
    pub total_bytes: u64,
}
pub fn read(path: &Path) -> Result<VerifiedPackage> {
    // Preserve lexical ancestors so junctions remain visible to open_source.
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| error(ErrorCode::SourceUnavailable))?
            .join(path)
    };
    let mut file = library::open_source(&absolute)?;
    let before = file.metadata().map_err(|_| invalid())?;
    preflight(&mut file)?;
    let mut archive = ZipArchive::with_config(
        zip::read::Config {
            archive_offset: zip::read::ArchiveOffset::Known(0),
        },
        file,
    )
    .map_err(|_| invalid())?;
    if archive.offset() != 0 || archive.has_duplicate_names() {
        return Err(invalid());
    }
    archive
        .validate_header_consistency()
        .map_err(|_| invalid())?;
    let mut names = HashSet::new();
    let mut starts_at_zero = false;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|_| invalid())?;
        starts_at_zero |= entry.header_start() == 0;
        let name = std::str::from_utf8(entry.name_raw()).map_err(|_| invalid())?;
        if !entry.is_file()
            || entry.encrypted()
            || entry.compression() != CompressionMethod::Stored
            || entry.compressed_size() != entry.size()
            || entry
                .unix_mode()
                .is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
            || !names.insert(name.to_owned())
        {
            return Err(invalid());
        }
    }
    if !starts_at_zero {
        return Err(invalid());
    }
    parse_descriptor(&metadata(&mut archive, "game4.json", MAX_DESCRIPTOR)?)?;
    let manifest = manifests::parse(&metadata(
        &mut archive,
        "manifest.json",
        manifests::MAX_BYTES as u64,
    )?)?;
    let total_bytes = validate_manifest(&manifest)?;
    if names.len() != manifest.files.len() + 2 {
        return Err(invalid());
    }
    let mut buffer = vec![0u8; 1024 * 1024];
    for file in &manifest.files {
        let name = format!("data/{}", file.path);
        if !names.contains(&name) {
            return Err(invalid());
        }
        let mut entry = archive.by_name(&name).map_err(|_| invalid())?;
        let expected = file.size_bytes.parse::<u64>().map_err(|_| invalid())?;
        if entry.size() != expected {
            return Err(error(ErrorCode::PackageIntegrityFailed));
        }
        let mut hash = Sha256::new();
        let mut size = 0u64;
        loop {
            let n = entry
                .read(&mut buffer)
                .map_err(|_| error(ErrorCode::PackageIntegrityFailed))?;
            if n == 0 {
                break;
            }
            size += n as u64;
            if size > expected {
                return Err(error(ErrorCode::PackageIntegrityFailed));
            }
            hash.update(&buffer[..n]);
        }
        if size != expected || format!("{:x}", hash.finalize()) != file.sha256 {
            return Err(error(ErrorCode::PackageIntegrityFailed));
        }
    }
    let after = archive.into_inner().metadata().map_err(|_| invalid())?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(error(ErrorCode::SourceChanged));
    }
    Ok(VerifiedPackage {
        manifest,
        total_bytes,
    })
}

/// Candidate paths are local-only, in manifest order. No paths enter the package.
pub fn create(path: &Path, manifest: &Manifest, sources: &[Arc<Vec<PathBuf>>]) -> Result<()> {
    create_observed(path, manifest, sources, &mut |_| {})
}
fn create_observed(
    path: &Path,
    manifest: &Manifest,
    sources: &[Arc<Vec<PathBuf>>],
    after_candidate: &mut impl FnMut(&Path),
) -> Result<()> {
    validate_manifest(manifest)?;
    if !path
        .extension()
        .is_some_and(|v| v.eq_ignore_ascii_case("g4"))
        || sources.len() != manifest.files.len()
    {
        return Err(invalid());
    }
    let manifest_bytes = manifests::serialize(manifest)?;
    let parent = path
        .parent()
        .filter(|v| !v.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let fail = || error(ErrorCode::PackageWriteFailed);
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| fail())?;
    {
        let mut zip = ZipWriter::new(temporary.as_file_mut());
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(0o644)
            .system(zip::System::Unix);
        zip.start_file("game4.json", options).map_err(|_| fail())?;
        zip.write_all(&serde_json::to_vec_pretty(&descriptor()).map_err(|_| invalid())?)
            .map_err(|_| fail())?;
        zip.start_file("manifest.json", options)
            .map_err(|_| fail())?;
        zip.write_all(&manifest_bytes).map_err(|_| fail())?;
        let mut buffer = vec![0u8; 1024 * 1024];
        // Serialization sorts manifest paths. Payload entry order uses the same
        // ordering while retaining each path's candidate mapping.
        let mut entries: Vec<_> = manifest.files.iter().zip(sources).collect();
        entries.sort_by(|a, b| a.0.path.cmp(&b.0.path));
        for (expected, candidates) in entries {
            let length = expected.size_bytes.parse::<u64>().map_err(|_| invalid())?;
            let mut source = None;
            for candidate in candidates.iter() {
                if !library::open_source(candidate)
                    .ok()
                    .and_then(|f| f.metadata().ok())
                    .is_some_and(|m| m.len() == length)
                {
                    continue;
                }
                if let Ok((size, hash, _)) =
                    library::hash_file_with_buffer(candidate, &mut buffer, &mut || {})
                    && size.to_string() == expected.size_bytes
                    && hash == expected.sha256
                {
                    after_candidate(candidate);
                    if let Ok(open) = library::open_source(candidate) {
                        source = Some(open);
                        break;
                    }
                }
            }
            let mut source = source.ok_or_else(|| error(ErrorCode::PackageIntegrityFailed))?;
            let before = source
                .metadata()
                .map_err(|_| error(ErrorCode::SourceUnavailable))?;
            zip.start_file(
                format!("data/{}", expected.path),
                options.large_file(length >= u64::from(u32::MAX)),
            )
            .map_err(|_| fail())?;
            let mut hash = Sha256::new();
            let mut written = 0u64;
            loop {
                let n = source
                    .read(&mut buffer)
                    .map_err(|_| error(ErrorCode::SourceUnavailable))?;
                if n == 0 {
                    break;
                }
                written += n as u64;
                if written > length {
                    return Err(error(ErrorCode::PackageIntegrityFailed));
                }
                hash.update(&buffer[..n]);
                zip.write_all(&buffer[..n]).map_err(|_| fail())?;
            }
            let after = source
                .metadata()
                .map_err(|_| error(ErrorCode::SourceUnavailable))?;
            if written != length
                || format!("{:x}", hash.finalize()) != expected.sha256
                || before.len() != after.len()
                || before.modified().ok() != after.modified().ok()
            {
                return Err(error(ErrorCode::PackageIntegrityFailed));
            }
        }
        zip.finish().map_err(|_| fail())?;
    }
    if temporary.as_file().metadata().map_err(|_| fail())?.len() > MAX_ARCHIVE {
        return Err(error(ErrorCode::PackageLimitExceeded));
    }
    temporary.as_file().sync_all().map_err(|_| fail())?;
    temporary.persist_noclobber(path).map_err(|_| fail())?;
    Ok(())
}
/// Caller holds the shared worker to keep the catalog stable across the snapshot.
pub fn from_catalog(db: &Database, item: &str) -> Result<(Manifest, Vec<Arc<Vec<PathBuf>>>)> {
    let manifest = manifests::from_catalog(db, item)?;
    validate_manifest(&manifest)?;
    let c = db
        .connection
        .lock()
        .map_err(|_| error(ErrorCode::DatabaseUnavailable))?;
    let mut q=c.prepare("SELECT DISTINCT f.sha256,f.size_bytes,l.path FROM collection_item_files cf JOIN files f ON f.id=cf.file_id JOIN file_locations l ON l.file_id=f.id WHERE cf.item_id=?1 ORDER BY l.id LIMIT 500001").map_err(sql)?;
    let mut copies: HashMap<(String, String), Vec<PathBuf>> = HashMap::new();
    let mut count = 0;
    for row in q
        .query_map(params![item], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?.to_string(),
                PathBuf::from(r.get::<_, String>(2)?),
            ))
        })
        .map_err(sql)?
    {
        let (hash, size, path) = row.map_err(sql)?;
        count += 1;
        if count > 500_000 {
            return Err(error(ErrorCode::PackageLimitExceeded));
        }
        copies.entry((hash, size)).or_default().push(path);
    }
    let copies: HashMap<_, _> = copies.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();
    let sources = manifest
        .files
        .iter()
        .map(|f| {
            copies
                .get(&(f.sha256.clone(), f.size_bytes.clone()))
                .cloned()
                .unwrap_or_default()
        })
        .collect();
    Ok((manifest, sources))
}
#[cfg(test)]
mod tests;
