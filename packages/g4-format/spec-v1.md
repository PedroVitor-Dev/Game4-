# Game4 Package (.g4), version 1

Status: initial public specification, November 13, 2026. Version 1 is implemented by Game4∞. Normative terms **MUST**, **MUST NOT**, **SHOULD** and **MAY** describe conformance to this profile. Publication in this repository does not imply third-party adoption. Changes incompatible with these rules require a new package version.

## 1. Purpose and identification

A `.g4` is a portable, self-contained collection of original file bytes and a preservation manifest. It requires no account, server, blockchain, torrent client or Game4∞ database to read. It verifies consistency with its manifest; neither SHA-256 nor ZIP CRC establishes authenticity, ownership or permission to redistribute.

The container MUST be a single-volume ZIP conforming to [PKWARE APPNOTE](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT), using only **Stored (method 0)** entries. ZIP64 is supported and MUST be used when required by ZIP field limits. A `.g4` filename is recommended; the Game4∞ writer requires that extension. Readers MUST identify the package using ZIP structure and its descriptor, rather than the extension alone. There is no custom binary magic or private codec. Generic tools can list/read it as ZIP; some tools may require renaming a copy to `.zip`.

The profile prohibits encryption, compression, split volumes, self-extracting prefixes, directory/link/device entries, duplicate ZIP names, inconsistent local/central headers, overlapping entries and unlisted local headers. The first local ZIP header MUST begin at offset zero. EOCD MUST end at the physical file end and its ZIP archive comment MUST be empty, ensuring an unambiguous terminal record for bounded indexing. Entry comments, timestamps and permission bits are non-authoritative and MUST NOT trigger execution or permission restoration.

## 2. Layout

```text
example.g4
├── game4.json                 Package descriptor, version 1
├── manifest.json              Existing Game4 manifest v1
└── data/
    ├── game/original.bin      Manifest path: game/original.bin
    ├── artwork/cover.png      Optional, if explicitly in manifest
    └── docs/manual.txt        Optional, if explicitly in manifest
```

The ZIP MUST contain exactly `game4.json`, `manifest.json` and one file entry named `data/` followed by each manifest path, with exact case and Unicode spelling. No explicit directory entries or undeclared files are allowed. A manifest file called `manifest.json` is stored as `data/manifest.json`, without conflicting with package metadata. Empty files are valid; empty collections and directory-only entries are not.

All ZIP entry names MUST be valid UTF-8 byte sequences. Writers MUST set the ZIP UTF-8 language flag for non-ASCII names. Readers MUST NOT reinterpret raw names as native absolute paths or follow them on disk. Manifest paths inherit all [manifest v1](../manifest-schema/README.md) safety rules and additionally MUST be at most **1,024 UTF-8 bytes** and **128 components**. Distinct paths MUST remain distinct after Unicode lowercase conversion, and no lowercased file path may be a directory prefix of another. This catches case and file/directory conflicts; it is not a universal filesystem normalization guarantee. Any future extractor must additionally reject destination-specific name collisions.

Entry order MAY vary. Game4∞ writes descriptor, manifest, then payload paths sorted lexicographically, with fixed DOS timestamp 1980-01-01, Unix regular-file mode 0644 and no comments. Repeat creation with the same manifest and source bytes is deterministic for the same writer version. Cross-implementation byte-for-byte canonical ZIP serialization is not required and must not be used as an implicit signing contract.

## 3. Package descriptor

`game4.json` MUST be a UTF-8 JSON object matching [the public descriptor schema](v1.schema.json):

```json
{
  "$schema": "urn:game4:package:1",
  "format": "game4-package",
  "packageVersion": 1,
  "manifest": "manifest.json",
  "payloadPrefix": "data/"
}
```

All five fields are required. Unknown or duplicate fields are invalid. UTF-8 BOMs are not accepted by the reference parser. `packageVersion` is independent from manifest schema version, IPC contract version and SQLite schema version. Readers MUST reject unsupported package versions explicitly and MUST NOT silently reinterpret them as version 1. The schema URN is an identifier, not a network fetch location. Parsing MUST work offline.

## 4. Preservation manifest and payload

`manifest.json` MUST satisfy the existing [Game4 manifest v1 specification](../../docs/manifest-v1.md), including its schema, semantic validation, portable collection UUID, relative paths, nullable declared metadata, lowercase SHA-256 and canonical unsigned decimal-string sizes. Manifest namespaced extensions remain advisory. They cannot authorize additional ZIP entries or override core hashes, sizes or package rules.

The payload bytes MUST match their declared byte lengths and SHA-256 values. CRC32 MUST also be valid as required by ZIP. Readers claiming a **verified** result MUST read every payload entry to completion, check length, CRC and SHA-256, and validate the complete container and manifest. Merely opening the central directory or trusting stored hashes is insufficient. A failed check MUST NOT be represented as success. An inspection result is an observation of those package bytes at read time, not a permanent guarantee or a signed provenance statement.

No separate `checksums.sha256` file is used in version 1; duplicating manifest hashes would introduce two competing sources of truth. Authorized artwork, manuals, additional metadata or instructions can be included as ordinary declared payload files, without special execution semantics.

## 5. Mandatory limits

These profile limits apply to conforming v1 writers/readers, not just the initial UI:

| Resource                      | Maximum                           |
| ----------------------------- | --------------------------------- |
| Declared payload files        | 100,000 (at least 1)              |
| ZIP entries                   | 100,002                           |
| Payload bytes, summed exactly | 1 TiB = 1,099,511,627,776         |
| Physical archive bytes        | Payload limit + 512 MiB           |
| Package descriptor bytes      | 16 KiB                            |
| Manifest bytes                | 32 MiB                            |
| Central directory bytes       | 256 MiB                           |
| Manifest path                 | 1,024 UTF-8 bytes; 128 components |

Readers MUST bound central-directory count/size **before** allocating their ZIP index and MUST bound JSON reads before parsing. ZIP32 EOCD and optional ZIP64 values MUST be consistent. ZIP64 EOCD uses the fixed 44-byte record body; extensible data sectors are excluded. Only one volume is permitted; central directory ends immediately before ZIP64 end records, or before ZIP32 EOCD when ZIP64 is absent. Integer addition MUST be checked for overflow. Unsupported limits MUST produce a failure, not truncation or partial verification.

Game4∞ additionally bounds candidate local source locations to 500,000 when exporting a catalog item. This is a writer/catalog resource limit and does not add entries to the package.

## 6. Safe creation and reading

Writers MUST validate the manifest/profile and compare actual input bytes with its stored hashes and lengths. They MUST reject unavailable or mismatched sources rather than silently updating reference hashes. Game4∞ can try other registered copies of the same content. Its sources are regular read-only files; links and Windows reparse points, including ancestor reparse points, are rejected. The selected copy is checked again while streaming into ZIP, preventing changes between selection and copying from creating a false verified package.

Game4∞ writes a temporary sibling, finalizes ZIP, syncs it and publishes without replacing an existing destination. Normal failures remove the temporary file and publish no `.g4`; an abrupt OS/process failure may leave a temporary sibling, but not an intentionally published partial package. Source files are never moved, modified, executed or shared.

Reading is inspection and full validation, **not extraction or automatic catalog import**. The initial reader never writes payload files. A future extractor must use safe destination handling, enforce collisions/permissions and avoid following links; a general ZIP extractor's behavior is outside this reference implementation. Reading instructions or executable payloads MUST NOT launch them.

Unknown package versions, invalid structure, limit violations, content mismatches and output failures are distinct failures. The app exposes stable localized error codes rather than absolute paths or raw OS errors. Operations run in blocking workers while the UI remains responsive; this increment has a busy indicator, without byte-level package progress or cancellation. Export waits for an idle library and excludes concurrent catalog mutation/restore during its source snapshot and copy.

## 7. Reference implementation and interoperability

- Rust reader/writer: `apps/desktop/src-tauri/src/packages/`.
- Native commands: `library_export_package({itemId})`, `package_open()`.
- Offline CLI: `npm.cmd run g4 -- create --manifest MANIFEST.json --root DIRECTORY --output PACKAGE.g4`; `npm.cmd run g4 -- read PACKAGE.g4`.
- Public schema checks: `npm.cmd run g4:test`.
- Binary synthetic fixtures: [ZIP32](fixtures/v1-minimal.g4), [ZIP64](fixtures/v1-zip64.g4). Their descriptor and manifest are provided separately. Both are generated with Python's standard `zipfile`, independently of Rust's ZIP writer; the ZIP64 fixture includes forced small-file ZIP64 headers and end records.

Fixtures contain synthetic text and empty bytes only, marked CC0-1.0. They do not contain game assets. Their 40-byte payload covers Unicode paths and empty files. See [fixture generator](generate-fixtures.py). ZIP64 fixture checks exercise record compatibility without allocating a 4 GiB file; very large physical archives have not been tested in this increment.

Payload extraction, catalog reconstruction, signatures, compression, password protection, multipart packages and `.g4t`/`.g4lib` are outside version 1. New incompatible capabilities need a versioned specification before implementation.

## 8. Optional packaging and discovery sidecars

Package creation is optional. Game4∞ sharing must also support original files directly, without conversion, renaming or mandatory payload staging copies. [Game4 Ever Manifest (.g4e)](../g4e-format/spec-v1.md) is a separate JSON discovery/P2P descriptor that can refer to originals or to an explicitly created .g4. It is not encryption or an execution profile. The .g4 v1 container, manifest and authoritative per-file SHA-256 checksums remain unchanged; no mandatory password, DRM or central service is introduced.
