use super::{encoder::*, local, ports::*, types::*};
use reed_solomon_simd::{
    engine::{DefaultEngine, NoSimd},
    rate::{HighRateDecoder, HighRateEncoder, RateDecoder, RateEncoder},
};
use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
fn ctx() -> OperationContext {
    OperationContext {
        cancellation: CancellationToken::new(),
        deadline: Instant::now() + Duration::from_secs(120),
    }
}
fn profile() -> CodingProfile {
    CodingProfile::new(8, 4, 64 * 1024).unwrap()
}
fn pattern(n: usize) -> Vec<u8> {
    let mut state = 0x0123_4567_89ab_cdefu64;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 56) as u8
        })
        .collect()
}

#[derive(Default)]
struct MemorySink {
    shards: HashMap<FragmentCoordinate, Vec<u8>>,
}
impl FragmentSink for MemorySink {
    fn write_fragment(
        &mut self,
        descriptor: &FragmentDescriptor,
        reader: &mut dyn Read,
        ctx: &OperationContext,
    ) -> Result<()> {
        ctx.checkpoint()?;
        let mut b = Vec::new();
        reader.read_to_end(&mut b)?;
        assert_eq!(b.len(), descriptor.bytes as usize);
        assert_eq!(sha256(&b), descriptor.sha256);
        assert!(self.shards.insert(descriptor.coordinate, b).is_none());
        Ok(())
    }
}
fn encode(bytes: &[u8]) -> (ValidatedManifest, MemorySink) {
    let mut sink = MemorySink::default();
    let m = RsEncoder::default()
        .encode(
            &mut Cursor::new(bytes),
            bytes.len() as u64,
            profile(),
            &mut sink,
            &ctx(),
        )
        .unwrap();
    (m, sink)
}
#[test]
fn groups_are_bounded_systematic_and_final_padding_is_zero() {
    for n in [
        0,
        1,
        65535,
        65536,
        65537,
        8 * 65536,
        8 * 65536 + 1,
        17 * 65536 + 19,
    ] {
        let bytes = pattern(n);
        let (m, sink) = encode(&bytes);
        let m = m.manifest();
        assert_eq!(m.codec.as_str(), CODEC_ID);
        assert_eq!(m.object.sha256, sha256(&bytes));
        assert_eq!(
            m.fragments.len(),
            profile().layout(n as u64).unwrap().fragments as usize
        );
        let mut recovered = Vec::new();
        for group in 0..profile().layout(n as u64).unwrap().stripes {
            for index in 0..8 {
                recovered.extend_from_slice(
                    &sink.shards[&FragmentCoordinate {
                        stripe: group,
                        shard: index,
                    }],
                );
            }
        }
        assert_eq!(&recovered[..n], &bytes);
        assert!(recovered[n..].iter().all(|b| *b == 0));
    }
}
#[test]
fn any_eight_of_twelve_recover_every_original_shard_all_495_combinations() {
    let bytes = pattern(8 * 65536);
    let (_, sink) = encode(&bytes);
    let mut decoder = HighRateDecoder::new(8, 4, 65536, DefaultEngine::new(), None).unwrap();
    let mut count = 0;
    for mask in 0u16..(1 << 12) {
        if mask.count_ones() != 8 {
            continue;
        }
        for index in 0..12u8 {
            if mask & (1 << index) == 0 {
                continue;
            }
            let shard = &sink.shards[&FragmentCoordinate {
                stripe: 0,
                shard: index,
            }];
            if index < 8 {
                decoder.add_original_shard(index as usize, shard).unwrap();
            } else {
                decoder
                    .add_recovery_shard((index - 8) as usize, shard)
                    .unwrap();
            }
        }
        let decoded = decoder.decode().unwrap();
        for index in 0..8u8 {
            let expected = &sink.shards[&FragmentCoordinate {
                stripe: 0,
                shard: index,
            }];
            if mask & (1 << index) == 0 {
                assert_eq!(decoded.restored_original(index as usize).unwrap(), expected);
            } else {
                assert!(decoded.restored_original(index as usize).is_none());
            }
        }
        drop(decoded);
        count += 1;
    }
    assert_eq!(count, 495);
    let mut decoder = HighRateDecoder::new(8, 4, 65536, DefaultEngine::new(), None).unwrap();
    for i in 0..7 {
        decoder
            .add_original_shard(i, &bytes[i * 65536..(i + 1) * 65536])
            .unwrap();
    }
    assert!(decoder.decode().is_err());
}
#[test]
fn fixed_high_rate_mapping_agrees_with_scalar_engine() {
    let bytes = pattern(8 * 65536);
    let (_, sink) = encode(&bytes);
    let mut reference = HighRateEncoder::new(8, 4, 65536, NoSimd::new(), None).unwrap();
    for shard in bytes.as_chunks::<65536>().0 {
        reference.add_original_shard(shard).unwrap();
    }
    let parity = reference.encode().unwrap();
    for i in 0..4 {
        assert_eq!(
            parity.recovery(i).unwrap(),
            &sink.shards[&FragmentCoordinate {
                stripe: 0,
                shard: (8 + i) as u8
            }]
        );
    }
    let hashes: (Vec<_>, Vec<_>) = (
        (0..8)
            .map(|i| {
                local::hex(sha256(
                    &sink.shards[&FragmentCoordinate {
                        stripe: 0,
                        shard: i,
                    }],
                ))
            })
            .collect(),
        (8..12)
            .map(|i| {
                local::hex(sha256(
                    &sink.shards[&FragmentCoordinate {
                        stripe: 0,
                        shard: i,
                    }],
                ))
            })
            .collect(),
    );
    let actual = serde_json::json!({"codec":CODEC_ID,"shardBytes":65536,"pattern":"xorshift64-star, state 0x0123456789abcdef, shifts left13/right7/left17, multiplier 0x2545f4914f6cdd1d, output high byte","objectSha256":local::hex(sha256(&bytes)),"dataSha256":hashes.0,"paritySha256":hashes.1});
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/g4ep/codec-vector-v1.json"
    ))
    .unwrap();
    assert_eq!(actual, expected);
}
struct Chunked {
    inner: Cursor<Vec<u8>>,
    calls: usize,
}
impl Read for Chunked {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        assert!(b.len() <= IO_BYTES);
        self.calls += 1;
        if self.calls == 1 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let n = b.len().min(37);
        self.inner.read(&mut b[..n])
    }
}
#[test]
fn short_reads_interrupts_and_bounded_read_requests_are_supported() {
    let bytes = pattern(9 * 65536 + 3);
    let mut source = Chunked {
        inner: Cursor::new(bytes.clone()),
        calls: 0,
    };
    let m = RsEncoder::default()
        .encode(
            &mut source,
            bytes.len() as u64,
            profile(),
            &mut MemorySink::default(),
            &ctx(),
        )
        .unwrap();
    assert_eq!(m.manifest().object.sha256, sha256(&bytes));
    assert!(source.calls > 100);
}
struct IgnoringSink;
impl FragmentSink for IgnoringSink {
    fn write_fragment(
        &mut self,
        _: &FragmentDescriptor,
        _: &mut dyn Read,
        _: &OperationContext,
    ) -> Result<()> {
        Ok(())
    }
}
struct FailingSink;
impl FragmentSink for FailingSink {
    fn write_fragment(
        &mut self,
        _: &FragmentDescriptor,
        _: &mut dyn Read,
        _: &OperationContext,
    ) -> Result<()> {
        Err(io::Error::from(io::ErrorKind::StorageFull).into())
    }
}
#[test]
fn short_long_failing_input_and_incomplete_or_failed_sinks_never_succeed() {
    for (data, size) in [(vec![1], 2), (vec![1, 2], 1), (vec![1], 0)] {
        assert!(matches!(
            RsEncoder::default().encode(
                &mut Cursor::new(data),
                size,
                profile(),
                &mut MemorySink::default(),
                &ctx()
            ),
            Err(Error::SourceLengthMismatch)
        ));
    }
    assert!(matches!(
        RsEncoder::default().encode(
            &mut Cursor::new(vec![1]),
            1,
            profile(),
            &mut IgnoringSink,
            &ctx()
        ),
        Err(Error::IncompleteSink)
    ));
    assert!(matches!(
        RsEncoder::default().encode(
            &mut Cursor::new(vec![1]),
            1,
            profile(),
            &mut FailingSink,
            &ctx()
        ),
        Err(Error::Io(_))
    ));
    struct FailedRead;
    impl Read for FailedRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::PermissionDenied.into())
        }
    }
    assert!(matches!(
        RsEncoder::default().encode(
            &mut FailedRead,
            1,
            profile(),
            &mut MemorySink::default(),
            &ctx()
        ),
        Err(Error::Io(_))
    ));
}
#[test]
fn cancellation_deadline_and_unsupported_profiles_fail_before_success() {
    let expired = OperationContext {
        cancellation: CancellationToken::new(),
        deadline: Instant::now(),
    };
    assert!(matches!(
        RsEncoder::default().encode(
            &mut Cursor::new(vec![1]),
            1,
            profile(),
            &mut MemorySink::default(),
            &expired
        ),
        Err(Error::Deadline)
    ));
    for coding in [
        CodingProfile::new(7, 4, 65536).unwrap(),
        CodingProfile::new(8, 4, 65537).unwrap(),
    ] {
        assert!(matches!(
            RsEncoder::default().encode(
                &mut Cursor::new(vec![1]),
                1,
                coding,
                &mut MemorySink::default(),
                &ctx()
            ),
            Err(Error::InvalidProfile)
        ));
    }
    let context = ctx();
    let cancel = context.cancellation.clone();
    let mut progress = Vec::new();
    let result = RsEncoder::default().encode_with_progress(
        &mut Cursor::new(pattern(16 * 65536)),
        16 * 65536,
        profile(),
        &mut MemorySink::default(),
        &context,
        &mut |p| {
            progress.push(p);
            cancel.cancel();
        },
    );
    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].written_fragments, 12);
}
#[test]
fn local_fragments_are_independent_verified_no_clobber_and_originals_unchanged() {
    let d = tempfile::tempdir().unwrap();
    let source = d.path().join("synthetic Ω.input");
    let root = d.path().join("encoded");
    let bytes = pattern(9 * 65536 + 7);
    std::fs::write(&source, &bytes).unwrap();
    let before = std::fs::metadata(&source).unwrap();
    let m = local::encode_file(&source, &root, profile(), &ctx(), &mut |_| {}).unwrap();
    assert_eq!(m.manifest().fragments.len(), 24);
    let read = local::verify_bundle(&root, &ctx()).unwrap();
    assert_eq!(read.manifest().object, m.manifest().object);
    assert!(!root.join(local::INCOMPLETE_FILE).exists());
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&source).unwrap().modified().unwrap(),
        before.modified().unwrap()
    );
    assert!(matches!(
        local::encode_file(&source, &root, profile(), &ctx(), &mut |_| {}),
        Err(Error::DestinationExists)
    ));
    let f = &m.manifest().fragments[23];
    let path = root.join(local::fragment_name(f.coordinate));
    let mut corrupted = std::fs::read(&path).unwrap();
    corrupted[0] ^= 1;
    std::fs::write(&path, corrupted).unwrap();
    assert!(matches!(
        local::verify_fragment(&root, f, &ctx()),
        Err(Error::IntegrityMismatch)
    ));
}
#[test]
fn incomplete_cancelled_and_hostile_metadata_bundles_are_rejected() {
    let d = tempfile::tempdir().unwrap();
    let source = d.path().join("synthetic.input");
    std::fs::write(&source, pattern(16 * 65536)).unwrap();
    let root = d.path().join("cancelled");
    let context = ctx();
    let cancel = context.cancellation.clone();
    assert!(matches!(
        local::encode_file(&source, &root, profile(), &context, &mut |_| cancel
            .cancel()),
        Err(Error::Cancelled)
    ));
    assert!(root.join(local::INCOMPLETE_FILE).exists());
    assert!(!root.join(local::METADATA_FILE).exists());
    assert!(local::read_manifest(&root, &ctx()).is_err());
    let root = d.path().join("valid");
    local::encode_file(&source, &root, profile(), &ctx(), &mut |_| {}).unwrap();
    let path = root.join(local::METADATA_FILE);
    let original = std::fs::read(&path).unwrap();
    for mutation in [0, 1, 2, 3] {
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match mutation {
            0 => value["codecId"] = serde_json::json!("rs-gf256-draft"),
            1 => value["fragments"][0]["group"] = serde_json::json!("../escape"),
            2 => value["fragments"][1] = value["fragments"][0].clone(),
            _ => value["version"] = serde_json::json!(2),
        }
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(local::read_manifest(&root, &ctx()).is_err());
    }
}
