//! Systematic RS(8,4), Leopard GF(2^16), explicit high-rate mapping.
use super::{
    ports::{Encoder, FragmentSink},
    types::*,
};
use reed_solomon_simd::{
    engine::DefaultEngine,
    rate::{HighRateEncoder, RateEncoder},
};
use sha2::{Digest, Sha256};
use std::io::{self, Cursor, Read};
pub const CODEC_ID: &str = "g4ep-rs16-leopard-high-8-4-v1";
pub const DATA_SHARDS: u8 = 8;
pub const PARITY_SHARDS: u8 = 4;
pub const DEFAULT_SHARD_BYTES: u32 = 1024 * 1024;
pub const IO_BYTES: usize = 64 * 1024;
#[derive(Debug)]
pub struct RsEncoder {
    id: CodecId,
}
impl Default for RsEncoder {
    fn default() -> Self {
        Self {
            id: CodecId::new(CODEC_ID).expect("static codec ID"),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct EncodingProgress {
    pub source_bytes: u64,
    pub total_source_bytes: u64,
    pub completed_groups: u64,
    pub total_groups: u64,
    pub written_fragments: u64,
}
pub fn default_profile() -> CodingProfile {
    CodingProfile::new(DATA_SHARDS, PARITY_SHARDS, DEFAULT_SHARD_BYTES)
        .expect("static coding profile")
}
pub fn validate_profile(profile: CodingProfile) -> Result<()> {
    if profile.data_shards() != DATA_SHARDS
        || profile.parity_shards() != PARITY_SHARDS
        || !profile.shard_bytes().is_multiple_of(64)
    {
        return Err(Error::InvalidProfile);
    }
    Ok(())
}
fn read_some(reader: &mut dyn Read, buffer: &mut [u8], ctx: &OperationContext) -> Result<usize> {
    loop {
        ctx.checkpoint()?;
        match reader.read(buffer) {
            Ok(n) => {
                ctx.checkpoint()?;
                return Ok(n);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
}
pub(crate) fn require_eof(reader: &mut dyn Read, ctx: &OperationContext) -> Result<()> {
    if read_some(reader, &mut [0u8; 1], ctx)? != 0 {
        return Err(Error::SourceLengthMismatch);
    }
    Ok(())
}
pub(crate) fn read_exact_checked(
    reader: &mut dyn Read,
    mut buffer: &mut [u8],
    ctx: &OperationContext,
) -> Result<()> {
    while !buffer.is_empty() {
        let count = buffer.len().min(IO_BYTES);
        let n = read_some(reader, &mut buffer[..count], ctx)?;
        if n == 0 {
            return Err(Error::SourceLengthMismatch);
        }
        buffer = &mut buffer[n..];
    }
    Ok(())
}
pub fn sha256(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest(Sha256::digest(bytes).into())
}
fn emit(
    output: &mut dyn FragmentSink,
    group: u64,
    index: u8,
    data: &[u8],
    ctx: &OperationContext,
) -> Result<FragmentDescriptor> {
    ctx.checkpoint()?;
    let descriptor = FragmentDescriptor {
        coordinate: FragmentCoordinate {
            stripe: group,
            shard: index,
        },
        bytes: data.len() as u32,
        sha256: sha256(data),
    };
    let mut reader = Cursor::new(data);
    output.write_fragment(&descriptor, &mut reader, ctx)?;
    ctx.checkpoint()?;
    if reader.position() != data.len() as u64 {
        return Err(Error::IncompleteSink);
    }
    Ok(descriptor)
}
impl RsEncoder {
    pub fn encode_with_progress(
        &self,
        source: &mut dyn Read,
        declared_size: u64,
        coding: CodingProfile,
        output: &mut dyn FragmentSink,
        ctx: &OperationContext,
        on_progress: &mut dyn FnMut(EncodingProgress),
    ) -> Result<ValidatedManifest> {
        ctx.checkpoint()?;
        validate_profile(coding)?;
        let layout = coding.layout(declared_size)?;
        let mut descriptors = Vec::with_capacity(layout.fragments as usize);
        let mut original_hash = Sha256::new();
        let mut processed = 0u64;
        // One library workspace + a single reusable shard; no complete-file payload buffer.
        if layout.stripes > 0 {
            let mut rs = HighRateEncoder::new(
                usize::from(DATA_SHARDS),
                usize::from(PARITY_SHARDS),
                coding.shard_bytes() as usize,
                DefaultEngine::new(),
                None,
            )?;
            let mut shard = vec![0u8; coding.shard_bytes() as usize];
            for group in 0..layout.stripes {
                ctx.checkpoint()?;
                for index in 0..DATA_SHARDS {
                    shard.fill(0);
                    let actual =
                        (declared_size - processed).min(u64::from(coding.shard_bytes())) as usize;
                    read_exact_checked(source, &mut shard[..actual], ctx)?;
                    original_hash.update(&shard[..actual]);
                    processed += actual as u64;
                    ctx.checkpoint()?;
                    rs.add_original_shard(&shard)?;
                    descriptors.push(emit(output, group, index, &shard, ctx)?);
                }
                ctx.checkpoint()?;
                let parity = rs.encode()?;
                ctx.checkpoint()?;
                for index in 0..PARITY_SHARDS {
                    let bytes = parity
                        .recovery(usize::from(index))
                        .ok_or(Error::InvalidManifest)?;
                    descriptors.push(emit(output, group, DATA_SHARDS + index, bytes, ctx)?);
                }
                drop(parity); // Resets the same workspace before the next group.
                on_progress(EncodingProgress {
                    source_bytes: processed,
                    total_source_bytes: declared_size,
                    completed_groups: group + 1,
                    total_groups: layout.stripes,
                    written_fragments: descriptors.len() as u64,
                });
            }
        }
        require_eof(source, ctx)?;
        ctx.checkpoint()?;
        ValidatedManifest::new(PreservationManifest {
            codec: self.id.clone(),
            object: ObjectDescriptor {
                bytes: declared_size,
                sha256: Sha256Digest(original_hash.finalize().into()),
            },
            coding,
            fragments: descriptors,
        })
    }
}
impl Encoder for RsEncoder {
    fn codec_id(&self) -> &CodecId {
        &self.id
    }
    fn encode(
        &self,
        source: &mut dyn Read,
        declared_size: u64,
        coding: CodingProfile,
        output: &mut dyn FragmentSink,
        ctx: &OperationContext,
    ) -> Result<ValidatedManifest> {
        self.encode_with_progress(source, declared_size, coding, output, ctx, &mut |_| {})
    }
}
