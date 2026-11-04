# SQLite storage

Status: schema v2, migrations, local import/hashing, task progress/recovery, and library listing are implemented. Editorial editing, cancellation, standalone integrity rechecks, and backups remain future work.

## Location and lifecycle

Tauri resolves `app_local_data_dir()` and opens `database/game4.sqlite3` during startup. On Windows the default location is `%LOCALAPPDATA%/io.github.pedrovitor-dev.game4/database/game4.sqlite3`. It is independent of the repository, current directory, and application installation folder. Parent directories are created as needed. No game files are copied or moved by startup.

The Rust core uses rusqlite with bundled SQLite so development does not require a separate SQLite installation. One connection is protected by a mutex and managed as Tauri state. Database IPC work runs on a blocking worker, not the UI thread. Foreign keys are enabled, the busy timeout is five seconds, journal mode is WAL, and synchronous mode is FULL.

Database startup failure stops startup rather than resetting or deleting user data. Errors are currently handled by the native startup failure path; a dedicated recovery screen is not implemented.

## Schema version 1

| Table                   | Purpose and invariants                                                                                                         |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `files`                 | Content identity: lowercase SHA-256 and nonnegative byte size, unique as a pair                                                |
| `file_locations`        | Physical copies: original display path, unique caller-normalized path key, verification state, and last verification timestamp |
| `collection_items`      | Editorial identity: title, platform, edition, language, declared source/license, and notes                                     |
| `collection_item_files` | Maps content to an item's relative paths; deleting an item removes links, not content                                          |
| `tasks`                 | Import/verification task state, progress counts, error code, and timestamps                                                    |

Tables use SQLite STRICT typing and constraints. Foreign keys prevent deletion of referenced content. Indexes cover file references and task status. UTC timestamps use ISO-like text with a trailing `Z`; identifiers and schema keys do not depend on the UI locale.

Verification states are `unverified`, `verified`, `changed`, `missing`, and `inaccessible`. Health belongs to a location because separate copies of the same content may be damaged independently.

Task states are `queued`, `running`, `succeeded`, `failed`, `cancelled`, and `interrupted`. Imports transition queued → running → succeeded/failed; startup marks unfinished work interrupted. Cancellation and independent verification workers remain planned. Applications must update `updated_at` explicitly when editing collection items.

Import path keys use exact Rust-canonicalized UTF-8 spelling. Links/reparse points are rejected; relative separators are `/`. Case aliases and hard links can retain separate locations. See [import path policy](library-import.md).

## Migration rules

SQL lives under `apps/desktop/src-tauri/src/database/migrations/`. An ordered embedded migration list maps to `PRAGMA user_version`. Startup obtains an immediate write transaction, reads the version under that lock, applies pending steps, updates the version, and commits. A failed migration rolls back schema changes and version updates together.

Reopening the same schema is idempotent. A schema newer than the application supports is rejected without downgrading or resetting it. Negative schema versions are rejected before indexing migrations, preserving the database unchanged. Do not edit an already released migration; append a new numbered migration and register it in order. Existing databases are never deleted automatically.

Future schema changes must include migration and data-preservation tests. Do not insert transaction-control statements into migration files; the runner owns the transaction.

## IPC boundary

`database_info` returns `schemaVersion`, `fileCount`, `locationCount`, `itemCount`, and `taskCount`. It does not accept SQL or paths and does not expose a user-data directory. Errors use the common `IpcError` contract, including stable codes `DATABASE_UNAVAILABLE` and `DATABASE_TASK_FAILED`, `retryable: false`, and `field: null`. Internal counts are range-checked before crossing JSON. See [IPC contracts](ipc-contracts.md). The frontend adapter is `src/services/desktop.ts`; the current interface does not display these counts yet.

## Verification and backups

Run `npm.cmd run rust:test` on Windows PowerShell. Tests use temporary databases and cover initialization, reopening with preserved records, duplicate content, separate copy health, foreign keys, invalid states/progress, relationship deletion, future-version refusal, and migration rollback.

Personal databases and WAL sidecars are ignored by Git. Never commit them as fixtures. Do not copy only the main database while the app is writing in WAL mode. A live-backup/export feature is future work; for manual offline copies, close all application instances first and preserve the database directory, including any remaining sidecars. No backup is created automatically by this initial setup.

## Schema version 2

Migration 0002 adds nullable unique request IDs and selection-token fingerprints to tasks, persisted monotonic revisions and update timestamps, plus `import_sources` mapping canonical source roots to stable collection items. Existing v1 records remain intact. On startup queued/running tasks become interrupted, with a revision increase and sanitized error. Import work is serialized; task progress is committed separately, while successful catalog records and task completion share one transaction. See [import behavior](library-import.md).
