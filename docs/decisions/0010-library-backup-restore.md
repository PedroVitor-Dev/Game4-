# ADR 0010: Validated local catalog backup and transactional restore

Date: November 4, 2026

Status: Accepted

## Context

Per-item manifests intentionally omit physical locations, private notes, and history. Users also need to save and recover their complete local catalog without mishandling SQLite WAL files or treating a saved path as permission to alter game data.

## Decision

Use a versioned private JSON snapshot of all current catalog tables. Read the live database under a SQLite transaction; preserve exact integer cells as decimal strings. Validate restored typed rows in an in-memory database created from trusted embedded migrations and parameterized inserts. Reject incompatible versions and bounded-input/record violations.

Restore is a two-step native picker/preview plus explicit UI replacement confirmation. Bind the process-local, five-minute token to a current-catalog fingerprint. Block active imports, save a no-clobber recovery copy, and replace all catalog rows in one transaction. Invalidate picker authorizations, interrupt unfinished historical jobs, and reset copy verification to unverified. Never touch referenced game bytes.

## Alternatives and consequences

Copying a live SQLite main file would miss WAL data. Opening an arbitrary backup SQLite schema could introduce untrusted triggers or schema behavior. A manifest-only export would lose local metadata/history. Blind replacement would discard the current catalog without a recovery path. These alternatives are rejected.

The backup is private and unencrypted, tied to database schema v3, limited to 64 MiB/500,000 records, and replaces rather than merges. The final transaction can briefly delay catalog queries. Automatic backup scheduling, retention, encryption, and standalone reconstruction from manifests remain future work. Recovery copies require disk space and are retained after failures occurring after their creation.

## Verification

Integration tests verify round trips, identity/history preservation, constraint rejection, expiry/concurrency, recovery-copy failure, transactional rollback, copy-health reset, and untouched source bytes. See [library backup](../library-backup.md).
