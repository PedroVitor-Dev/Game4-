# G4EP step 2 — streaming Reed–Solomon encoding

Implemented November 14, 2026. `g4ep::encoder::RsEncoder` implements the step-1 Encoder port and the offline `g4ep` example writes independently identifiable/verifiable fragment files. This is real local byte processing, not a visual demonstration. No database, networking, StorageNode, automatic repair or desktop IPC action is started.

## Codec selection and compatibility

The dependency is pinned to **reed-solomon-simd 3.1.0** in Cargo.toml/Cargo.lock. Inspection of the upstream repository, published crate, API and source found runtime SIMD dispatch with a scalar Rust fallback, reusable per-group workspaces and MIT/BSD-3-Clause notices. The latter are retained in [the third-party notices](../licenses/reed-solomon-simd-3.1.0.txt). Pinning and golden vectors constrain compatibility; this is not a claim that the dependency has been independently audited.

The actual codec is **Leopard Reed–Solomon over GF(2^16), explicit high rate, 8 data + 4 recovery shards**. Its registration ID is `g4ep-rs16-leopard-high-8-4-v1`. The GF(256) option in the step-1 design was a proposal, not an implemented wire format; it is not compatible with these bytes and is not reused as the ID. High-rate and low-rate matrices must not be mixed. DefaultEngine chooses CPU execution paths, not coding rate. Scalar/default-engine parity equality and the fixed [digest vector](codec-vector-v1.json) protect the byte mapping. Before any codec upgrade, verify or deliberately version this mapping.

Original shard indexes are 0–7, in contiguous source order. Recovery indexes are 8–11 in G4EP, mapped to library recovery indexes 0–3. Each group is encoded independently; data is systematic (the original padded bytes), with full-shard SHA-256 covering padding for both data and parity. The whole-object SHA-256 covers only the actual original bytes.

The encoder accepts only 8+4 profiles with shard size between 64 KiB and 4 MiB and a multiple of 64. Default: 1 MiB per shard, hence **8 MiB of source data per group** and 12 MiB encoded per group. Profile layout limits from step 1 still apply. Empty input has the standard empty-file hash and zero fragments. An exact group has no extra trailing group. A partial final group fills missing bytes with zero; the original length determines padding removal by a future decoder.

## Streaming, integrity and failure behavior

One reusable shard buffer feeds the library workspace, then immediately streams each data fragment to its sink. Parity buffers are borrowed from the library result, emitted, then dropped to reset/reuse the workspace for the next group. Source read requests are at most 64 KiB and support short/interrupted reads. A complete-file payload buffer and 12 whole-file-size shards are never allocated.

Memory grows with configured group/workspace size and bounded fragment metadata, not the complete source payload. For the default 8+4 profile, the pinned high-rate encoder's workspace is 8 MiB plus the 1 MiB input buffer, hash tables/metadata/runtime and sink overhead. File sinks use another 64 KiB I/O buffer. This implementation detail is not a fixed process-memory guarantee; the actual Windows 700 MB run is recorded below.

Cancellation/deadline checks run before/after reads, hashes, coding, writes and group progress callbacks. A native library encode call and a blocking filesystem operation are not forcibly interrupted mid-call; checks occur at bounded operation boundaries. A future Tauri adapter must execute this blocking pipeline on `spawn_blocking`; the standalone command intentionally occupies its console until completion. Progress reports count original bytes, completed groups and written fragments distinctly.

A short source, extra source byte, I/O/storage error, wrong profile, cancelled context or incompletely consumed sink produces an error, never a successful manifest. The file adapter opens originals read-only using existing link/junction/source checks and Windows write/delete sharing restrictions, compares source metadata before publication and rejects changes it observes. It never converts or modifies the original. Portable protection against arbitrary hostile filesystem races is not claimed.

## Offline CLI

Run from the repository in PowerShell using `npm.cmd`:

```powershell
npm.cmd run g4ep -- encode "D:\Tests\authorized-input.iso" --output "D:\Tests\new-g4ep-output"
npm.cmd run g4ep -- verify "D:\Tests\new-g4ep-output"
npm.cmd run g4ep -- verify-fragment "D:\Tests\new-g4ep-output" 0 8
```

The third command checks parity fragment 8 of group 0 independently against its expected digest/size. Input extensions have no effect on encoding. Output must be a **new directory** with an existing parent; an existing directory/file is never overwritten. Non-ASCII input paths are covered by tests. Every run has a one-hour operation deadline.

Change group size explicitly (bytes, divisible by eight, yielding supported shard sizes):

```powershell
npm.cmd run g4ep -- encode "D:\Tests\authorized-input.zip" --output "D:\Tests\another-output" --group-bytes 4194304
```

This gives eight 512 KiB data shards and four 512 KiB parity shards per group. The default 8388608-byte group is eight 1 MiB data shards plus four parity shards. The JSON success report includes codec, actual byte counts, groups, fragments, original SHA-256, elapsed time and `networkStarted: false`.

## Local output contract (experimental, not the public G4EP manifest)

- Raw fragment files: `g000000000000-s00.g4frag` through `...-s11.g4frag`, then the next group. File content is exactly one encoded shard, without a custom header or compression. Coordinates identify the file; the metadata binds its SHA-256 and byte length. Hashes are integrity checks, not authorization/signatures.
- `encoding.g4ep-experiment.json` is an explicitly experimental local inventory with `format: game4-g4ep-encoding-experiment`, `version: 1`, exact codec ID, original identity, geometry and all fragment coordinates/hashes. Counts that may be large are canonical decimal strings. It contains no original filesystem path. No .g4/.g4e/.g4lib format changes or Windows associations are made.
- A `.game4-g4ep-incomplete` marker is created first. Fragment writes verify length/hash, sync a temporary file and publish without clobbering. Inventory is synced/published last; only then is the marker removed. Readers reject marked directories. This is an application completion boundary, not a full cross-file/power-loss transaction guarantee.
- Failure leaves clearly marked partial output for inspection; it does not delete originals or automatically delete/reuse the destination. Choose a new directory for a retry. Resumable encoding and GC belong to later lifecycle work.
- The experimental adapter caps inventory at 65,536 fragment records and 16 MiB JSON. Geometry is checked before encoding. Inventory parsing rejects wrong codec/version, malformed identities, duplicate/out-of-range coordinates and wrong declared lengths; filenames are derived internally, never accepted from JSON. Missing/changed/truncated/extra fragment bytes fail verification. No network request occurs while parsing.

The internal Rust manifest remains independent from public-format serialization. This local inventory exists to make step 2 usable and testable across CLI invocations; a public authenticated protocol descriptor still needs a later specification. Library backups do not include these output directories. Encoding and `verify` do not reconstruct the original or prove remote custody.

## Verification evidence

Eight new native test cases cover:

- Empty/odd/exact/multiple-group sources, original SHA-256, systematic byte order and zero padding.
- **All 495 possible sets of eight distinct shards out of twelve**, with non-linear deterministic data, restore every missing data shard using the selected library decoder; seven are rejected. This is a codec-level recovery proof, not a delivered Decoder service/UI or 700 MB reconstruction command.
- Exact scalar/SIMD parity equality and a pinned codec/hash vector.
- 64 KiB read bounds, interrupted/short reads, wrong source lengths, source errors, failed/ignored sinks and unsupported geometry.
- Cancellation/deadlines, incomplete publication, original content/mtime retention, existing-destination refusal, fragment corruption and hostile inventories.

A separate local Windows debug CLI run encoded a real, synthetic authorized **700,000,000-byte** file. It produced **84 groups, 1,008 fragments × 1,048,576 bytes = 1,056,964,608 bytes** before metadata. Original SHA-256 agreed with Python's independently generated hash. The CLI reread and checked all stored fragment hashes. Approximate process peak working set was 28.1 MiB for encoding and 5.9 MiB for verification; encoding took 73.33 s and verification 22.23 s on this development machine under development workload. These are one-run observations, not throughput or memory promises for other hardware/build profiles. See [machine-readable evidence](../evaluations/g4ep-encoder-2026-11-14.json).

No real two-PC transfer, public DHT, automatic reconstruction/repair, encryption or storage-node enrollment was performed in this step. Those remain separate acceptance gates; four parity shards do not imply tolerance of four failed computers.

## Primary upstream sources

- [Published crate/API 3.1.0](https://docs.rs/reed-solomon-simd/3.1.0/reed_solomon_simd/) and its pinned registry source, including `rate/rate_high.rs`, workspace handling and algorithm documentation.
- [Upstream repository](https://github.com/AndersTrier/reed-solomon-simd) and distributed Cargo.toml/license notices.
- [Encoder API](https://docs.rs/reed-solomon-simd/3.1.0/reed_solomon_simd/struct.ReedSolomonEncoder.html) and [decoder API](https://docs.rs/reed-solomon-simd/3.1.0/reed_solomon_simd/struct.ReedSolomonDecoder.html) for shard indexing and workspace lifetime.
