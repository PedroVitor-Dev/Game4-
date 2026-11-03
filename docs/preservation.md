# Preservation and formats

## Principles

Preserve original bytes and record transformations as separate copies. A hash detects differences; it does not establish historical authenticity, authorship, or redistribution rights. Observed availability does not prove independent copies exist.

## Initial manifest

Design a versioned JSON manifest first, with a public schema and examples. Candidate fields: schema version, collection identifier, relative paths, size, SHA-256, platform, edition, language, declared provenance, declared license, and execution instructions.

Label declared metadata accordingly. Separate required fields from extensions. Explicitly reject incompatible versions. Define path normalization, Unicode handling, and canonical serialization before signing documents. Do not export personal absolute paths by default. Schema keys remain in English and do not change with the interface language; preserve original titles, paths, and user metadata.

## Proposed formats

| Format | Purpose | Status |
|---|---|---|
| `.g4` | Portable package containing manifest and files | Conceptual; container and limits undecided |
| `.g4t` | Sharing references and metadata | Conceptual; justify benefits over `.torrent` + JSON |
| `.g4lib` | Catalog and reference export | Future; does not imply game files are included |

Do not treat extensions as stable formats before specifying security, compatibility, and migrations. Avoid mandatory recompression that impedes random access, resume, and torrent interoperability.

## Integrity and recovery

Verify during import, after transfers, and on demand. Future scheduling must respect disk load and power usage. Display the last verification time and distinguish inaccessible files from corruption. Repairs use verified copies or available redundancy; never silently overwrite the only original.

## Usable preservation

Record authorized manuals, dependencies, tool versions, settings, and execution instructions. Include auxiliary files only when redistributable. Never execute content automatically.

## Survival test

Export a test collection with manifest, schema, instructions, and hashes. Reconstruct the catalog and verify files in a separate offline environment without Game4∞ servers. Repeat after relevant format changes. Include signatures and timestamp proofs in exports when available.
