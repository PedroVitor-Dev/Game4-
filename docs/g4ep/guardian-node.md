# Game4∞ Guardian Node — local storage service

Status: G4EP step 4 implemented. This is a real disk-backed `StorageNode` adapter, not a capacity simulation. It admits, hashes, persists and supplies individual fragments. It does **not** start a network listener, advertise availability, enroll peers or implement authenticated computer-to-computer fragment transport. Those adapters remain a separate milestone. Existing BitTorrent sessions are unchanged.

## User controls

Open Settings → Game4∞ Guardian Node. Choose the storage folder while participation is disabled; existing bytes remain in the previous folder. No automatic migration or deletion occurs. The selected directory and inventory persist independently of the game catalog. Participation reopens **disabled** every session, including CLI invocations.

Set the quota in MiB and upload/download limits in KiB/s; zero speed means unlimited. Limits apply to the aggregate stream directions, including local import/export through the storage port. The quota is a logical maximum, not a preallocation or promise of free disk space. Metadata, SQLite journal and filesystem overhead require additional space.

Select a group and fragment index (zero based), choose a valid `.g4e` v2 descriptor, and explicitly authorize that coordinate. Sharing is a separate checkbox, disabled by default. Only hashes and metadata for the selected fragment are retained. Importing a descriptor never admits its entire collection or starts a transfer. The stored fragment is at most 4 MiB; the node never requires the full original game.

Enable participation to store a selected `.g4frag` or save a shared copy. Reads require sharing permission and rehash the stored bytes first. Disable participation to interrupt paced I/O. Revoking permission immediately prevents subsequent reads; it does not delete bytes. Use the explicit revoke-and-delete action or remove unapproved fragments to reclaim space. A blob still referenced by another permission is retained.

The bounded inventory shows up to 100 permissions per page, coordinates, SHA-256, byte sizes, sharing and health. Total physical fragment count can exceed the permission list after revocation. Unknown means not checked since restart, missing means not yet stored or no longer present, verified records a successful check, and corrupt means a size/hash mismatch. The aggregate state is unchecked, ready or needs attention. A ready empty node is operational, not evidence of preserved content. Use Check stored fragments for an offline audit.

## Persistence and atomicity

The service creates the dedicated `Game4-Guardian-v1` child only in an empty/unowned location. Its ownership marker carries a persistent random node identity; that identifier is not a cryptographic peer-authentication key. `state.sqlite3` stores settings, exact admissions and blob observations. This store is separate from the catalog/transfer databases and is not included in library backups. Private directory paths remain local configuration and never enter `.g4e` metadata.

Blobs use lowercase SHA-256 names under `fragments/`; originals and source fragments are opened read-only through the existing protected file helper. Ancestor links/reparse points are rejected. The root must remain owner-controlled; this implementation does not defend against an administrator replacing paths or editing the database concurrently outside the service.

An exclusive OS file lock prevents two processes from owning the same store. Live fragment readers retain the lock. Within the process, writes and quota changes are serialized. Before admission, the complete expected shard size is checked against all physical blob bytes, including unindexed crash orphans. An admitted write owns that reservation until completion; other writers cannot overcommit it.

A bounded 64 KiB loop checks exact length, SHA-256, cancellation, deadline, activity and the global download limit. It writes only a host-named `.part` in `staging/`, synchronizes the file, then atomically publishes without overwriting. Invalid/short/long input, cancellation, quota exhaustion and disk errors leave no committed partial blob. Existing content deduplicates by hash, but duplicate incoming bytes are still validated. SQLite inventory commits after blob publication. A crash in between leaves a complete orphan that counts toward quota on restart, never an implicitly shared fragment. Startup removes only owned staged `.part` files and marks surviving blobs unknown until checked.

Atomic publication prevents partial visibility; this is not a guarantee against every filesystem/hardware power-loss scenario. SQLite uses FULL synchronization. On restart, missing/corrupt bytes remain observable rather than silently recreated. Export uses a staged, synced file and refuses to overwrite an existing destination. Serving holds a protected source handle and rechecks activity and permissions on bounded reads. Already delivered bytes cannot be recalled.

## CLI

```powershell
npm.cmd run guardian -- "C:/Guardian" status
npm.cmd run guardian -- "C:/Guardian" limits 1073741824 262144 262144
npm.cmd run guardian -- "C:/Guardian" approve preservation.g4e 0 0 --share
npm.cmd run guardian -- "C:/Guardian" import 1 fragment.g4frag --enable
npm.cmd run guardian -- "C:/Guardian" export 1 shared-copy.g4frag --enable
npm.cmd run guardian -- "C:/Guardian" verify
npm.cmd run guardian -- "C:/Guardian" revoke 1
npm.cmd run guardian -- "C:/Guardian" prune
```

IDs come from `status`; do not assume they remain constant after removing/reapproving entries. Omit `--share` for storage-only permission. `--enable` authorizes only that CLI session; the next invocation reopens disabled. Keep the original manifest separately to identify reconstruction groups.

## Validation and boundaries

Native tests cover bad hashes, short/long input, duplicate admission, quota races, process/store exclusion, active reader lifetime, disabled/revoked reads, throttled write interruption, restart staging recovery, corrupt/missing/orphan detection, explicit deletion and shared directional limits. IPC tests reject malformed permissions, inventory bounds and unexpected network state.

The [recorded local acceptance](../evaluations/guardian-node-2026-11-14.json) admitted one 1 MiB shard from the retained 700,000,000-byte synthetic fixture into each of two independent stores. The shared copy retained SHA-256 and 256 KiB/s limits were exercised in both directions. This is local adapter evidence; it is not a two-computer network test, distributed durability proof or reconstruction claim. Automatic placement, authentication, remote admission, peer transfer and repair remain open.
