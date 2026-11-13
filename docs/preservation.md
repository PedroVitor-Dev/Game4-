# Preservation and formats

## Principles

Preserve original bytes and record transformations as separate copies. A hash detects differences; it does not establish historical authenticity, authorship, or redistribution rights. Observed availability does not prove independent copies exist.

## Initial manifest

Manifest v1 is implemented with a public JSON Schema, examples, Rust serializer/parser, semantic validation, and a per-item Library export action. It records a portable collection UUID, declared metadata, exact relative paths, decimal-string byte sizes, and stored SHA-256 hashes. Machine-specific locations are omitted. Exporting is a catalog snapshot, not a fresh integrity check or file package.

See [manifest v1](manifest-v1.md) for version compatibility, validation limits, path rules, deterministic serialization, and no-clobber export. Standalone rechecks are implemented. Manifest import/reconstruction, signatures, canonical signing serialization, and the full offline survival test remain planned.

## Proposed formats

| Format   | Purpose                                        | Status                                                         |
| -------- | ---------------------------------------------- | -------------------------------------------------------------- |
| `.g4`    | Portable package containing manifest and files | Implemented v1: stored ZIP/ZIP64, manifest and checked payload |
| `.g4t`   | Sharing references and metadata                | Conceptual; justify benefits over `.torrent` + JSON            |
| `.g4lib` | Catalog and reference export                   | Future; does not imply game files are included                 |

Do not treat extensions as stable formats before specifying security, compatibility, and migrations. Avoid mandatory recompression that impedes random access, resume, and torrent interoperability.

## Integrity and recovery

Verify during import, after transfers, and on demand. Future scheduling must respect disk load and power usage. Display the last verification time and distinguish inaccessible files from corruption. Repairs use verified copies or available redundancy; never silently overwrite the only original.

## Usable preservation

Record authorized manuals, dependencies, tool versions, settings, and execution instructions. Include auxiliary files only when redistributable. Never execute content automatically.

## Survival test

Export a test collection with manifest, schema, instructions, and hashes. Reconstruct the catalog and verify files in a separate offline environment without Game4∞ servers. Repeat after relevant format changes. Include signatures and timestamp proofs in exports when available.

## Implemented .g4 package v1

The [public specification](../packages/g4-format/spec-v1.md) defines stored ZIP/ZIP64 with a descriptor, manifest v1 and original payload. Desktop Library exports items and reads packages with full hash/size checks; the offline CLI can create and read without a catalog or server. Reading does not extract, execute or import content.
