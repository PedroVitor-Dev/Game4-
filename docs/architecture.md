# Planned architecture

Status: the Tauri / React / Rust shell and localization are implemented. The initial SQLite schema and startup initialization are implemented; file operations and transfers remain planned. Details remain subject to experiments. See [development setup](development.md).

| Layer               | Technology / responsibility                                    |
| ------------------- | -------------------------------------------------------------- |
| Interface           | React, TypeScript, Tailwind CSS; library, tasks, and transfers |
| Application shell   | Tauri 2; explicitly exposed interface commands                 |
| Local core          | Rust; import, hashes, formats, policies, and tasks             |
| Persistence         | SQLite with versioned migrations                               |
| Transfers           | BitTorrent adapter; engine not yet selected                    |
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

The [isolated engine evaluation](torrent-engine-evaluation.md) records local evidence and a creator defect; the desktop still has no BitTorrent engine dependency.

## Engine selection

Create an ADR comparing distribution-compatible licensing, Rust/Tauri integration, maintenance, DHT, magnets, trackers, resume, resource limits, and Windows behavior. Do not choose solely by popularity. Run a real transfer and resume experiment before deciding.

## Verification and distribution

Test domain rules and parsers; integration-test SQLite, files, and transfers. Validate the interface and installer on Windows. Record tool versions. Never include commercial games, keys, or personal data in fixtures. Automatic updates and installer signing require a later infrastructure and key-management decision.

## Implemented local import

Native source selection authorizes process-local tokens. Serialized blocking workers enumerate and stream SHA-256 outside the database mutex. Schema v2 persists request IDs, progress/revisions, and source identities; catalog mutations and successful completion share one transaction. React polls task snapshots while mounted and lists library items. Events, cooperative cancellation, and standalone rechecks remain planned. See [library import](library-import.md) and [ADR 0008](decisions/0008-local-import.md).

The `backups` Rust module implements bounded private catalog snapshots, independent validation, preview tokens/fingerprints, recovery files, and transactional replacement. See [library backup](library-backup.md).
