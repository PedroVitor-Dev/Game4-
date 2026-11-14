//! Local experimental output adapter for step 2. Not a StorageNode or public G4EP format.
use super::{
    encoder::{self, EncodingProgress, IO_BYTES, RsEncoder},
    ports::FragmentSink,
    types::*,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
pub const METADATA_FILE: &str = "encoding.g4ep-experiment.json";
pub const INCOMPLETE_FILE: &str = ".game4-g4ep-incomplete";
pub const MAX_METADATA_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LOCAL_FRAGMENTS: u64 = 65_536;
const FORMAT: &str = "game4-g4ep-encoding-experiment";
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    format: String,
    version: u32,
    codec_id: String,
    original: Original,
    coding: Coding,
    fragments: Vec<Record>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Original {
    bytes: String,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Coding {
    data_shards: u8,
    parity_shards: u8,
    shard_bytes: u32,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    group: String,
    index: u8,
    bytes: u32,
    sha256: String,
}
pub fn hex(digest: Sha256Digest) -> String {
    digest.0.iter().map(|v| format!("{v:02x}")).collect()
}
fn digest(value: &str) -> Result<Sha256Digest> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::InvalidBundle);
    }
    let mut b = [0u8; 32];
    for (i, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        b[i] = u8::from_str_radix(
            std::str::from_utf8(pair).map_err(|_| Error::InvalidBundle)?,
            16,
        )
        .map_err(|_| Error::InvalidBundle)?;
    }
    Ok(Sha256Digest(b))
}
fn decimal(value: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|n| n.to_string() == value)
        .ok_or(Error::InvalidBundle)
}
pub fn fragment_name(coordinate: FragmentCoordinate) -> String {
    format!("g{:012}-s{:02}.g4frag", coordinate.stripe, coordinate.shard)
}
fn guard(path: &Path) -> Result<()> {
    crate::library::reject_ancestors(path).map_err(|_| Error::InvalidBundle)
}
fn open(path: &Path) -> Result<File> {
    crate::library::open_source(path).map_err(|_| Error::InvalidBundle)
}
fn abs(path: &Path) -> Result<PathBuf> {
    Ok(if path.is_absolute() {
        path.into()
    } else {
        std::env::current_dir()?.join(path)
    })
}
fn encode_document(manifest: &ValidatedManifest) -> Document {
    let m = manifest.manifest();
    Document {
        format: FORMAT.into(),
        version: 1,
        codec_id: m.codec.as_str().into(),
        original: Original {
            bytes: m.object.bytes.to_string(),
            sha256: hex(m.object.sha256),
        },
        coding: Coding {
            data_shards: m.coding.data_shards(),
            parity_shards: m.coding.parity_shards(),
            shard_bytes: m.coding.shard_bytes(),
        },
        fragments: m
            .fragments
            .iter()
            .map(|f| Record {
                group: f.coordinate.stripe.to_string(),
                index: f.coordinate.shard,
                bytes: f.bytes,
                sha256: hex(f.sha256),
            })
            .collect(),
    }
}
fn decode_document(doc: Document) -> Result<ValidatedManifest> {
    if doc.format != FORMAT
        || doc.version != 1
        || doc.codec_id != encoder::CODEC_ID
        || doc.fragments.len() as u64 > MAX_LOCAL_FRAGMENTS
    {
        return Err(Error::InvalidBundle);
    }
    let coding = CodingProfile::new(
        doc.coding.data_shards,
        doc.coding.parity_shards,
        doc.coding.shard_bytes,
    )?;
    encoder::validate_profile(coding)?;
    let fragments = doc
        .fragments
        .into_iter()
        .map(|f| {
            Ok(FragmentDescriptor {
                coordinate: FragmentCoordinate {
                    stripe: decimal(&f.group)?,
                    shard: f.index,
                },
                bytes: f.bytes,
                sha256: digest(&f.sha256)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    ValidatedManifest::new(PreservationManifest {
        codec: CodecId::new(&doc.codec_id)?,
        object: ObjectDescriptor {
            bytes: decimal(&doc.original.bytes)?,
            sha256: digest(&doc.original.sha256)?,
        },
        coding,
        fragments,
    })
}
struct FileSink {
    root: PathBuf,
}
impl FileSink {
    fn new(destination: &Path) -> Result<Self> {
        let destination = abs(destination)?;
        let parent = destination.parent().ok_or(Error::InvalidBundle)?;
        guard(parent)?;
        let root = parent
            .canonicalize()?
            .join(destination.file_name().ok_or(Error::InvalidBundle)?);
        fs::create_dir(&root).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                Error::DestinationExists
            } else {
                e.into()
            }
        })?;
        guard(&root)?;
        File::options()
            .write(true)
            .create_new(true)
            .open(root.join(INCOMPLETE_FILE))?
            .sync_all()?;
        Ok(Self { root })
    }
    fn complete(&self, manifest: &ValidatedManifest, ctx: &OperationContext) -> Result<()> {
        ctx.checkpoint()?;
        guard(&self.root)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&self.root)?;
        {
            let mut writer = BufWriter::new(temporary.as_file_mut());
            serde_json::to_writer(&mut writer, &encode_document(manifest))
                .map_err(|_| Error::InvalidBundle)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
        if temporary.as_file().metadata()?.len() > MAX_METADATA_BYTES as u64 {
            return Err(Error::MetadataLimit);
        }
        temporary.as_file().sync_all()?;
        ctx.checkpoint()?;
        temporary
            .persist_noclobber(self.root.join(METADATA_FILE))
            .map_err(|e| Error::Io(e.error))?;
        // Last state transition: readers reject a bundle while the incomplete marker exists.
        guard(&self.root)?;
        fs::remove_file(self.root.join(INCOMPLETE_FILE))?;
        Ok(())
    }
}
impl FragmentSink for FileSink {
    fn write_fragment(
        &mut self,
        expected: &FragmentDescriptor,
        reader: &mut dyn Read,
        ctx: &OperationContext,
    ) -> Result<()> {
        ctx.checkpoint()?;
        guard(&self.root)?;
        let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
        let mut hash = Sha256::new();
        let mut remaining = usize::try_from(expected.bytes).map_err(|_| Error::InvalidBundle)?;
        let mut buffer = [0u8; IO_BYTES];
        while remaining > 0 {
            let n = remaining.min(buffer.len());
            encoder::read_exact_checked(reader, &mut buffer[..n], ctx)?;
            file.write_all(&buffer[..n])?;
            hash.update(&buffer[..n]);
            remaining -= n;
        }
        encoder::require_eof(reader, ctx)?;
        if Sha256Digest(hash.finalize().into()) != expected.sha256 {
            return Err(Error::IntegrityMismatch);
        }
        file.as_file().sync_all()?;
        ctx.checkpoint()?;
        guard(&self.root)?;
        file.persist_noclobber(self.root.join(fragment_name(expected.coordinate)))
            .map_err(|e| Error::Io(e.error))?;
        Ok(())
    }
}
pub fn encode_file(
    source: &Path,
    destination: &Path,
    profile: CodingProfile,
    ctx: &OperationContext,
    on_progress: &mut dyn FnMut(EncodingProgress),
) -> Result<ValidatedManifest> {
    ctx.checkpoint()?;
    encoder::validate_profile(profile)?;
    let source = abs(source)?;
    let mut input = open(&source)?;
    let before = input.metadata()?;
    let layout = profile.layout(before.len())?;
    if layout.fragments > MAX_LOCAL_FRAGMENTS {
        return Err(Error::MetadataLimit);
    }
    let mut output = FileSink::new(destination)?;
    let result = RsEncoder::default().encode_with_progress(
        &mut input,
        before.len(),
        profile,
        &mut output,
        ctx,
        on_progress,
    )?;
    let after = input.metadata()?;
    guard(&source)?;
    if before.len() != after.len()
        || before.modified()? != after.modified()?
        || before.created().ok() != after.created().ok()
    {
        return Err(Error::SourceChanged);
    }
    output.complete(&result, ctx)?;
    Ok(result)
}
pub fn read_manifest(root: &Path, ctx: &OperationContext) -> Result<ValidatedManifest> {
    ctx.checkpoint()?;
    let root = abs(root)?;
    guard(&root)?;
    match fs::symlink_metadata(root.join(INCOMPLETE_FILE)) {
        Ok(_) => return Err(Error::InvalidBundle),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let mut bytes = Vec::new();
    open(&root.join(METADATA_FILE))?
        .take((MAX_METADATA_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_METADATA_BYTES {
        return Err(Error::MetadataLimit);
    }
    let doc = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidBundle)?;
    let manifest = decode_document(doc)?;
    ctx.checkpoint()?;
    Ok(manifest)
}
pub fn verify_fragment(
    root: &Path,
    descriptor: &FragmentDescriptor,
    ctx: &OperationContext,
) -> Result<()> {
    ctx.checkpoint()?;
    let mut file = open(&abs(root)?.join(fragment_name(descriptor.coordinate)))?;
    if file.metadata()?.len() != u64::from(descriptor.bytes) {
        return Err(Error::IntegrityMismatch);
    }
    let mut buffer = [0u8; IO_BYTES];
    let mut hash = Sha256::new();
    let mut remaining = descriptor.bytes as usize;
    while remaining > 0 {
        let n = remaining.min(buffer.len());
        encoder::read_exact_checked(&mut file, &mut buffer[..n], ctx)?;
        hash.update(&buffer[..n]);
        remaining -= n;
    }
    encoder::require_eof(&mut file, ctx)?;
    if Sha256Digest(hash.finalize().into()) != descriptor.sha256 {
        return Err(Error::IntegrityMismatch);
    }
    ctx.checkpoint()?;
    Ok(())
}
pub fn verify_bundle(root: &Path, ctx: &OperationContext) -> Result<ValidatedManifest> {
    let manifest = read_manifest(root, ctx)?;
    for fragment in &manifest.manifest().fragments {
        verify_fragment(root, fragment, ctx)?;
    }
    Ok(manifest)
}
