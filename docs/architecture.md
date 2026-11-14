# Planned architecture

Status: the Tauri / React / Rust shell and localization are implemented. The initial SQLite schema and startup initialization are implemented; file operations and transfers remain planned. Details remain subject to experiments. See [development setup](development.md).

| Layer               | Technology / responsibility                                    |
| ------------------- | -------------------------------------------------------------- |
| Interface           | React, TypeScript, Tailwind CSS; library, tasks, and transfers |
| Application shell   | Tauri 2; explicitly exposed interface commands                 |
| Local core          | Rust; import, hashes, formats, policies, and tasks             |
| Persistence         | SQLite with versioned migrations                               |
| Transfers           | Experimental librqbit 9.1.0 adapter                            |
| Future integrations | Independent IPFS and timestamp adapters                        |

## Boundaries

The interface requests operations; the core validates input and enforces permissions. The transfer engine must not own the catalog model. Network failures must not prevent local library browsing. Long operations run outside the UI thread, with progress events and cooperative cancellation.

Public DTOs are Rust-owned and generate TypeScript through the private IPC package. Active responses are validated at runtime, with version/capability negotiation and stable localized error codes. See [IPC contracts](ipc-contracts.md).

User-facing strings use locale resources for English and Brazilian Portuguese. Rust errors expose stable codes and structured parameters for localized presentation. Internal identifiers, schema keys, and logs remain stable across locales. See [localization](localization.md).

## Import flow

Select path → enumerate with limits → register task → read file in chunks → calculate hash → check file stability → persist result → show summary and individual failures.

Do not load whole games into memory. Symbolic links, junctions, and cycles require explicit policy; initially do not automatically follow redirected directories. Restarting and retrying must not create unintended duplicate records.

## Initial entities

- File: internal identity, size, and hash. Verification state belongs to each physical location.
- Location: path and associated file; content may have multiple locations.
- Collection item: title, platform, version, and file references.
- Manifest: schema version, entries, metadata, and identifiers.
- Task: type, state, progress, and recoverable error.
- Transfer: BitTorrent identifier, destination, and state; separate from the collection.

The initial schema is defined in migration `0001_initial.sql`; see [SQLite storage](database.md). Content hash and editorial identity are different concepts: two versions of a game can have different hashes.

See the [current project structure](project-structure.md) for implemented and reserved boundaries.

## Proposed structure

```text
apps/desktop/src/           Interface and locale resources
apps/desktop/src-tauri/src/ Rust core and adapters
packages/manifest-schema/  Schemas and interoperable examples
docs/decisions/            Architecture decisions
tests/fixtures/            Synthetic or redistributable samples
```

The [isolated engine evaluation](torrent-engine-evaluation.md) records local evidence and a creator defect; the desktop now embeds pinned librqbit behind the experimental `torrents` adapter; production acceptance remains open.

## Engine selection

Create an ADR comparing distribution-compatible licensing, Rust/Tauri integration, maintenance, DHT, magnets, trackers, resume, resource limits, and Windows behavior. Do not choose solely by popularity. Run a real transfer and resume experiment before deciding.

## Verification and distribution

Test domain rules and parsers; integration-test SQLite, files, and transfers. Validate the interface and installer on Windows. Record tool versions. Never include commercial games, keys, or personal data in fixtures. Automatic updates and installer signing require a later infrastructure and key-management decision.

## Implemented local import

Native source selection authorizes process-local tokens. Serialized blocking workers enumerate and stream SHA-256 outside the database mutex. Schema v2 persists request IDs, progress/revisions, and source identities; catalog mutations and successful completion share one transaction. React polls task snapshots while mounted and lists library items. Events, cooperative cancellation, and standalone rechecks remain planned. See [library import](library-import.md) and [ADR 0008](decisions/0008-local-import.md).

The `backups` Rust module implements bounded private catalog snapshots, independent validation, preview tokens/fingerprints, recovery files, and transactional replacement. See [library backup](library-backup.md).

## Portable package boundary

`packages/g4-format/` owns the public .g4 v1 specification, descriptor schema and independent fixtures. Rust `packages` handles streaming ZIP/ZIP64 creation and complete read validation; native package commands only provide pickers, idle-library coordination and DTO summaries. ZIP compression/encryption features are disabled. No package operation executes content, extracts payload or imports it automatically.

## Discovery descriptor boundary

`packages/g4e-format/` specifies .g4e v1, with schema, synthetic examples/companions and a Node offline reference validator. The format package itself is offline. The native `torrents` adapter separately maps original source files through private read-only handles, while descriptor validation and content verification remain independent. No central service or mandatory staging copy is a prerequisite.

## .g4e implementation boundary

Rust `ever` owns strict parsing/serialization, direct-original torrent generation and explicit metainfo/payload verification. Native commands expose export/open only and run off the UI thread. The Node public validator remains an independent implementation. A standalone librqbit example checks torrent parse compatibility without starting a session. These offline format commands never start the separately integrated experimental network engine.

## Desktop BitTorrent increment — November 13, 2026

The desktop now embeds real experimental BitTorrent sessions, creates and imports public v1 `.torrent` files, seeds original catalog files through read-only storage and downloads to new owned directories. Offline previews and explicit initial-peer actions are available in both locales. DHT/trackers/local discovery/UPnP are disabled; peer exchange remains enabled. Sessions are runtime-only, with Stop and completion cleanup; persistence, resume, discovery and production acceptance remain open. See [desktop transfers](bittorrent.md) and [ADR 0016](decisions/0016-desktop-bittorrent.md). Earlier dated experiment statements describe their original scope.

## Magnets and DHT — November 14, 2026

Transfers now generates magnets without a metainfo export, imports them through cancellable metadata-only acquisition, and optionally discovers/announces peers over ephemeral UDP DHT. DHT is off by default; enabled actions can omit the initial peer. Strict v1/hash/path validation and explicit destination review remain in place. Real isolated-DHT discovery and transfer tests pass; public-WAN/NAT acceptance, persistence and resume remain open. See [desktop transfers](bittorrent.md) and [ADR 0017](decisions/0017-magnets-and-dht.md).

## Selected trackers — November 14, 2026

Transfers supports explicitly selected HTTP/HTTPS/UDP trackers for magnet metadata, payload discovery and generated torrent/magnet references. Imported endpoints are suggestions with opt-in off; only selected routes reach the engine. Exact torrent info bytes and read-only originals are preserved. Local HTTP/UDP tests transfer successfully without DHT or a manual peer and leave unselected trackers untouched. See [tracker operations](bittorrent.md) and [ADR 0018](decisions/0018-selected-trackers.md). Public tracker/WAN and independent-client acceptance remain open.

## Upload, seeding and peer observations — November 14, 2026

Transfers can explicitly continue seeding a completed download and now displays received/uploaded bytes, sampled rates, connected seeders/other peers and a conservative remote availability indicator. Inactive observations are unavailable; global swarm populations and exact combined-piece coverage are not invented. A real three-session handoff test proves the completed download uploads correctly after the original source goes offline. See [the current guide](bittorrent.md) and [ADR 0019](decisions/0019-transfer-observations-and-seeding.md).

## Transfer recovery — November 14, 2026

Pause, reverified Resume and private transfer persistence are now implemented in a separate SQLite v1 store; catalog SQLite v4 and backup v2 remain unchanged. Interrupted work reopens Paused without networking. Owned downloads are repaired without duplication or truncation, while original seeding sources remain read-only. Inactive records can be forgotten without deleting content. See [transfer persistence](transfer-persistence.md) and [ADR 0020](decisions/0020-private-transfer-recovery.md) for storage, credentials, sidecars, limits and process-restart evidence.
