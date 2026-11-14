# Game4 Ever Manifest v2: G4EP reconstruction descriptor

Status: implemented experimental private-network MVP. Schema v2 extends the `.g4e` family; G4EP protocol version remains 1. The normative field structure is [v2.schema.json](v2.schema.json); semantic requirements below also apply. Public unsigned and signed examples are in `examples/g4ep-fragments-v2*.g4e`. They describe synthetic bytes only.

## Compatibility and identity

Readers dispatch explicitly on `schemaVersion`: 1 retains its existing original-file/package behavior, 2 requires `representation: g4ep_fragments`. Unknown versions fail closed. Existing v1 payload/torrent operations reject v2 rather than treating parity fragments as an original game. No `.g4`, `.g4lib`, SQLite or backup migration is required.

`contentId` is `urn:sha256:` followed by the lowercase SHA-256 of the original bytes. The embedded v1 `manifest` contains exactly one original file with its portable base name, decimal byte size and SHA-256. Its extensions must be empty. Names contain no directory separators and are at most 255 UTF-8 bytes. Existing portable-name restrictions also apply. Empty originals require SHA-256 of the empty byte sequence. Root and collection IDs are UUIDv4 values; creating a fresh descriptor may produce fresh IDs.

The descriptor contains metadata only: no original bytes, fragment bytes, ROM, encryption key or local absolute path. Unknown fields are rejected. Byte sizes and offsets use canonical unsigned decimal strings, without leading zeroes except `0`.

## Reconstruction

The exact codec is `g4ep-rs16-leopard-high-8-4-v1`: Reed–Solomon over GF(2^16), polynomial `0x1002d`, high rate, eight systematic data shards and four recovery shards. Recovery indices start at 8. Shard bytes are configurable from 64 KiB through 4 MiB in multiples of 64; a full group describes eight shard lengths of original data. The default is 1 MiB per shard and 8 MiB per group. Source data is contiguous, with zero padding only at the end of the last group.

The `leopard-64-byte-planes` symbol layout places 32 low bytes then 32 high bytes per 64-byte unit. The reference implementation pins reed-solomon-simd 3.1.0 with HighRateEncoder. Its Cantor basis is `0001 acca 3c0e 163e c582 ed2e 914c 4012 6c98 10d8 6a72 b900 fdb8 fb34 ff38 991e`. The [codec vector](../../docs/g4ep/codec-vector-v1.json) and all 495 subsets of eight shards define cross-implementation compatibility; using an arbitrary Reed–Solomon library with the same shard counts is insufficient.

Groups have ordered contiguous indices, IDs `g` followed by the twelve-digit zero-padded group index, original byte offsets and original byte counts. Each group has exactly twelve ordered fragment entries indexed 0 through 11. Each fragment has its padded shard size, lowercase SHA-256 and `urn:sha256:` identity. Equal fragment hashes at different coordinates are permitted. Group offsets, counts, lengths and original total must agree exactly with the codec geometry. Empty files have zero groups.

A decoder verifies each supplied fragment hash, obtains any eight intact fragments in each group, reconstructs missing data with the specified codec, concatenates systematic data in group/index order, trims to original lengths and verifies the original size and SHA-256 before publishing the result. This step delivers the complete descriptor and verification APIs; the automatic distributed decoder/repair runtime remains a later step.

Limits: 64 MiB descriptor JSON, 32 MiB embedded original manifest, 1 TiB original, 83,333 groups and at most one million fragments. Implementations must enforce all limits, not only the JSON Schema.

## Discovery

`discovery` may be empty for offline use. At most 4,096 references are accepted. `private_peer` binds a lowercase 32-byte node identity to a canonical IP socket endpoint. Up to 32 explicitly allowed peers may use loopback, RFC1918, CGNAT 100.64/10, IPv6 unique-local or link-local addresses. DNS, public endpoints, zone identifiers, duplicate nodes/endpoints and zero ports are rejected.

`bittorrent_fragment` binds a known fragment identity to a lowercase BitTorrent v1 info hash and an exact xt-only magnet for that hash. Public trackers and extra magnet parameters are excluded from this private profile. Reading a reference neither connects to a peer nor proves availability. Fragment references are never exposed as original-file download links.

## Deterministic metadata and optional signatures

Canonicalization ID: `game4-g4e-v2-canonical-json-v1`. Serialize compact JSON with recursively UTF-16-code-unit-sorted object keys, unchanged array order, UTF-8 strings and no Unicode normalization. Allowed numeric fields use unsigned integer tokens, without exponent, decimal point or negative zero. Large sizes remain strings. This restricted profile follows RFC 8785 ordering/escaping, but does not accept arbitrary floating-point JSON. Duplicate keys, BOM, invalid Unicode and trailing JSON are rejected.

The authenticated payload is the entire root object with `authentication` removed. `payloadSha256` is SHA-256 of its canonical bytes. Thus it binds the original metadata, geometry, every fragment and discovery references. The writer emits canonical bytes for the complete descriptor; input whitespace and key ordering need not already be canonical.

Signatures are optional, up to eight, uniquely sorted by `keyId`. Algorithm `ed25519` signs the bytes `Game4.g4e.v2` followed by one NUL byte followed by the **32 raw payload-hash bytes**. This is normal Ed25519, not Ed25519ph. Public keys are lowercase 32-byte hex, signatures lowercase 64-byte hex, and key IDs are `urn:sha256:` plus SHA-256 of the raw public key. Noncanonical point encodings, weak keys and invalid signatures are rejected. Every included signature must validate.

A valid included signature proves possession of that signing key, not the identity of a publisher. Origin-sensitive callers must pin an expected fingerprint independently with `require_origin` or `inspect-manifest --trust-key`. Otherwise removal of all signatures creates an unsigned descriptor; applications must never silently treat that as authenticated. Native signing accepts an in-memory SigningKey; no private-key generation/storage or mandatory signature is introduced. Example signatures use the publicly known RFC 8032 test key, which must never authenticate production content.

## Commands and integration

```powershell
npm.cmd run g4ep -- export-manifest BUNDLE_DIR --name original.bin --output preservation.g4e
npm.cmd run g4ep -- inspect-manifest preservation.g4e
npm.cmd run g4ep -- inspect-manifest signed.g4e --trust-key urn:sha256:EXPECTED_PUBLIC_KEY_HASH
npm.cmd run g4ep -- verify-manifest-fragments preservation.g4e FRAGMENT_DIRECTORY
npm.cmd run g4e:validate -- preservation.g4e
npm.cmd run g4e:test
```

Export first checks all local fragment bytes and writes a new file without overwriting an existing descriptor. Optional `--discovery FILE.json` supplies an explicit reference array. Native and Node readers validate independently, sharing positive fixtures and an invalid conformance corpus. Desktop opening/import previews expose original metadata, group/fragment counts, metadata hash and signer fingerprints in English and Portuguese. They report neither original integrity nor publisher trust merely from reading the descriptor. Archive attachment and ordinary torrent downloads retain their v1-only boundary.
