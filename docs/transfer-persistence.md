# Pause, resume and local transfer state

Implemented November 14, 2026 for the experimental desktop adapter. [BitTorrent operations](bittorrent.md) explains discovery, original read-only sharing and peer observations.

## Controls and restart behavior

- **Pause** stops the engine session, closes network listeners, syncs partial download files and releases handles. Received/uploaded counters and the recipe remain. Live peers/speeds become unavailable.
- **Resume and recheck** reopens only the recorded source/output, scans torrent pieces with fast resume disabled, then reopens peer connections. The status is Resuming during preparation. Existing valid pieces are reused; invalid or missing pieces in owned downloads are fetched again. Oversized files are refused rather than truncated. Missing parent directories currently require restoration before retrying.
- **Stop** also retains files and a resumable record, labelled Stopped. Incomplete Stopped/Failed records can be retried explicitly. Default Completed records stay terminal; use Library import to seed those files as a new original collection if needed.
- **Forget record (keep files)** is available only for inactive entries. It deletes the private record and request-replay entry, never payloads, source files or ownership sidecars. It also frees a saved-record slot. Forgetting discards this recipe's resume capability.
- On application startup, Active, Resuming, Stopping and Paused saved records load as **Paused**, with no listener or live observations. Nothing contacts DHT, trackers or peers until an explicit action. Completed/Stopped/Failed history is retained. Continue-seeding preference is retained but does not authorize automatic restart sharing.

Pause/Stop can cancel a resume in preparation. Startup and resume operations are serialized; repeated Resume on an already active entry returns that entry. Generation guards prevent an old session monitor or checkpoint from closing/replacing a newer resumed session. Stop remains independent of unrelated long preparation.

Resume uses the item's saved peer, DHT, tracker, port and continued-seeding settings, not the global fields for new transfers. Assigned listening ports are retained; a busy port or unreachable saved peer may require freeing the port or restoring connectivity. Editing a saved route or relocating a saved folder is not implemented. No duplicate payload directory is created by resume, and a remembered request UUID replays the same accepted record across restarts.

## Integrity and ownership

Original seeding sources remain read-only. A resumed seed must pass a full torrent-piece scan before sharing; changed/missing/locked originals fail without repair or modification. Catalog SHA-256 is checked when original seeding is first created; resumed integrity checks use the saved torrent piece hashes, without silently replacing the torrent identity from a changed catalog.

Downloads use new application-owned UUID directories. A small `.<download-folder>.owner.json` sidecar is created **beside** the payload directory, containing a version, random ownership token and info hash. Keep it with the folder. The private record also remembers the directory's creation timestamp. Resume checks those values, rejects redirected paths/reparse points, and opens only validated manifest-relative regular files without truncation. The sidecar is not part of the torrent or a payload staging copy. These checks prevent accidental adoption of a different folder; they are not a security boundary against a local user who can alter both files and the private database.

Partial files may be shorter than their expected lengths. Existing valid pieces survive pause/restart; corruption is rechecked and repaired only in owned downloads. Unexpected extra files are untouched. Resume does not search other directories or overwrite files outside the recorded root. Original-source read-only protections remain separate from the write authority for an owned download. Completed downloads explicitly continuing to seed are flushed and sealed against adapter writes; after a later resume they are checked again before seeding.

The engine does a full piece recheck rather than trusting cached bitmaps or progress counters. This may take time for large collections and is cancellable before peer activation. Local disk verification has no one-minute metadata-network timeout. DHT bootstrap may start during explicit resume preparation when that route was selected, but the payload is not unpaused before verification. File I/O and SQLite updates run on workers, not the webview thread.

## Private store, checkpoints and limits

The store is `%LOCALAPPDATA%/<application identifier>/transfers/state.sqlite3` (normal identifier `io.github.pedrovitor-dev.game4`), separate from catalog SQLite v4. Transfer-store schema is v1 (`PRAGMA user_version=1`). It holds bounded metainfo, private absolute paths, routing settings (potentially tracker passkeys), identity/replay fields, ownership information, durable statuses and sampled counters. It is private application state, not a public preservation/interchange format. It is not encrypted by this increment; local account/filesystem protections apply. Public `.g4`, `.g4e` and manifest exports do not acquire private transfer paths.

State is committed before a new session can unpause, on lifecycle transitions and approximately every second while active. Frequent checkpoints use a separate small snapshot table, avoiding rewrites of metainfo rows; early v1 stores are upgraded additively without losing records. SQLite transactions and FULL synchronization protect records. A hard exit can lose the last sampling interval of byte counters, so lifetime traffic totals across a crash are last-known observations rather than an accounting ledger. Verified progress is advisory until rechecked. Process recovery and piece hashes handle incomplete OS writes; this test does not certify power-loss behavior of storage hardware.

Maximum 64 saved entries, 64 MiB aggregate stored metainfo, 8 MiB per torrent and four Active/Resuming/Stopping sessions apply. Preview tokens and temporary magnet lookups are not persisted. Unsupported versions, malformed rows or unavailable transfer storage do not trigger a reset: transfer actions become unavailable while Library remains usable. A checkpoint failure stops the affected session rather than silently continuing unsaved. Pause/Stop reports storage failures instead of claiming the transition was saved.

Full-library backup v2 still covers the preservation catalog, not transfer state. For a manual transfer-state backup, close the app and preserve the private transfer directory plus relevant source/output folders and ownership sidecars. Paths/creation stamps are local; moving state between machines or changing folder identity is not automatically supported. Do not publish this SQLite file: it may contain private paths, endpoints and passkeys.

## Validation

Native tests cover partial pause/close/reopen, stable identity and replay, piece corruption repair without a new copy, cumulative sampled counters across clean resumes, changed-original refusal, unchanged source bytes, retained listening port, concurrent Resume idempotency, cancelling Resume, ownership/oversize refusal, safe history removal, and unsupported/corrupt store handling without catalog changes. Existing direct, DHT, tracker and seeding handoff tests remain covered.

A separate acceptance harness uses the integrated desktop `Transfers` API across processes: the client exits abruptly after 1 MiB, a new process opens the same store offline as Paused, rechecks/reuses the partial pieces and completes the same 8 MiB + 17 byte payload. An independent SHA-256 comparison matches the source. See [the result record](evaluations/transfer-recovery-2026-11-14.json).

To reproduce with a **new synthetic probe fixture only**, build `tools/torrent-probe`, run its `prepare --directory <new-dir>`, then build the desktop example with `cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --example transfer_recovery`. Run `transfer_recovery seed <fixture-dir>` in one process (bounded to 90 seconds; it prints its loopback peer), `transfer_recovery crash <fixture-dir> <peer>` in another (intentional nonzero exit without cleanup), then `transfer_recovery resume <fixture-dir>`. Compare the source and received SHA-256 independently. The harness is for local synthetic acceptance and does not certify independent-client or WAN behavior.
