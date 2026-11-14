# Save and restore the local library

Status: implemented in **Settings → Save and restore your library**, with English and Brazilian Portuguese UI. This is a complete private catalog backup, separate from the portable per-item [manifest v1](manifest-v1.md).

## What is saved

A backup includes all eight current catalog tables: content hashes/sizes, physical-copy paths and verification observations, collection metadata (including private notes), relative file mappings, task history/request deduplication, source-to-item mappings, portable manifest UUIDs, and content-bound identification observations. Local IDs, timestamps, and metadata remain intact. Game-file bytes, interface language/preferences, pending native-picker authorizations, VPN configuration, and application binaries are not included.

**Backups contain private local paths and notes.** The Settings screen states this before export. Keep the file in a trusted location; it is plain UTF-8 JSON, without encryption or signatures. The app does not upload it or start P2P sharing.

Choose **Save library backup** and an unused `.json` filename. Cancellation writes nothing. The snapshot reads committed catalog data inside a SQLite read transaction, including current WAL data; it never copies only the live main `.sqlite3` file. The connection mutex is released before serializing, validating, and writing the saved snapshot. New imports can start after the snapshot has been captured without changing that snapshot.

## Restoring

1. Wait for imports to finish, then choose **Choose backup to restore** in Settings.
2. The native picker selects a file. Rust reads a bounded document and validates its version, typed records, constraints, relationships, task vocabulary, and UUIDs using a separate in-memory database built from the application's embedded schema. Nothing is replaced during preview.
3. Review the filename and counts of items, unique contents, physical copies, and tasks. An empty backup is valid and displays zero counts.
4. Select the explicit replacement checkbox and choose **Replace library and restore**. This replaces the current catalog rather than merging it.

Before replacement, the app creates a new recovery backup of the current catalog in `database/backups/before-restore-<UUID>.game4-library.json` under the same app data directory as SQLite. On Windows this is normally `%LOCALAPPDATA%/io.github.pedrovitor-dev.game4/database/backups`. The result displays its filename. Select that recovery file through the same restore flow if you need to return to the previous catalog. Recovery copies are retained automatically; deletion/retention management is not implemented.

If a recovery copy cannot be written, restoration does not begin. Deletion of old rows, insertion of restored rows, and reset of observation states occur in **one SQLite transaction**. Failure rolls back the catalog transaction; an already-created recovery file can remain. Existing export/backup destinations are never overwritten. The app stages and syncs a temporary sibling and uses no-clobber publication. Filesystem/power-loss guarantees beyond SQLite FULL synchronization and file sync are not claimed.

Game files are not opened, copied, deleted, moved, or repaired by restore. All restored physical locations become `unverified`, with no `last_verified_at`, because a saved observation does not prove present-day disk health. Historical tasks remain; queued/running tasks become `interrupted` with incremented revisions and a sanitized error. No job is automatically resumed. Pending import-picker tokens from the previous catalog are invalidated. Navigate to Library to load the restored items.

## Concurrency and confirmation

Backend admission/worker locks and durable queued/running task checks prevent saving/restoring while imports are active. The UI stays on a working state while blocking workers perform file/database operations. Catalog reads may wait during the final restore transaction; no hashing or filesystem traversal is performed.

A successful preview creates one process-local, random UUID token and a SHA-256 fingerprint of the current catalog. Only one preview is retained; selecting another file replaces it. The preview expires after five minutes and is not persisted across app restart. Confirmation rechecks the catalog fingerprint under an immediate transaction; changed data, unknown/expired tokens, and repeated confirmation are rejected. Preparing a restore does not give the frontend any source path or catalog contents beyond counts/filename/token. Canceling the review discards the UI token; the cached backend preview expires or is replaced, and no restoration occurs automatically.

Only selecting a file is insufficient to start restoration. The UI requires a reviewed preview and the replacement checkbox. Callers must invoke the separate confirm command with the returned token. After an accepted confirmation attempt (including busy/stale/write failure), select the file again rather than automatically retrying a mutation.

## Backup format and compatibility

The current envelope is `format: "game4-library-backup"`, `backupVersion: 2`, `databaseSchemaVersion: 4`, a UTC `createdAt`, and `data`. The eight data arrays are `files`, `fileLocations`, `collectionItems`, `collectionItemFiles`, `tasks`, `importSources`, `collectionManifestIds`, and `fileIdentifications`. Row field names use camelCase matching the current embedded schema. The writer emits optional SQL fields as explicit nulls; omitted optional fields are read as null. All SQLite integer cells are canonical decimal **strings** (including IDs, sizes, revisions, and counts), preserving the full signed-64-bit range without JavaScript precision loss. Envelope versions are JSON integers. IDs must be positive and task revisions nonnegative with room for recovery increments.

The current full-catalog format is tied to schema v4. The original v1/schema v3 pair is also accepted, with absent identification rows treated as unavailable evidence rather than scanned or invented. Unsupported version/schema pairs are rejected without mutation. Future schema support needs an explicit format compatibility decision. Portable manifests remain independent and omit private physical locations; they cannot be selected as full-library backups.

Input/output is limited to 64 MiB and 500,000 total records across all arrays. Typed Rust deserialization rejects malformed JSON, invalid decimal strings, unknown typed fields, and duplicate typed fields. SQLite validation enforces STRICT types, uniqueness, foreign keys, hashes, sizes, statuses, and progress constraints. Task error codes/update timestamps and portable UUIDs also receive semantic checks. Restored values are always parameter-bound; SQL/schema/triggers from the selected file are never executed. Backup paths are catalog data, not instructions to access those files. Parsing inherits serde_json's recursion guard.

Backup/restore uses the current embedded schema. Identification introduced migration 0004; backup compatibility is checked independently. The app's current schema is v4. Backups saved by this version include durable catalog data only; in-memory dialog state is excluded.

## Verification

Rust integration tests cover a full save/restore with metadata, hashes, original paths, task history and UUID identity; reopening; empty catalogs; exact 64-bit strings; unsupported/malformed/inconsistent files; FK and uniqueness failures; stale/expired/replayed tokens; active work; no-clobber saves; recovery-file failure; injected SQL rollback; interruption recovery; and invalidation of old picker tokens. Original game fixture bytes remain unchanged. Tests use disposable synthetic catalogs.

Frontend contract tests validate no-argument picker commands, token-only confirmation, preview counts, filename-only responses, cancellation combinations, bounds, and both locales. Run `npm.cmd run check` and `npm.cmd run rust:test`. See [ADR 0010](decisions/0010-library-backup-restore.md).

Native Windows smoke passed for saving a synthetic catalog, Portuguese success feedback, loading the written file for preview, correct counts, and a disabled restore action before the checkbox is selected. The preview was closed without native replacement; actual replacement, rollback, and reopening are covered by Rust integration tests. Independent inspection verified the saved backup and its source hashes/sizes.

The original v1/schema v3 reader remains supported. Current writes/recovery copies use v2/schema v4, including `fileIdentifications` in row limits, fingerprints, validation, snapshots, and transactional replacement. Restored content evidence remains historical; location health always resets to unverified. See [identification compatibility](file-identification.md).

## Transfer recovery — November 14, 2026

Pause, reverified Resume and private transfer persistence are now implemented in a separate SQLite v1 store; catalog SQLite v4 and backup v2 remain unchanged. Interrupted work reopens Paused without networking. Owned downloads are repaired without duplication or truncation, while original seeding sources remain read-only. Inactive records can be forgotten without deleting content. See [transfer persistence](transfer-persistence.md) and [ADR 0020](decisions/0020-private-transfer-recovery.md) for storage, credentials, sidecars, limits and process-restart evidence.

## Storage folder preferences — November 14, 2026

Independent defaults for download, preservation-file and backup dialogs are implemented. They do not move existing sources, transfer payloads, internal databases or recovery files, and are excluded from catalog backup v2. Every save/download destination remains explicitly confirmed. See [storage folders](storage-folders.md) for controls, private settings and failure behavior.
