# ADR 0009: Portable versioned JSON manifests

Date: November 4, 2026

Status: Accepted initial format

## Context

The local catalog stores SHA-256 identities, relative item paths, and physical copies. Preservation needs an independently documented snapshot that omits machine-specific locations and can evolve without conflating IPC, database, or application versions.

## Decision

Implement manifest v1 with a public offline JSON Schema 2020-12, typed Rust serializer/parser, semantic validation, fixtures, and a Library export action. Assign portable UUID-v4 identities through a schema-v3 mapping table, independently of SQLite row IDs. Preserve exact Unicode relative paths and string-valued u64 byte sizes. Use explicit nullable declared metadata and namespaced optional extensions; reject unknown core fields and unsupported versions.

Export stored catalog hashes without claiming a new disk verification. Use deterministic sorted serialization, but defer signature canonicalization. Write new files through a synced temporary sibling and refuse overwrites. Native save dialogs own destinations; the frontend sends only an item ID.

## Alternatives and consequences

Exporting SQLite IDs or absolute paths would bind the document to one installation. Numeric JSON byte counts could lose precision. Treating JSON Schema as sufficient would miss duplicate paths, ancestor conflicts, u64 overflow, and encoded-size limits. Silent version upgrades would weaken compatibility. These approaches are rejected.

The strict portable-path policy can refuse a collection containing names that the local catalog accepts. Export does not bundle bytes or reconstruct catalogs. Rechecks, import/restore, migrations between manifest versions, signatures, and archives require separate work. Public schema publication waits for the owner's push.

## Verification

Shared fixtures exercise public structural and Rust semantic validation. Rust tests exercise actual catalog snapshots, identity persistence, output stability, and no-clobber file writes. See [manifest v1](../manifest-v1.md).
