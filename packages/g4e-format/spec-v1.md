# Game4 Ever Manifest (.g4e), version 1

Status: initial public specification, November 13, 2026. The schema, Node/Rust validators, native create/read actions, offline CLI, examples and conformance checks are implemented. Desktop discovery and transfer-manager integration remain future work. See [implemented workflows](../../docs/ever-manifests.md). **MUST**, **MUST NOT**, **SHOULD** and **MAY** express normative requirements. This is a metadata format, not a new transport protocol.

## 1. Purpose and relationship to .g4

`.g4` (**Game4 Package**) is the optional, lossless ZIP/ZIP64 preservation package: original files, portable metadata, documentation, authorized artwork, `manifest.json` and its per-file SHA-256 checksums. Existing [.g4 v1](../g4-format/spec-v1.md) remains unchanged. It introduces no password encryption, mandatory DRM, account or central service. A separate checksum list is unnecessary because the manifest is the authoritative checksum list.

`.g4e` (**Game4 Ever Manifest**) is a separate, portable UTF-8 JSON descriptor for P2P discovery and sharing references. It carries metadata, exact payload hashes and BitTorrent identities/magnets, without embedding game bytes. It is neither an encrypted `.g4` nor an emulation/execution profile. The earlier conceptual `.g4t` name is superseded; there is no implemented legacy format to migrate and no automatic extension alias.

A conforming Game4∞ sharing workflow MUST support **original_files** directly: source files stay at their existing locations, are opened read-only and are mapped privately to torrent-relative paths. Creating a `.g4`, converting content, renaming originals, or duplicating the payload into a staging folder MUST NOT be prerequisites. If an engine cannot map the selected source layout safely, it must report the limitation rather than silently create copies or modify originals. Users MAY explicitly create and share a `.g4` as an alternative representation.

## 2. Envelope and versioning

The descriptor MUST match [the public JSON Schema](v1.schema.json), the embedded [manifest v1 schema](../manifest-schema/v1.schema.json) and all semantic rules below:

| Field            | Required value or meaning                                                    |
| ---------------- | ---------------------------------------------------------------------------- |
| `$schema`        | `urn:game4:ever-manifest:1`                                                  |
| `format`         | `game4-ever-manifest`                                                        |
| `schemaVersion`  | Integer `1`; independent from .g4, SQLite and IPC versions                   |
| `id`             | Canonical lowercase UUID v4 identifying this descriptor                      |
| `representation` | `original_files` or `g4_package`                                             |
| `manifest`       | Embedded Game4 preservation manifest v1 describing the **transferred bytes** |
| `torrents`       | 1–16 alternative complete-payload BitTorrent v1 references                   |
| `extensions`     | Namespaced advisory JSON metadata; empty object is valid                     |

All fields are required; core objects reject unknown properties. Required nullable fields must be present. Duplicate JSON keys at any depth, including equivalent escaped spellings, are invalid. JSON must be valid UTF-8 without a BOM, trailing values, unpaired Unicode surrogates or non-finite numeric values. Unsupported schema versions MUST fail explicitly; readers MUST NOT reinterpret them as v1. Schema URNs are identifiers, not URLs to fetch. Parsing and validation MUST work offline.

A descriptor ID is not a cryptographic content address or signature. Changing representation, payload identities or torrent references SHOULD produce a new descriptor ID; the embedded collection UUID can remain the same curated collection identity. Metadata corrections do not establish authorship or authenticity. No canonical signing representation is defined in v1. Unknown namespaced extensions MUST NOT be required for accessing core content, override hashes, trigger execution, impose DRM or activate network requests.

## 3. Representation and original preservation

### original_files

The embedded manifest describes each original file using its exact relative path, canonical decimal byte length and lowercase SHA-256 of raw bytes. It can include authorized covers, documentation and historical information as ordinary files. Its paths are portable sharing names, never absolute machine locations. Native source mappings are private local state and MUST NOT appear in the descriptor.

No original may be patched, normalized, recompressed, moved, renamed, deleted, or opened for writing by the sharing workflow. Source metadata remains local unless deliberately included as declared metadata. SHA-256 verification MUST precede seeding; changes during reads must stop or fail that source without repairing it in place. Incoming downloads and optional derived files use a separate user-selected destination. Resuming a managed download does not authorize reusing an original source as a writable destination.

### g4_package

The embedded manifest MUST describe exactly **one `.g4` file**. Its SHA-256 and size cover the complete ZIP file, not an inner original, a torrent dictionary or a concatenation of files. After downloading and verifying that outer file, a client MUST use the `.g4` reader to verify its embedded manifest and every original payload file before claiming package preservation integrity. The outer and inner collection UUIDs MUST agree; other declared metadata is advisory. Creating this representation is always an explicit optional action.

The two representations may refer to the same curated collection while using different transmitted bytes, hashes and swarms. Their hashes MUST NOT be substituted for one another.

## 4. Torrent reference

Each entry in `torrents` has exactly:

```json
{
  "infoHashV1": "<40 lowercase hexadecimal characters>",
  "magnet": "magnet:?xt=urn:btih:<the same infoHashV1>",
  "metainfoFile": null
}
```

`infoHashV1` is the SHA-1 digest of the **exact bencoded `info` dictionary bytes**, as defined by [BEP 3](https://www.bittorrent.org/beps/bep_0003.html). It is not a file SHA-256 or the hash of the whole `.torrent`. Torrent piece hashes and manifest file hashes serve different purposes. Every reference MUST bind the same complete payload file set and byte lengths; partial/subset torrents and duplicate info-hashes are invalid in this profile.

The initial profile supports BitTorrent v1 swarms only. BitTorrent v2-only/hybrid metadata, padding files and `btmh` topics are outside v1; clients must not silently downgrade or invent a v1 hash from a SHA-256 field. Their later adoption needs an explicit versioned profile and fixtures. This does not change the mandatory raw-file SHA-256 checks.

### Magnet profile

Use the [BEP 9 magnet form](https://www.bittorrent.org/beps/bep_0009.html), constrained as follows:

- Exactly one `xt`, equal to `urn:btih:` plus `infoHashV1`. V1 uses lowercase 40-digit hexadecimal; externally received base32 forms must be normalized before authoring this profile.
- Optional `dn` appears at most once; it is an advisory display name. Optional `tr` appears at most 32 times with distinct decoded values.
- Tracker hints use `https`, `http` or `udp`, a hostname and no credentials or fragments; UDP requires a nonzero explicit port. These hints do not authorize connections.
- Parameters use ASCII URI encoding, with valid percent-encoded UTF-8 values and no control characters. Unknown parameters, fragments, authority components and embedded acquisition URLs are rejected. `xs`, `as`, `ws` and `x.pe` are not part of this initial profile.

Trackers are optional. A trackerless descriptor is valid and can be used with [DHT](https://www.bittorrent.org/beps/bep_0005.html) or independently configured peer discovery. No particular tracker, bootstrap host, catalog, domain, account or Game4∞ server is mandatory. Discovery still needs reachable peers and a permitted network path; the descriptor does not guarantee availability, NAT traversal, anonymity or independent copies.

### Optional companion .torrent

`metainfoFile` can instead be `{ "path": "original-files.torrent", "sizeBytes": "271", "sha256": "<64 lowercase hex>" }` (illustrative values). The path is a safe, `.torrent`-suffixed path relative to the `.g4e` file's directory; it MUST NOT traverse parents, use a URL, collide with payload paths/prefixes, or collide with another companion path/prefix. Its SHA-256/size describe the **whole companion file**. A missing companion is allowed when the field is null; a referenced companion may be unavailable without invalidating the descriptor's syntax. A client can try magnet metadata exchange instead.

Offline descriptor validation MUST NOT open companions or URLs. A future acquisition client must request the user's action before reading a companion, deny redirected/unsafe filesystem paths, enforce the declared bounds/hash/length, then separately verify the exact torrent info-hash. Magnet-acquired metadata must also be bound to that info-hash. A syntactically valid reference does not establish that these external bytes exist or match.

## 5. Binding torrent metadata to the manifest

Before transferring or seeding, clients MUST verify all of the following:

1. The acquired info dictionary matches `infoHashV1`, with strict bencoding validation or exact original-byte hashing. Reject private-only swarms (`private=1`), unsupported metadata versions, unsafe paths and unexpected file types.
2. For multi-file torrents, join the UTF-8 `files[].path` components with `/`, excluding the advisory root `info.name`; compare the exact resulting path/length set with `manifest.files`. Order can differ for identity comparison but torrent piece verification follows metadata order. For single-file torrents, `info.name` must equal the sole manifest file's leaf path; a nested manifest path requires multi-file layout, even for one file. No automatic renaming or staging copy is required.
3. Apply manifest path safety plus byte/depth/case rules below to all acquired names. No undeclared entries, link entries, file/directory collisions or padding files may be ignored to force a match.
4. After transport piece validation, hash each complete raw file with SHA-256 and compare its exact length. Only then report verified content; the info-hash alone does not prove these per-file claims.
5. In packaged mode, additionally validate the `.g4` as described above. Do not extract or execute it automatically.

Alternate torrents can use different piece lengths/order and therefore different info-hashes while describing the same original file set. None is trusted merely because it appears in a `.g4e`. A failed metadata/payload binding MUST stop the reference and surface a stable error, without mutating originals or silently changing reference hashes.

## 6. Bounds and safe handling

| Resource                                                          | V1 limit                                                        |
| ----------------------------------------------------------------- | --------------------------------------------------------------- |
| `.g4e` input                                                      | 64 MiB UTF-8 bytes                                              |
| Embedded manifest's JSON value, including its internal whitespace | 32 MiB UTF-8 bytes                                              |
| JSON nesting                                                      | 64 levels; root value is level 0                                |
| Files                                                             | 1–100,000                                                       |
| File and total payload lengths                                    | Canonical unsigned decimal strings; each and sum fit u64        |
| Path                                                              | Existing manifest path rules; 4,096 UTF-8 bytes; 128 components |
| Alternate torrent identities                                      | 1–16                                                            |
| Magnet URI                                                        | 16,384 characters; 32 trackers                                  |
| Companion metainfo file                                           | 8 MiB maximum, positive canonical byte length                   |
| Packaged payload                                                  | The physical .g4 v1 archive limit (1 TiB + 512 MiB)             |

All payload paths must be distinct after Unicode lowercase conversion; no such file path can prefix another through `/`. This catches common case/file-directory collisions, without promising all filesystem normalization behavior. Unsupported names must be reported without modifying originals. Filesystem-specific destination checks are still required by future download/extraction implementations.

Limit input and depth before general parsing. Preserve exact sizes with arbitrary-precision/integer arithmetic, not JavaScript Number. A client can enforce stricter local disk/network limits and must explain the failure rather than truncate or silently skip entries. Every network operation requires a separate deliberate sharing/download action; parsing, catalog browsing and example validation must make no connections. Unknown metadata must never become executable commands or shell arguments.

## 7. Reference validation and public examples

The [offline Node validator](validation.mjs) implements schema/JSON/path/identity consistency and resource rules. It reads only the supplied descriptor and bundled schemas. Success means **descriptor validity**, not verified companion files, verified torrent metadata, a reachable swarm, verified payloads or production P2P readiness. Native create/read commands are implemented separately; opening a descriptor never starts a transfer session. Explicit Rust CLI verification can bind a selected torrent and local originals; see [the implementation guide](../../docs/ever-manifests.md).

```powershell
npm.cmd run g4e:validate -- packages/g4e-format/examples/original-files.g4e
npm.cmd run g4e:validate -- packages/g4e-format/examples/package.g4e
npm.cmd run g4e:test
```

The [original-file example](examples/original-files.g4e) references three synthetic originals totaling 40 bytes. The [package example](examples/package.g4e) references the existing public `.g4` fixture's whole-file hash. Companion torrents contain trackerless BitTorrent v1 metadata generated independently in Python; tests check their exact info-hashes, full-file SHA-256 and piece digests. They are metadata fixtures, not advertised live swarms. [The shared invalid corpus](invalid-cases.json) distinguishes structural schema violations from semantic failures. Fixtures use synthetic CC0 bytes only.

The format is open and implementation-independent. Encryption, DRM, mandatory central services, automatic seeding, automatic execution, game bytes embedded in JSON, emulation profiles and financial mechanisms are not requirements of `.g4e` v1.
