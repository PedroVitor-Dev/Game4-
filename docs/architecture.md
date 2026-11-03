# Planned architecture

Status: accepted initial direction; details remain subject to experiments.

| Layer | Technology / responsibility |
|---|---|
| Interface | React, TypeScript, Tailwind CSS; library, tasks, and transfers |
| Application shell | Tauri 2; explicitly exposed interface commands |
| Local core | Rust; import, hashes, formats, policies, and tasks |
| Persistence | SQLite with versioned migrations |
| Transfers | BitTorrent adapter; engine not yet selected |
| Future integrations | Independent IPFS and timestamp adapters |

## Boundaries

The interface requests operations; the core validates input and enforces permissions. The transfer engine must not own the catalog model. Network failures must not prevent local library browsing. Long operations run outside the UI thread, with progress events and cooperative cancellation.

User-facing strings use locale resources for English and Brazilian Portuguese. Rust errors expose stable codes and structured parameters for localized presentation. Internal identifiers, schema keys, and logs remain stable across locales. See [localization](localization.md).

## Import flow

Select path → enumerate with limits → register task → read file in chunks → calculate hash → check file stability → persist result → show summary and individual failures.

Do not load whole games into memory. Symbolic links, junctions, and cycles require explicit policy; initially do not automatically follow redirected directories. Restarting and retrying must not create unintended duplicate records.

## Initial entities

- File: internal identity, size, hash, and verification state.
- Location: path and associated file; content may have multiple locations.
- Collection item: title, platform, version, and file references.
- Manifest: schema version, entries, metadata, and identifiers.
- Task: type, state, progress, and recoverable error.
- Transfer: BitTorrent identifier, destination, and state; separate from the collection.

Detail the schema in a migration before implementation. Content hash and editorial identity are different concepts: two versions of a game can have different hashes.

## Proposed structure

```text
apps/desktop/src/           Interface and locale resources
apps/desktop/src-tauri/src/ Rust core and adapters
packages/manifest-schema/  Schemas and interoperable examples
docs/decisions/            Architecture decisions
tests/fixtures/            Synthetic or redistributable samples
```

## Engine selection

Create an ADR comparing distribution-compatible licensing, Rust/Tauri integration, maintenance, DHT, magnets, trackers, resume, resource limits, and Windows behavior. Do not choose solely by popularity. Run a real transfer and resume experiment before deciding.

## Verification and distribution

Test domain rules and parsers; integration-test SQLite, files, and transfers. Validate the interface and installer on Windows. Record tool versions. Never include commercial games, keys, or personal data in fixtures. Automatic updates and installer signing require a later infrastructure and key-management decision.
