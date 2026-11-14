# Schedule and milestones

Current product/experience baseline (November 14, 2026): [vision and user experience](vision-and-experience.md). It reconciles the renewed community preservation plan with implemented local Library, direct P2P, recovery and speed controls; older dated scope notes below are historical. Community catalog/discovery and .g4lib remain planned.

Baseline: November 3, 2026. Initial estimate: 16 active weeks for a P2P alpha, followed by a 4-week contingency. Assumes one main developer with regular availability and project-owner review. The estimate has not been validated against historical delivery velocity.

Weeks start when implementation actually begins. Holidays, availability, incompatibilities, and scope changes may shift dates. Review estimates after each milestone; acceptance criteria take precedence over dates.

| Milestone               | Weeks | Delivery                                                             | Exit criteria                                                                             |
| ----------------------- | ----- | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| M0 — Foundation         | 1–2   | Tauri, React, TypeScript, Rust, CI, base interface, and localization | Windows build and launch; basic automated checks; persistent English/Portuguese selection |
| M1 — Library            | 3–5   | Import, SQLite, hashes, and browsing                                 | Persistence after restart; cancellation and problematic files handled                     |
| M2 — Local preservation | 6–7   | Manifest v0, export, and verification                                | Catalog reconstruction and corruption detection demonstrated                              |
| M3 — P2P prototype      | 8–9   | BitTorrent engine experiment                                         | Transfer between two installations; license and integration assessed                      |
| M4 — Transfers          | 10–12 | Queue, torrents, magnets, limits, and resume                         | State survives restart; final integrity verified                                          |
| M5 — Windows alpha      | 13–16 | Integration, tests, installer, and documentation                     | Clean installation, restoration, security review, and both locales verified               |
| Contingency             | 17–20 | Fixes and compatibility                                              | Use for observed risks, not additional features                                           |

## Current milestone status — November 4, 2026

M0 foundation code is implemented and locally verified through a Windows native debug build and WebView2 smoke test. English/Portuguese switching and persistence work. The CI workflow is configured but has not run on GitHub. A clean-machine build and installation check remains required before declaring the full M0 exit criterion met. M1 import/hashing/browsing is implemented with task progress, error handling, and restart recovery; cancellation and independent rechecks remain open. Other milestones remain planned; no delivery-velocity claim is made from this initial setup.

## M3 experiment status — November 4, 2026

The isolated librqbit 9.1.0 probe passed loopback transfer, session/process resume, corruption repair, magnet, and SHA-256 checks. Its exact-piece-boundary torrent creator failed. The local creator adapter now passes boundary, multi-file, and independent metadata checks. Two physical Windows PCs on different networks subsequently passed transfer, process restart, independent SHA-256, and magnet over Tailscale DERP relay. Native direct-WAN/NAT, discovery, independent-client, and production adoption gates remain open. M3 remains in progress; no production engine selection is approved. See [the evaluation](torrent-engine-evaluation.md) and [test procedure](torrent-validation.md).

## Dependencies and alternatives

- M0 requires available Windows tools and SDKs. Localization is part of M0; review its effect on estimates when implementation starts.
- M1 defines the model used by M2. M3 may affect integration, but the catalog must remain independent of the engine.
- M3 is a decision gate: if no engine meets requirements, release a local-library version first and replan P2P.
- `.g4` v1 has a public specification, descriptor schema, fixtures and create/read implementation. `.g4e` v1 has native create/read, direct-original torrent references, a public specification, schema, examples and offline verification; in-app sessions/discovery remain pending. Original-file sharing must not require packaging or staging copies. Neither format blocks the P2P prototype; use `.torrent` files and JSON manifests first.
- Alpha does not imply stable formats. Document migrations and limitations.

## Later horizon without committed dates

| Phase       | Scope                                             | Prerequisite                                           |
| ----------- | ------------------------------------------------- | ------------------------------------------------------ |
| Communities | Signed catalogs, history, and mirroring           | Usable alpha and validated manifest format             |
| Guardians   | Quotas, scheduling, health, and rescue            | Reliable availability observation and sharing controls |
| Long term   | Recovery, offline kits, IPFS, and timestamps      | Restoration tests and compatibility policy             |
| Funding     | Donations, sponsorship, and paid-storage research | Demonstrated need, costs, and responsibilities         |

## Cadence

Plan one small increment at a time. On completion, validate it, update the log, record new risks, and commit. At each milestone, compare estimates with observed effort, review the backlog, and present the next delivery. Publication and push remain under the project owner's control.

## M2 manifest increment — November 4, 2026

Manifest v1, public JSON Schema, offline parser/validation, and per-item Library JSON export are implemented and locally tested. Export records stored catalog hashes rather than performing a new disk check. Manifest import/catalog reconstruction and the offline survival test remain open, so M2 is not complete. See [the format](manifest-v1.md).

Manual registered-copy SHA-256 verification is implemented (November 13). See [behavior and limits](library-integrity.md). M2 remains open for catalog reconstruction and offline survival validation.

The .g4 v1 increment (November 13) adds public layout/limits/schema, synthetic ZIP32/ZIP64 fixtures, background desktop export/open and an offline create/read CLI. Extraction, catalog reconstruction and the full offline survival milestone remain open.

The .g4e v1 specification increment (November 13) defines original-files/package representations, BitTorrent v1/magnet identity binding, limits, examples and offline validation. Before P2P delivery, implement read-only native source mapping without mandatory staging copies, acquired-metainfo validation, explicit sharing controls and real transfer verification. Format validation is not completion of that milestone.

The .g4e creation/reading increment (November 13) implements native actions and an offline CLI, checks original SHA-256 while generating standard torrent references without .g4/staging copies, and adds full explicit local descriptor/torrent/source verification. Format compatibility tests now span Node, Rust, independent Python fixtures and librqbit parsing. In-app seeding/download sessions and real transfer validation remain open.

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
