# Preservation JSON manifest v1

Status: implemented initial format. This is manifest schema version **1**, independent of IPC v1, SQLite schema v3, and application version. The format is intended for portable catalog snapshots; archives, signatures, manifests imported back into the catalog, and file restoration remain future work.

## Export from Library

Choose **Export JSON manifest** on a library item and select a new `.json` filename in the native save dialog. Canceling the dialog creates no file or manifest identity. The export runs on a blocking worker. Success displays only the saved filename; failures use localized errors without exposing absolute paths.

The exporter reads a consistent catalog snapshot, including the stored SHA-256 identities. It does **not** reread game files or perform a fresh integrity check. A missing/changed source can therefore still have an exportable historical fingerprint. Imports and exports do not start sharing or execute content. Referenced files are not included in the JSON.

The exporter does not query or serialize physical location/source-root fields, path keys, SQLite row/task IDs, request tokens, or internal verification state. Titles and declared metadata are user-provided content and are preserved, so review them before publishing. The default filename contains the local item number for convenience; that number is not used as the portable collection identity.

## Required structure

See the complete [synthetic example](../packages/manifest-schema/examples/v1.example.json) and [public schema](../packages/manifest-schema/v1.schema.json).

| Field           | Meaning                                                                           |
| --------------- | --------------------------------------------------------------------------------- |
| `$schema`       | Exact identifier `urn:game4:manifest:1`; resolve it to the bundled schema locally |
| `format`        | Exact discriminator `game4-manifest`                                              |
| `schemaVersion` | Integer `1`; reject unsupported versions rather than guessing                     |
| `collection`    | Portable ID, original title, and explicitly declared metadata                     |
| `files`         | Nonempty array of relative path, exact size, and SHA-256 records                  |
| `extensions`    | Object for optional namespaced, nonessential metadata; `{}` is valid              |

`collection.id` is a lowercase RFC UUID v4, independent of local database identifiers. Schema v3 adds `collection_manifest_ids`; the ID is created when the first valid catalog snapshot is built and survives export, restart, metadata changes, and reimport of the same source. Deleting/recreating an item gives it a new identity. IDs do not prove authorship or authenticity.

Collection fields `platform`, `edition`, `language`, `declaredSource`, and `declaredLicense` are required but explicitly nullable. The title must contain nonwhitespace text, no control characters, and at most 512 Unicode characters. Platform/edition are limited to 200 characters, language to 100, and source/license to 4096. Non-null metadata must be nonblank and contain no control characters. The catalog's free-form private notes are omitted.

Each file contains exactly `path`, `sizeBytes`, and `sha256`. Hashes are exactly 64 lowercase hexadecimal characters, using SHA-256 of the original bytes. Empty files are valid. Sizes are canonical decimal **strings**, from `0` through `18446744073709551615`, with no sign or leading zeroes. Consumers must not convert them through JavaScript Number. Numeric JSON sizes are rejected. Current SQLite imports remain bounded by signed 64-bit sizes; the portable format can describe the full u64 range.

## Relative paths and Unicode

Paths use `/`, preserve exact Unicode spelling/case, and are limited to 4096 Unicode characters. They are not URLs. Absolute, drive-prefixed, UNC, backslash, empty-component, `.`/`..`, trailing-slash, trailing-dot/space, control-character, and Windows-reserved-character paths are invalid. Windows device basenames (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, including superscript 1/2/3 variants) are rejected case-insensitively, including with extensions.

Each path is unique within a manifest. A file cannot also be an ancestor of another path (`data` and `data/file.bin`). Files with identical bytes may appear at distinct paths. No Unicode normalization or case folding is applied: case-only or normalization-equivalent names remain distinct. Consumers restoring onto less expressive filesystems must detect collisions and reject or require an explicit rename policy. Schema validation alone does not authorize extraction or guarantee safe writes through filesystem links; no extractor is implemented here.

## Validation and limits

Documents use UTF-8 JSON. The Rust parser rejects malformed JSON, unsupported versions, unknown typed fields, missing required nullable fields, duplicate typed fields, invalid hashes/sizes/paths, duplicate or ancestor-conflicting paths, more than 100,000 files, and serialized input/output exceeding 32 MiB. Parsing reads no referenced files, loads no remote schemas, and performs no database mutation. JSON recursion is subject to serde_json's recursion guard. Duplicate keys inside arbitrary extension objects follow serde_json's object semantics and are not a signature mechanism.

The public JSON Schema uses [Draft 2020-12](https://json-schema.org/draft/2020-12) and validates structure/local syntax offline. Cross-record path rules, u64 range within a 20-digit string, duplicate JSON keys, and encoded-document byte limits require semantic validation as well. The shared rejection corpus marks cases that intentionally pass structural schema validation but fail semantic validation.

Extensions use reverse-domain-style ASCII keys such as `org.example.metadata` (lowercase letters, digits, hyphens, dot-separated nonempty components starting with a letter; max 200 characters). Values may be JSON data. Extensions cannot override integrity fields, authorize actions, or be required for understanding core v1. Unknown extensions may be retained/ignored. Unknown core fields are rejected; introducing new required/core semantics requires another version and a separate schema artifact. Existing version schemas should remain available when future versions are added.

## Serialization and file safety

The writer sorts files by exact Rust string order (UTF-8 lexicographic), uses the Rust field declaration order, pretty-prints with two spaces, and ends with LF. No current time, machine name, absolute path, or changing tool version is inserted. Repeated exports of an unchanged item produce identical bytes. This is a deterministic application serialization, **not** RFC 8785/JCS canonicalization and not a signing format. Cryptographic signatures and timestamp proofs require a separate specification.

A complete document is validated/serialized before writing. The exporter writes and syncs a temporary sibling file, then publishes it with no-clobber semantics. Existing files are never overwritten, including when another process creates the destination after the picker closes. Pick an unused `.json` name; disk/permission failures report `MANIFEST_WRITE_FAILED`. Temporary files are removed on ordinary failure through tempfile cleanup; a process crash can leave a temporary sibling. No full power-loss/directory-fsync guarantee is claimed.

## Verification and next steps

Run `npm.cmd run manifests:test`, `npm.cmd run check`, and `npm.cmd run rust:test`. Tests share the public example and malformed corpus, verify Unicode and exact large sizes, round-trip serialization, stable identity/reimport/restart, stored known SHA-256, absence of private fields, atomic no-clobber export, invalid requests, and cancellation response validation.

This increment provides schema, serializer/parser, library export, and validation fixtures. Reconstructing a catalog from a manifest, comparing files on disk, signatures, packaging, and the full offline survival test remain open. See [preservation](preservation.md) and [ADR 0009](decisions/0009-versioned-manifests.md).

For offline semantic validation of a saved file, run:

```powershell
npm.cmd run manifests:validate -- "C:\path\collection.manifest.v1.json"
```

The development CLI reads at most 32 MiB plus one sentinel byte, returns a JSON validation summary, and exits nonzero for invalid documents. It neither verifies referenced game bytes nor changes the catalog. It requires the repository's Rust build tools; a standalone validator binary is not distributed yet.

Native Windows smoke passed for a two-file Library export through the save dialog with Portuguese success feedback. The actual written JSON passed both offline schema and Rust semantic validation; an independent SHA-256 calculation matched each referenced synthetic source file.
