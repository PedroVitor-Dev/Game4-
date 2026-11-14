//! .g4e v1, offline parsing and checked direct-original torrent creation.
use crate::{
    contracts::*,
    database::Database,
    library,
    manifests::{self, Manifest, ManifestFile},
};
use serde::{
    Deserialize, Serialize,
    de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Value, value::RawValue};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fmt,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use url::Url;
use uuid::Uuid;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_METAINFO: usize = 8 * 1024 * 1024;
pub const PIECE_LENGTH: usize = 1024 * 1024;
const SCHEMA: &str = "urn:game4:ever-manifest:1";
type Result<T> = std::result::Result<T, IpcError>;
fn err(code: ErrorCode) -> IpcError {
    IpcError::new(code)
}
fn invalid() -> IpcError {
    err(ErrorCode::G4eInvalid)
}
fn limit() -> IpcError {
    err(ErrorCode::G4eLimitExceeded)
}
fn integrity() -> IpcError {
    err(ErrorCode::G4eIntegrityFailed)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EverManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format: String,
    pub schema_version: u32,
    pub id: String,
    pub representation: EverRepresentation,
    pub manifest: Manifest,
    pub torrents: Vec<TorrentReference>,
    pub extensions: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TorrentReference {
    pub info_hash_v1: String,
    pub magnet: String,
    #[serde(deserialize_with = "required_nullable")]
    pub metainfo_file: Option<MetainfoFile>,
}
fn required_nullable<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<MetainfoFile>, D::Error> {
    Option::deserialize(d)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetainfoFile {
    pub path: String,
    pub size_bytes: String,
    pub sha256: String,
}
// Validate depth and duplicate JSON keys without building an untyped copy.
struct Shape(usize);
impl<'de> DeserializeSeed<'de> for Shape {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        if self.0 > 64 {
            return Err(D::Error::custom("depth limit"));
        }
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Shape {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("strict JSON")
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<(), E> {
        if v.is_finite() {
            Ok(())
        } else {
            Err(E::custom("nonfinite"))
        }
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<(), A::Error> {
        while seq.next_element_seed(Shape(self.0 + 1))?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
        let mut keys = HashSet::new();
        while let Some(k) = map.next_key::<String>()? {
            if !keys.insert(k) {
                return Err(A::Error::custom("duplicate key"));
            }
            map.next_value_seed(Shape(self.0 + 1))?;
        }
        Ok(())
    }
}
fn hex(s: &str, n: usize) -> bool {
    s.len() == n
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn path(s: &str) -> Result<()> {
    if !manifests::valid_path(s) {
        return Err(invalid());
    }
    if s.len() > 4096 || s.split('/').count() > 128 {
        return Err(limit());
    }
    Ok(())
}
fn namespace(key: &str) -> bool {
    key.len() <= 200
        && key.split('.').count() >= 2
        && key.split('.').all(|p| {
            !p.is_empty()
                && p.as_bytes()[0].is_ascii_lowercase()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
fn magnet(s: &str, hash: &str) -> Result<()> {
    if s.len() > 16384
        || !s.starts_with("magnet:?")
        || !s.bytes().all(|b| (0x21..=0x7e).contains(&b))
    {
        return Err(invalid());
    }
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'%'
            && (i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit())
        {
            return Err(invalid());
        }
    }
    let u = Url::parse(s).map_err(|_| invalid())?;
    if u.scheme() != "magnet"
        || u.host_str().is_some()
        || !u.path().is_empty()
        || u.fragment().is_some()
    {
        return Err(invalid());
    }
    let mut xt = 0;
    let mut dn = 0;
    let mut trackers = HashSet::new();
    for (k, v) in u.query_pairs() {
        if v.chars().any(|c| c.is_control() || c == '\u{fffd}') {
            return Err(invalid());
        }
        match k.as_ref() {
            "xt" => {
                xt += 1;
                if v != format!("urn:btih:{hash}") {
                    return Err(invalid());
                }
            }
            "dn" => dn += 1,
            "tr" => {
                let t = Url::parse(&v).map_err(|_| invalid())?;
                if !["http", "https", "udp"].contains(&t.scheme())
                    || t.host_str().is_none()
                    || !t.username().is_empty()
                    || t.password().is_some()
                    || t.fragment().is_some()
                    || v.chars().any(|c| c <= ' ' || c == '\u{7f}')
                    || t.port() == Some(0)
                    || (t.scheme() == "udp" && t.port().is_none())
                    || !trackers.insert(v.into_owned())
                {
                    return Err(invalid());
                }
            }
            _ => return Err(invalid()),
        }
    }
    if xt != 1 || dn > 1 || trackers.len() > 32 {
        return Err(invalid());
    }
    Ok(())
}
pub fn validate(doc: &EverManifest) -> Result<u64> {
    if doc.schema_version != 1 {
        return Err(err(ErrorCode::G4eUnsupportedVersion));
    }
    if doc.schema != SCHEMA
        || doc.format != "game4-ever-manifest"
        || !Uuid::parse_str(&doc.id).is_ok_and(|id| {
            id.get_version_num() == 4
                && id.get_variant() == uuid::Variant::RFC4122
                && id.to_string() == doc.id
        })
    {
        return Err(invalid());
    }
    manifests::validate(&doc.manifest).map_err(|_| invalid())?;
    if doc.torrents.is_empty()
        || doc.torrents.len() > 16
        || doc.extensions.keys().any(|k| !namespace(k))
    {
        return Err(invalid());
    }
    let mut paths = HashSet::new();
    let mut total = 0u64;
    for f in &doc.manifest.files {
        path(&f.path)?;
        if !paths.insert(f.path.to_lowercase()) {
            return Err(invalid());
        }
        total = total
            .checked_add(f.size_bytes.parse::<u64>().map_err(|_| invalid())?)
            .ok_or_else(invalid)?;
    }
    for p in &paths {
        for (i, _) in p.match_indices('/') {
            if paths.contains(&p[..i]) {
                return Err(invalid());
            }
        }
    }
    if matches!(doc.representation, EverRepresentation::G4Package) {
        if doc.manifest.files.len() != 1
            || !doc.manifest.files[0].path.to_lowercase().ends_with(".g4")
        {
            return Err(invalid());
        }
        if total > crate::packages::MAX_ARCHIVE {
            return Err(limit());
        }
    }
    let mut hashes = HashSet::new();
    let mut companions = Vec::<String>::new();
    for t in &doc.torrents {
        if !hex(&t.info_hash_v1, 40) || !hashes.insert(&t.info_hash_v1) {
            return Err(invalid());
        }
        magnet(&t.magnet, &t.info_hash_v1)?;
        if let Some(m) = &t.metainfo_file {
            path(&m.path)?;
            let lower = m.path.to_lowercase();
            if !lower.ends_with(".torrent") || !hex(&m.sha256, 64) || paths.contains(&lower) {
                return Err(invalid());
            }
            if paths.iter().chain(companions.iter()).any(|p| {
                p == &lower
                    || p.starts_with(&(lower.clone() + "/"))
                    || lower.starts_with(&(p.clone() + "/"))
            }) {
                return Err(invalid());
            }
            let size = m.size_bytes.parse::<u64>().map_err(|_| invalid())?;
            if size == 0 || size.to_string() != m.size_bytes {
                return Err(invalid());
            }
            if size > MAX_METAINFO as u64 {
                return Err(limit());
            }
            companions.push(lower);
        }
    }
    Ok(total)
}
pub fn parse(bytes: &[u8]) -> Result<EverManifest> {
    if bytes.len() > MAX_BYTES {
        return Err(limit());
    }
    let mut de = serde_json::Deserializer::from_slice(bytes);
    Shape(0).deserialize(&mut de).map_err(|e| {
        if e.to_string().starts_with("depth limit") {
            limit()
        } else {
            invalid()
        }
    })?;
    de.end().map_err(|_| invalid())?;
    // RawValue keeps the embedded JSON's own whitespace in its byte bound.
    #[derive(Deserialize)]
    struct Envelope<'a> {
        #[serde(rename = "schemaVersion")]
        version: Option<u32>,
        #[serde(borrow)]
        manifest: &'a RawValue,
    }
    let raw: Envelope<'_> = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if raw.version.is_some_and(|v| v != 1) {
        return Err(err(ErrorCode::G4eUnsupportedVersion));
    }
    if raw.manifest.get().len() > manifests::MAX_BYTES {
        return Err(limit());
    }
    let doc: EverManifest = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    validate(&doc)?;
    Ok(doc)
}
pub fn serialize(doc: &EverManifest) -> Result<Vec<u8>> {
    validate(doc)?;
    let mut bytes = serde_json::to_vec_pretty(doc).map_err(|_| invalid())?;
    bytes.push(b'\n');
    parse(&bytes)?;
    Ok(bytes)
}
fn absolute(p: &Path) -> Result<PathBuf> {
    if p.is_absolute() {
        Ok(p.into())
    } else {
        Ok(std::env::current_dir().map_err(|_| invalid())?.join(p))
    }
}
pub fn read(p: &Path) -> Result<EverManifest> {
    let f = library::open_source(&absolute(p)?)?;
    let mut bytes = Vec::new();
    f.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    parse(&bytes)
}

// Minimal deterministic bencoder; bytes and dictionaries are kept distinct.
#[derive(Debug, Clone)]
pub(crate) enum B {
    Int(i128),
    Bytes(Vec<u8>),
    List(Vec<B>),
    Dict(BTreeMap<Vec<u8>, B>),
}
fn string(s: &str) -> B {
    B::Bytes(s.as_bytes().to_vec())
}
fn dict(items: impl IntoIterator<Item = (&'static str, B)>) -> B {
    B::Dict(
        items
            .into_iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v))
            .collect(),
    )
}
fn encode(b: &B, out: &mut Vec<u8>) {
    match b {
        B::Int(n) => out.extend_from_slice(format!("i{n}e").as_bytes()),
        B::Bytes(s) => {
            out.extend_from_slice(format!("{}:", s.len()).as_bytes());
            out.extend_from_slice(s)
        }
        B::List(items) => {
            out.push(b'l');
            for i in items {
                encode(i, out)
            }
            out.push(b'e')
        }
        B::Dict(items) => {
            out.push(b'd');
            for (k, v) in items {
                encode(&B::Bytes(k.clone()), out);
                encode(v, out)
            }
            out.push(b'e')
        }
    }
}
fn source(root: &Path, file: &ManifestFile, multiple: bool) -> Result<PathBuf> {
    if root.is_dir() {
        Ok(root.join(&file.path))
    } else if !multiple && root.file_name().and_then(|n| n.to_str()) == Some(&file.path) {
        Ok(root.into())
    } else {
        Err(integrity())
    }
}
fn hash_sources(files: &[ManifestFile], root: &Path, piece_length: usize) -> Result<Vec<u8>> {
    let mut pieces = Vec::new();
    let mut piece = Sha1::new();
    let mut filled = 0;
    let mut buffer = vec![0u8; 1024 * 1024];
    for f in files {
        let p = source(root, f, files.len() > 1)?;
        let mut file = library::open_source(&p).map_err(|_| integrity())?;
        let before = file.metadata().map_err(|_| integrity())?;
        let expected = f.size_bytes.parse::<u64>().map_err(|_| invalid())?;
        if before.len() != expected {
            return Err(integrity());
        }
        let mut sha = Sha256::new();
        let mut size = 0u64;
        loop {
            let n = file.read(&mut buffer).map_err(|_| integrity())?;
            if n == 0 {
                break;
            }
            size = size.checked_add(n as u64).ok_or_else(limit)?;
            if size > expected {
                return Err(integrity());
            }
            sha.update(&buffer[..n]);
            let mut chunk = &buffer[..n];
            while !chunk.is_empty() {
                let take = (piece_length - filled).min(chunk.len());
                piece.update(&chunk[..take]);
                chunk = &chunk[take..];
                filled += take;
                if filled == piece_length {
                    pieces.extend_from_slice(&piece.finalize_reset());
                    filled = 0;
                    if pieces.len() > MAX_METAINFO {
                        return Err(limit());
                    }
                }
            }
        }
        let after = file.metadata().map_err(|_| integrity())?;
        if size != expected
            || format!("{:x}", sha.finalize()) != f.sha256
            || before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
        {
            return Err(integrity());
        }
    }
    if filled > 0 {
        pieces.extend_from_slice(&piece.finalize());
    }
    Ok(pieces)
}
pub struct Created {
    pub document: EverManifest,
    pub torrent: Vec<u8>,
}
pub fn create(manifest: &Manifest, root: &Path) -> Result<Created> {
    let mut m = manifest.clone();
    m.files.sort_by(|a, b| a.path.cmp(&b.path));
    let hash = "0".repeat(40);
    let mut document = EverManifest {
        schema: SCHEMA.into(),
        format: "game4-ever-manifest".into(),
        schema_version: 1,
        id: Uuid::new_v4().to_string(),
        representation: EverRepresentation::OriginalFiles,
        manifest: m,
        torrents: vec![TorrentReference {
            info_hash_v1: hash.clone(),
            magnet: format!("magnet:?xt=urn:btih:{hash}"),
            metainfo_file: None,
        }],
        extensions: BTreeMap::new(),
    };
    serialize(&document)?;
    let total = validate(&document)?;
    if total.div_ceil(PIECE_LENGTH as u64) > ((MAX_METAINFO / 20) as u64) {
        return Err(limit());
    }
    let root = absolute(root)?;
    let name = root
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(invalid)?;
    path(name)?;
    if name.contains('/') {
        return Err(invalid());
    }
    let mut info = if root.is_dir() {
        dict([
            (
                "files",
                B::List(
                    document
                        .manifest
                        .files
                        .iter()
                        .map(|f| {
                            dict([
                                ("length", B::Int(f.size_bytes.parse::<i128>().unwrap())),
                                ("path", B::List(f.path.split('/').map(string).collect())),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("name", string(name)),
            ("piece length", B::Int(PIECE_LENGTH as i128)),
            ("pieces", B::Bytes(vec![])),
        ])
    } else {
        if document.manifest.files.len() != 1 || document.manifest.files[0].path != name {
            return Err(integrity());
        }
        dict([
            ("length", B::Int(total as i128)),
            ("name", string(name)),
            ("piece length", B::Int(PIECE_LENGTH as i128)),
            ("pieces", B::Bytes(vec![])),
        ])
    };
    let mut overhead = Vec::new();
    encode(&info, &mut overhead);
    let piece_bytes = total
        .div_ceil(PIECE_LENGTH as u64)
        .checked_mul(20)
        .ok_or_else(limit)?;
    if overhead.len() as u64 + piece_bytes + 32 > MAX_METAINFO as u64 {
        return Err(limit());
    }
    let pieces = hash_sources(&document.manifest.files, &root, PIECE_LENGTH)?;
    if let B::Dict(ref mut d) = info {
        d.insert(b"pieces".to_vec(), B::Bytes(pieces));
    }
    let mut raw_info = Vec::new();
    encode(&info, &mut raw_info);
    let hash = format!("{:x}", Sha1::digest(&raw_info));
    document.torrents[0].info_hash_v1 = hash.clone();
    document.torrents[0].magnet = format!("magnet:?xt=urn:btih:{hash}");
    let mut torrent = Vec::new();
    encode(&dict([("info", info)]), &mut torrent);
    if torrent.len() > MAX_METAINFO {
        return Err(limit());
    }
    serialize(&document)?;
    Ok(Created { document, torrent })
}
pub fn from_catalog(db: &Database, item: &str) -> Result<Created> {
    let m = manifests::from_catalog(db, item)?;
    let root: String = db
        .connection
        .lock()
        .map_err(|_| err(ErrorCode::DatabaseUnavailable))?
        .query_row(
            "SELECT source_key FROM import_sources WHERE item_id=?1",
            [item],
            |r| r.get(0),
        )
        .map_err(|_| err(ErrorCode::SourceUnavailable))?;
    create(&m, Path::new(&root))
}
pub fn write_pair(path: &Path, created: &Created) -> Result<(String, String)> {
    validate(&created.document)?;
    metainfo::bind(&created.document, &created.torrent)?;
    if created.document.torrents.len() != 1
        || created.torrent.is_empty()
        || created.torrent.len() > MAX_METAINFO
    {
        return Err(invalid());
    }
    let fail = || err(ErrorCode::G4eWriteFailed);
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("g4e"))
        || path.try_exists().map_err(|_| fail())?
    {
        return Err(fail());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let torrent_name = format!("game4-{}.torrent", Uuid::new_v4());
    let mut doc = created.document.clone();
    doc.torrents[0].metainfo_file = Some(MetainfoFile {
        path: torrent_name.clone(),
        size_bytes: created.torrent.len().to_string(),
        sha256: format!("{:x}", Sha256::digest(&created.torrent)),
    });
    let bytes = serialize(&doc)?;
    // Prepare both before publishing; the descriptor is published last. Never
    // remove a path after a race. A late failure may leave the new companion.
    let mut torrent = tempfile::NamedTempFile::new_in(parent).map_err(|_| fail())?;
    torrent.write_all(&created.torrent).map_err(|_| fail())?;
    torrent.as_file().sync_all().map_err(|_| fail())?;
    let mut json = tempfile::NamedTempFile::new_in(parent).map_err(|_| fail())?;
    json.write_all(&bytes).map_err(|_| fail())?;
    json.as_file().sync_all().map_err(|_| fail())?;
    torrent
        .persist_noclobber(parent.join(&torrent_name))
        .map_err(|_| fail())?;
    json.persist_noclobber(path).map_err(|_| fail())?;
    Ok((
        path.file_name()
            .ok_or_else(invalid)?
            .to_string_lossy()
            .into_owned(),
        torrent_name,
    ))
}
#[cfg(test)]
mod tests;

mod metainfo;
pub use metainfo::verify;
pub(crate) use metainfo::{Layout, inspect, route};
pub fn create_package(path: &Path) -> Result<Created> {
    let path = absolute(path)?;
    let before = std::fs::metadata(&path).map_err(|_| integrity())?;
    let package = crate::packages::read(&path)?;
    let (size, hash, _) =
        library::hash_file_with_buffer(&path, &mut vec![0u8; 1024 * 1024], &mut || {})?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(invalid)?
        .to_owned();
    let manifest = Manifest {
        schema: manifests::SCHEMA_ID.into(),
        format: "game4-manifest".into(),
        schema_version: 1,
        collection: package.manifest.collection,
        files: vec![ManifestFile {
            path: name,
            size_bytes: size.to_string(),
            sha256: hash,
        }],
        extensions: BTreeMap::new(),
    };
    let mut created = create(&manifest, &path)?;
    created.document.representation = EverRepresentation::G4Package;
    let after = std::fs::metadata(&path).map_err(|_| integrity())?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(integrity());
    }
    serialize(&created.document)?;
    Ok(created)
}
