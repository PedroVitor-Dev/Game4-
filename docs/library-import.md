# Local library import

Status: implemented on Windows. Imports index local files; they do not copy, move, execute, upload, or share them.

## Using the application

1. Open Library and choose **Add files** or **Add folders**. Both native pickers support multiple selections.
2. Review the displayed source names, then choose **Import selection**. Dismissing the picker does not create a task.
3. Follow file-count progress in Library or Tasks. After completion, Library displays each source's title, number of relative file entries, and exact byte total.

Each selected file becomes one item. Each selected folder becomes one item containing files recursively, with paths relative to that folder. Empty files are valid; an empty folder fails the selection. Selecting a folder and one of its children creates distinct items intentionally. Duplicate identical roots within one selection are collapsed.

SHA-256 is streamed on Tauri’s blocking executor with one reusable 1 MiB buffer per import, independent of file size and reused between files. Progress writes are throttled to at most once per 200 ms during hashing; startup, enumeration, failure, and final completion still persist explicitly. This avoids a FULL-sync SQLite write for every tiny file. Final counts remain exact, and failed imports record the last processed count. Content identity is the pair `(sha256, size_bytes)`. Identical bytes share one content row, while each physical path retains its own location. Byte totals count relative entries, including copies with identical content. This is an initial local fingerprint, not proof of authenticity, provenance, or future file health. A separate user-triggered integrity recheck remains planned.

## Atomicity, reimport, and failures

All sources in one selection commit together. Traversal and hashing occur before opening the short catalog-write transaction. Any source failure prevents the entire selection from appearing as a partial successful catalog entry. Task progress and failure remain visible independently. Content rows, item mappings, locations, and the task's success state commit in one transaction.

Reimporting the same canonical root refreshes its existing item and relative mappings instead of creating another item. Editorial metadata is retained. Earlier content identities remain in the database, and other items' mappings are not rewritten when a physical file changes. Locations record the latest content observed at their path; there is no automatic watcher or garbage collection. Imported files can still change after they have been read. `last_verified_at` records the observation, not a permanent guarantee.

Import workers are serialized on a blocking thread. SQLite's mutex is held for brief progress/read operations and the final transaction, never during file traversal or hashing. Work unfinished at startup is marked interrupted; no implicit resume or automatic mutation retry occurs. Cancellation and standalone verification remain planned. M1 is not complete until its remaining acceptance criteria pass.

Errors are stable IPC codes with localized UI text. Inaccessible/locked files, unsupported filesystem objects, empty folders, links, junctions, and reparse points fail the import. Windows opens content with read sharing only and does not follow a final reparse point; writers/deletion cannot coexist with that handle. Metadata and byte counts are checked before/after reading. This is a trusted local-file importer, not a filesystem sandbox against an adversary concurrently replacing ancestor directories.

## Selection and path policy

The native Rust picker creates a random, process-local authorization token, expiring after five minutes. Only names and the token reach React. Requests cannot supply arbitrary absolute paths. Pending selections are capped at 16; each import accepts up to 100 roots, 100,000 files, and 128 directory levels. Large selections may take time to enumerate before a total is available. No network request is made.

Paths are canonical absolute UTF-8 paths from Rust. Windows extended-length/UNC spelling is retained internally. Relative separators are stored as `/`. Path keys use the exact canonical spelling, without destructive Unicode case folding; this avoids merging distinct names on case-sensitive volumes. Case aliases, hard links, and alternate Windows namespace spellings are not guaranteed to deduplicate locations/items. Content still deduplicates by SHA-256/size. Non-UTF-8 paths fail rather than silently changing identity.

## Storage and contracts

Schema v2 adds persistent UUID-v4 request IDs, consumed selection-token fingerprints, task revisions/update timestamps, and `import_sources` linking canonical roots to items. Repeating an accepted request ID with the same token returns the original task, including after restart; a different token conflicts. The UI retains the request ID if starting the operation fails. Selection errors require a fresh selection when a token has expired or been consumed before acceptance.

Active commands: `library_select_sources`, `library_import`, `library_list`, `tasks_list`, and `task_get`, in addition to the foundation commands. Full snapshots are polled every 1.5 seconds while the page is mounted, with no overlapping scheduled polls. Library pages contain up to 50 items and support loading more; recent tasks show the latest 50. Backend pagination supports 1–100 rows and scopes cursors to the command/filter/order. The backend query supports substring matching (SQLite's built-in case folding is ASCII); the current UI does not expose search.

See [IPC contracts](ipc-contracts.md), [SQLite](database.md), and [ADR 0008](decisions/0008-local-import.md).

## Verification

Rust tests cover recursive Unicode paths, empty files, a known SHA-256 vector, identical-content copies, reimport and changed content, source failure, injected SQL rollback, durable request conflicts, token expiry/consumption, scoped pagination, invalid values, and restart recovery. Windows checks include sharing violations. Frontend tests validate active response shapes, exact decimal strings, request envelopes, task state/progress, and both locales. Temporary fixtures contain synthetic bytes only.

Native file dialogs use [rfd](https://docs.rs/rfd/0.17.2/rfd/struct.FileDialog.html); request IDs use [uuid](https://docs.rs/uuid/latest/uuid/struct.Uuid.html).

Native Windows smoke also passed for picker cancellation, folder and single-file import through review/completion, and Portuguese result labels. Verification used a separate app identifier and synthetic files. An independent closed-catalog read confirmed two items, two content identities, three copies, two completed tasks, exact hashes, and SQLite integrity_check=ok.

A deterministic concurrency regression test pauses the real import worker after its first hash chunk. Separate blocking read operations must return task progress, library contents, and database statistics before that worker is released. The test then checks the final SHA-256 across multiple buffer boundaries and exact terminal progress. This demonstrates that hashing does not hold the catalog mutex; it does not claim a measured UI frame rate or disk latency bound.
