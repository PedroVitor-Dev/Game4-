//! .g4e schema v2: G4EP reconstruction metadata only. Parsing never opens peers or fragments.
use super::{encoder, local, types::*};
use crate::{
    contracts::{ErrorCode, IpcError},
    ever,
    manifests::{self, Collection, Manifest, ManifestFile},
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Read, Write},
    net::SocketAddr,
    path::Path,
};
use uuid::Uuid;
type IpcResult<T> = std::result::Result<T, IpcError>;
pub const SCHEMA: &str = "urn:game4:ever-manifest:2";
pub const DOMAIN: &[u8] = b"Game4.g4e.v2\0";
pub const CANONICALIZATION: &str = "game4-g4e-v2-canonical-json-v1";
pub const MAX_GROUPS: usize = 83_333;
pub const MAX_DISCOVERY: usize = 4096;
fn invalid() -> IpcError {
    IpcError::new(ErrorCode::G4eInvalid)
}
fn integrity() -> IpcError {
    IpcError::new(ErrorCode::G4eIntegrityFailed)
}
fn unsupported() -> IpcError {
    IpcError::new(ErrorCode::G4eUnsupportedVersion)
}
fn digest(value: &str) -> IpcResult<[u8; 32]> {
    decode_hex(value)
}
fn decode_hex<const N: usize>(value: &str) -> IpcResult<[u8; N]> {
    if value.len() != N * 2
        || !value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
    {
        return Err(invalid());
    }
    let mut out = [0; N];
    for (i, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(pair).map_err(|_| invalid())?, 16)
            .map_err(|_| invalid())?;
    }
    Ok(out)
}
fn decimal(value: &str) -> IpcResult<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|v| v.to_string() == value)
        .ok_or_else(invalid)
}
fn hash(bytes: &[u8]) -> String {
    local::hex(encoder::sha256(bytes))
}
fn content_id(sha: &str) -> String {
    format!("urn:sha256:{sha}")
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct G4epEverManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format: String,
    pub schema_version: u32,
    pub id: String,
    pub representation: String,
    pub content_id: String,
    pub manifest: Manifest,
    pub g4ep: G4epDescription,
    pub discovery: Vec<DiscoveryReference>,
    pub authentication: Authentication,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct G4epDescription {
    pub protocol_version: u32,
    pub coding: Coding,
    pub groups: Vec<Group>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Coding {
    pub codec_id: String,
    pub algorithm: String,
    pub field: String,
    pub field_polynomial: String,
    pub rate: String,
    pub symbol_layout: String,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub shard_bytes: u32,
    pub padding: String,
    pub data_layout: String,
    pub recovery_index_offset: u8,
}
impl Coding {
    pub fn from_profile(p: CodingProfile) -> Self {
        Self {
            codec_id: encoder::CODEC_ID.into(),
            algorithm: "reed-solomon".into(),
            field: "gf(2^16)".into(),
            field_polynomial: "0x1002d".into(),
            rate: "high".into(),
            symbol_layout: "leopard-64-byte-planes".into(),
            data_shards: p.data_shards(),
            parity_shards: p.parity_shards(),
            shard_bytes: p.shard_bytes(),
            padding: "zero".into(),
            data_layout: "contiguous-systematic".into(),
            recovery_index_offset: 8,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub index: String,
    pub original_offset: String,
    pub original_bytes: String,
    pub fragments: Vec<Fragment>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fragment {
    pub id: String,
    pub index: u8,
    pub size_bytes: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscoveryReference {
    PrivatePeer {
        #[serde(rename = "nodeId")]
        node_id: String,
        endpoint: String,
    },
    BittorrentFragment {
        #[serde(rename = "fragmentId")]
        fragment_id: String,
        #[serde(rename = "infoHashV1")]
        info_hash_v1: String,
        magnet: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Authentication {
    pub canonicalization: String,
    pub payload_sha256: String,
    pub signatures: Vec<ManifestSignature>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestSignature {
    pub algorithm: String,
    pub key_id: String,
    pub public_key: String,
    pub signature: String,
}

/// Deterministic JSON for this strictly typed profile: ASCII keys, integers <=u32, no floats.
/// Sort by UTF-16 code units, preserve strings/array order, no Unicode normalization.
fn canonical(value: &serde_json::Value, out: &mut Vec<u8>) -> IpcResult<()> {
    match value {
        serde_json::Value::Object(map) => {
            out.push(b'{');
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            for (i, key) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',')
                }
                serde_json::to_writer(&mut *out, key).map_err(|_| invalid())?;
                out.push(b':');
                canonical(&map[key], out)?;
            }
            out.push(b'}')
        }
        serde_json::Value::Array(array) => {
            out.push(b'[');
            for (i, item) in array.iter().enumerate() {
                if i > 0 {
                    out.push(b',')
                }
                canonical(item, out)?;
            }
            out.push(b']')
        }
        serde_json::Value::Number(n) => {
            if !n.as_u64().is_some_and(|v| v <= u64::from(u32::MAX)) {
                return Err(invalid());
            }
            serde_json::to_writer(&mut *out, n).map_err(|_| invalid())?;
        }
        _ => serde_json::to_writer(&mut *out, value).map_err(|_| invalid())?,
    }
    Ok(())
}
pub fn payload_bytes(doc: &G4epEverManifest) -> IpcResult<Vec<u8>> {
    let mut v = serde_json::to_value(doc).map_err(|_| invalid())?;
    v.as_object_mut()
        .ok_or_else(invalid)?
        .remove("authentication");
    let mut out = Vec::new();
    canonical(&v, &mut out)?;
    Ok(out)
}
pub fn payload_hash(doc: &G4epEverManifest) -> IpcResult<String> {
    Ok(hash(&payload_bytes(doc)?))
}
fn signing_message(doc: &G4epEverManifest) -> IpcResult<Vec<u8>> {
    let mut message = DOMAIN.to_vec();
    message.extend_from_slice(&digest(&doc.authentication.payload_sha256)?);
    Ok(message)
}
/// Signature validity is not publisher trust. Call require_origin with an out-of-band known key ID.
pub fn require_origin(doc: &G4epEverManifest, expected_key_id: &str) -> IpcResult<()> {
    validate(doc)?;
    if doc
        .authentication
        .signatures
        .iter()
        .any(|s| s.key_id == expected_key_id)
    {
        Ok(())
    } else {
        Err(integrity())
    }
}
pub fn sign(doc: &mut G4epEverManifest, key: &SigningKey) -> IpcResult<()> {
    validate(doc)?;
    doc.authentication.payload_sha256 = payload_hash(doc)?;
    let public = key.verifying_key().to_bytes();
    let key_id = content_id(&hash(&public));
    let signed = ManifestSignature {
        algorithm: "ed25519".into(),
        key_id: key_id.clone(),
        public_key: public.iter().map(|v| format!("{v:02x}")).collect(),
        signature: key
            .sign(&signing_message(doc)?)
            .to_bytes()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect(),
    };
    doc.authentication.signatures.retain(|s| s.key_id != key_id);
    doc.authentication.signatures.push(signed);
    doc.authentication
        .signatures
        .sort_by(|a, b| a.key_id.cmp(&b.key_id));
    validate(doc)?;
    Ok(())
}
fn validate_structure(doc: &G4epEverManifest) -> IpcResult<ValidatedManifest> {
    if doc.schema_version != 2 || doc.g4ep.protocol_version != 1 {
        return Err(unsupported());
    }
    if doc.schema != SCHEMA
        || doc.format != "game4-ever-manifest"
        || doc.representation != "g4ep_fragments"
        || !Uuid::parse_str(&doc.id).is_ok_and(|id| {
            id.get_version_num() == 4
                && id.get_variant() == uuid::Variant::RFC4122
                && id.to_string() == doc.id
        })
    {
        return Err(invalid());
    }
    manifests::validate(&doc.manifest).map_err(|_| invalid())?;
    if doc.manifest.files.len() != 1 || !doc.manifest.extensions.is_empty() {
        return Err(invalid());
    }
    let original = &doc.manifest.files[0];
    if original.path.len() > 255
        || original.path.contains('/')
        || !manifests::valid_path(&original.path)
        || doc.content_id != content_id(&original.sha256)
    {
        return Err(invalid());
    }
    let size = decimal(&original.size_bytes)?;
    let source_hash = digest(&original.sha256)?;
    if size == 0 && original.sha256 != hash(&[]) {
        return Err(integrity());
    }
    let c = &doc.g4ep.coding;
    let coding =
        CodingProfile::new(c.data_shards, c.parity_shards, c.shard_bytes).map_err(|_| invalid())?;
    encoder::validate_profile(coding).map_err(|_| invalid())?;
    if c.codec_id != encoder::CODEC_ID
        || c.algorithm != "reed-solomon"
        || c.field != "gf(2^16)"
        || c.field_polynomial != "0x1002d"
        || c.rate != "high"
        || c.symbol_layout != "leopard-64-byte-planes"
        || c.padding != "zero"
        || c.data_layout != "contiguous-systematic"
        || c.recovery_index_offset != 8
    {
        return Err(invalid());
    }
    let layout = coding.layout(size).map_err(|_| invalid())?;
    if doc.g4ep.groups.len() > MAX_GROUPS || doc.g4ep.groups.len() as u64 != layout.stripes {
        return Err(invalid());
    }
    let mut descriptors = Vec::with_capacity(layout.fragments as usize);
    let mut ids = HashSet::new();
    for (i, group) in doc.g4ep.groups.iter().enumerate() {
        let index = i as u64;
        let offset = index * coding.stripe_data_bytes();
        let length = (size - offset).min(coding.stripe_data_bytes());
        if group.id != format!("g{index:012}")
            || group.index != index.to_string()
            || group.original_offset != offset.to_string()
            || group.original_bytes != length.to_string()
            || group.fragments.len() != 12
        {
            return Err(invalid());
        }
        for (j, fragment) in group.fragments.iter().enumerate() {
            if fragment.index as usize != j
                || fragment.size_bytes != coding.shard_bytes().to_string()
                || fragment.id != content_id(&fragment.sha256)
            {
                return Err(invalid());
            }
            let bytes = digest(&fragment.sha256)?;
            ids.insert(fragment.id.as_str());
            descriptors.push(FragmentDescriptor {
                coordinate: FragmentCoordinate {
                    stripe: index,
                    shard: j as u8,
                },
                bytes: coding.shard_bytes(),
                sha256: Sha256Digest(bytes),
            });
        }
    }
    if doc.discovery.len() > MAX_DISCOVERY {
        return Err(invalid());
    }
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();
    for reference in &doc.discovery {
        let key = serde_json::to_string(reference).map_err(|_| invalid())?;
        if !seen.insert(key) {
            return Err(invalid());
        }
        match reference {
            DiscoveryReference::PrivatePeer { node_id, endpoint } => {
                let node = NodeId(digest(node_id)?);
                let address: SocketAddr = endpoint.parse().map_err(|_| invalid())?;
                if endpoint.contains('%') || address.to_string() != *endpoint {
                    return Err(invalid());
                }
                nodes.push(Peer { node, address });
            }
            DiscoveryReference::BittorrentFragment {
                fragment_id,
                info_hash_v1,
                magnet,
            } => {
                if !ids.contains(fragment_id.as_str())
                    || info_hash_v1.len() != 40
                    || !info_hash_v1
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    || *magnet != format!("magnet:?xt=urn:btih:{info_hash_v1}")
                {
                    return Err(invalid());
                }
            }
        }
    }
    if !nodes.is_empty() {
        PrivateNetwork::new("127.0.0.1:1".parse().map_err(|_| invalid())?, nodes)
            .map_err(|_| invalid())?;
    }
    if doc.authentication.canonicalization != CANONICALIZATION
        || doc.authentication.signatures.len() > 8
    {
        return Err(invalid());
    }
    let mut previous = None;
    for s in &doc.authentication.signatures {
        if s.algorithm != "ed25519" || previous.is_some_and(|p: &str| p >= s.key_id.as_str()) {
            return Err(invalid());
        }
        let key = decode_hex::<32>(&s.public_key)?;
        decode_hex::<64>(&s.signature)?;
        if s.key_id != content_id(&hash(&key)) {
            return Err(invalid());
        }
        previous = Some(s.key_id.as_str());
    }
    ValidatedManifest::new(PreservationManifest {
        codec: CodecId::new(&c.codec_id).map_err(|_| invalid())?,
        object: ObjectDescriptor {
            bytes: size,
            sha256: Sha256Digest(source_hash),
        },
        coding,
        fragments: descriptors,
    })
    .map_err(|_| invalid())
}
fn canonical_point(mut bytes: [u8; 32]) -> bool {
    bytes[31] &= 0x7f;
    let mut modulus = [0xff; 32];
    modulus[0] = 0xed;
    modulus[31] = 0x7f;
    bytes.iter().rev().cmp(modulus.iter().rev()).is_lt()
}
pub fn validate(doc: &G4epEverManifest) -> IpcResult<ValidatedManifest> {
    let manifest = validate_structure(doc)?;
    if doc.authentication.payload_sha256 != payload_hash(doc)? {
        return Err(integrity());
    }
    let message = signing_message(doc)?;
    for s in &doc.authentication.signatures {
        let key_bytes = decode_hex::<32>(&s.public_key)?;
        let signature_bytes = decode_hex::<64>(&s.signature)?;
        if !canonical_point(key_bytes)
            || !canonical_point(signature_bytes[..32].try_into().map_err(|_| invalid())?)
        {
            return Err(integrity());
        }
        let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| integrity())?;
        if key.is_weak() {
            return Err(integrity());
        }
        key.verify_strict(
            &message,
            &Signature::from_bytes(&decode_hex::<64>(&s.signature)?),
        )
        .map_err(|_| integrity())?;
    }
    Ok(manifest)
}
pub fn parse(bytes: &[u8]) -> IpcResult<G4epEverManifest> {
    ever::check_json(bytes)?;
    #[derive(Deserialize)]
    struct Bound<'a> {
        #[serde(borrow)]
        manifest: &'a serde_json::value::RawValue,
    }
    let raw: Bound<'_> = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if raw.manifest.get().len() > manifests::MAX_BYTES {
        return Err(IpcError::new(ErrorCode::G4eLimitExceeded));
    }
    let doc = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    validate(&doc)?;
    Ok(doc)
}
pub fn serialize(doc: &G4epEverManifest) -> IpcResult<Vec<u8>> {
    validate(doc)?;
    let value = serde_json::to_value(doc).map_err(|_| invalid())?;
    let mut bytes = Vec::new();
    canonical(&value, &mut bytes)?;
    if bytes.len() > ever::MAX_BYTES {
        return Err(IpcError::new(ErrorCode::G4eLimitExceeded));
    }
    Ok(bytes)
}
pub fn create(
    source: &ValidatedManifest,
    name: &str,
    discovery: Vec<DiscoveryReference>,
) -> IpcResult<G4epEverManifest> {
    if !manifests::valid_path(name) || name.contains('/') {
        return Err(invalid());
    }
    let m = source.manifest();
    if m.codec.as_str() != encoder::CODEC_ID {
        return Err(unsupported());
    }
    let mut descriptors = BTreeMap::new();
    for f in &m.fragments {
        descriptors.insert((f.coordinate.stripe, f.coordinate.shard), f);
    }
    let layout = m.coding.layout(m.object.bytes).map_err(|_| invalid())?;
    let mut groups = Vec::with_capacity(layout.stripes as usize);
    for index in 0..layout.stripes {
        let offset = index * m.coding.stripe_data_bytes();
        let fragments = (0..12)
            .map(|shard| {
                let f = descriptors.get(&(index, shard)).ok_or_else(invalid)?;
                let sha256 = local::hex(f.sha256);
                Ok(Fragment {
                    id: content_id(&sha256),
                    index: shard,
                    size_bytes: f.bytes.to_string(),
                    sha256,
                })
            })
            .collect::<IpcResult<Vec<_>>>()?;
        groups.push(Group {
            id: format!("g{index:012}"),
            index: index.to_string(),
            original_offset: offset.to_string(),
            original_bytes: (m.object.bytes - offset)
                .min(m.coding.stripe_data_bytes())
                .to_string(),
            fragments,
        });
    }
    let sha256 = local::hex(m.object.sha256);
    let mut doc = G4epEverManifest {
        schema: SCHEMA.into(),
        format: "game4-ever-manifest".into(),
        schema_version: 2,
        id: Uuid::new_v4().to_string(),
        representation: "g4ep_fragments".into(),
        content_id: content_id(&sha256),
        manifest: Manifest {
            schema: manifests::SCHEMA_ID.into(),
            format: "game4-manifest".into(),
            schema_version: 1,
            collection: Collection {
                id: Uuid::new_v4().to_string(),
                title: name.into(),
                platform: None,
                edition: None,
                language: None,
                declared_source: None,
                declared_license: None,
            },
            files: vec![ManifestFile {
                path: name.into(),
                size_bytes: m.object.bytes.to_string(),
                sha256,
            }],
            extensions: BTreeMap::new(),
        },
        g4ep: G4epDescription {
            protocol_version: 1,
            coding: Coding::from_profile(m.coding),
            groups,
        },
        discovery,
        authentication: Authentication {
            canonicalization: CANONICALIZATION.into(),
            payload_sha256: String::new(),
            signatures: vec![],
        },
    };
    doc.authentication.payload_sha256 = payload_hash(&doc)?;
    validate(&doc)?;
    Ok(doc)
}
pub fn write(path: &Path, doc: &G4epEverManifest) -> IpcResult<()> {
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("g4e"))
    {
        return Err(invalid());
    }
    let bytes = serialize(doc)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir().map_err(|_| invalid())?.join(parent)
    };
    crate::library::reject_ancestors(&parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| invalid())?;
    file.write_all(&bytes).map_err(|_| invalid())?;
    file.as_file().sync_all().map_err(|_| invalid())?;
    file.persist_noclobber(path)
        .map_err(|_| IpcError::new(ErrorCode::G4eWriteFailed))?;
    Ok(())
}
pub fn read(path: &Path) -> IpcResult<G4epEverManifest> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_err(|_| invalid())?.join(path)
    };
    let mut bytes = Vec::new();
    crate::library::open_source(&absolute)?
        .take((ever::MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    parse(&bytes)
}
