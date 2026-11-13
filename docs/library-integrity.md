# Manual library integrity verification

Status: implemented November 13, 2026. Available in English and Brazilian Portuguese from each Library item’s **Verify integrity** action.

## Scope and interpretation

The worker reads every distinct registered physical location for content referenced by the selected item. This includes copies registered by other items with the same SHA-256 and size. Repeated relative entries do not cause repeated checks of the same location. It does not discover new files, scan directories, reconstruct missing paths, or repair originals. Content with no registered location is reported separately as **unlocated**; no filesystem check can be performed for it.

Each readable regular file is streamed through SHA-256 and compared with the catalog’s original hash and size. Matching bytes are **verified**; a different size/hash or a detected mutation during reading is **changed**. A missing path is **missing**. Locked files, denied access, directories, links and Windows reparse points are **inaccessible**. An inaccessible copy does not establish corruption. Source files and reference content identities are never rewritten.

Observations describe a past read. A file can change afterward. The latest timestamp is the most recent observation among the item’s copies, not a claim that every copy was checked at that time. Imports already record their original hash observations. Counts concern physical copies rather than relative manifest entries.

## Background work and persistence

`library_verify({requestId,itemId})` returns an accepted queued task. Request IDs must be canonical UUID v4 values; item IDs must be canonical positive SQLite IDs. Repeating the same request returns its existing task; changing the item or colliding with an import request is rejected. A new intentional check uses a new request ID.

Imports and verification share a serialized worker. The worker runs with `spawn_blocking`, reuses a 1 MiB buffer, and releases the database mutex during file reads. Task progress counts physical copies, updates at most once per 200 ms between files, and always stores an exact terminal count. One large file may keep the file count unchanged while hashing. The plan is limited to 500,000 copies; larger plans fail with `VERIFICATION_LIMIT_EXCEEDED`.

All location observations and task success commit in one SQLite transaction. An operational failure rolls back the batch, preserves previous health, and marks the task failed. Restart recovery marks unfinished tasks interrupted. Changed, missing and inaccessible files are findings from a completed check, not operational task failures. A succeeded task alone does not establish that every copy matches; the interface directs the user to the Library counts and flags recorded problems, including unlocated content.

`library_integrity({itemId})` returns latest stored counts (`verified`, `changed`, `missing`, `inaccessible`, `unverified`, `unlocated`) and nullable `lastCheckedAt`. It performs database reads only and exposes no absolute paths. The interface reloads these observations after successful tasks. Results are current aggregate observations, not immutable per-run reports.

SQLite remains schema v4 and private backups remain v2: existing location health and task columns carry the feature. Backup restore preserves task history but resets copy health to unverified with no timestamp; perform a new check after restore. Portable manifest v1 and identification extensions remain unchanged and contain no copy-health fields.

## Validation and limits

Synthetic tests cover identical copies across items, empty content, same-size corruption, size changes, missing files, directory replacement, exclusive Windows locks, unchanged reference hashes, persisted observations, request replay and conflicts, reads during hashing, final transaction rollback, and backup restore followed by rechecking. Frontend tests cover request envelopes, capability gating, browser mode and malformed health responses.

Cancellation, scheduling, repairs, per-copy path browsing, and immutable run reports remain future work. This verifies consistency with the local catalog, not publisher authenticity or a trusted external preservation database.
