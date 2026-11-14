# Game4∞ documentation

An open network for preserving, verifying, and sharing digital collections.

| Document                                            | Purpose                                                      |
| --------------------------------------------------- | ------------------------------------------------------------ |
| [Product plan](product-plan.md)                     | Vision, audiences, scope, and priorities                     |
| [Roadmap](roadmap.md)                               | Estimates, dependencies, milestones, and acceptance criteria |
| [Architecture](architecture.md)                     | Components, data flow, and boundaries                        |
| [Preservation and formats](preservation.md)         | Integrity, metadata, recovery, and interoperability          |
| [Decentralization and funding](decentralization.md) | Guardians, IPFS, Bitcoin, and experimental integrations      |
| [Localization](localization.md)                     | English repository content and bilingual application policy  |
| [Security](security.md)                             | Threats and planned controls                                 |
| [Project log](project-log.md)                       | Current status, immediate backlog, and progress records      |
| [Risks](risks.md)                                   | Uncertainties, mitigation, and review triggers               |
| [Decisions](decisions/README.md)                    | Architecture decision records                                |

See also [SQLite storage](database.md), [project structure](project-structure.md), and [development setup](development.md).

See [torrent engine evaluation](torrent-engine-evaluation.md) and [two-computer validation](torrent-validation.md) for the isolated P2P experiment and remaining gates.

See [frontend–backend IPC contracts](ipc-contracts.md) for active commands, generated types, errors, and draft task/event boundaries.

## Maintenance and status vocabulary

- Planned: development direction, not implemented functionality.
- Experimental: requires research and validation before adoption.
- Implemented: supported by code and recorded verification.
- Done: meets the delivery's acceptance criteria.

Update affected documents in the same commit as a relevant change. Record lasting decisions in ADRs and progress in the project log. Dates are estimates, not release promises.

Planning baseline: November 3, 2026. As of November 4, the desktop foundation, bilingual interface, and initial SQLite storage are implemented; local file/folder import with SHA-256 and tasks is implemented; P2P integration remains planned. GitHub-facing content is initially in English. See the [development setup](development.md) for commands and prerequisites.

See [local library import](library-import.md) for file/folder selection, SHA-256, reimport, tasks, and limitations.

See [manifest v1](manifest-v1.md) for the public JSON format, schema, compatibility, and Library export.

See [save and restore the local library](library-backup.md) for private full-catalog backups and recovery.

See [platform, format, and size identification](file-identification.md) for detector scope, evidence, file details, and compatibility.

- [Manual library integrity](library-integrity.md): registered-copy rechecks, task outcomes and historical observations.

- [Public .g4 package specification](../packages/g4-format/spec-v1.md): ZIP64, layout, limits, integrity and offline tooling.

- [Original preservation contract](original-preservation.md): read-only originals and evidence across local workflows.
- [Public .g4e specification](../packages/g4e-format/spec-v1.md): Game4 Ever Manifest discovery/P2P references and offline validation.

- [Creating and reading .g4e](ever-manifests.md): native original-file torrent references, offline CLI and explicit integrity checks.

## Desktop BitTorrent increment — November 13, 2026

The desktop now embeds real experimental BitTorrent sessions, creates and imports public v1 `.torrent` files, seeds original catalog files through read-only storage and downloads to new owned directories. Offline previews and explicit initial-peer actions are available in both locales. DHT/trackers/local discovery/UPnP are disabled; peer exchange remains enabled. Sessions are runtime-only, with Stop and completion cleanup; persistence, resume, discovery and production acceptance remain open. See [desktop transfers](bittorrent.md) and [ADR 0016](decisions/0016-desktop-bittorrent.md). Earlier dated experiment statements describe their original scope.

## Magnets and DHT — November 14, 2026

Transfers now generates magnets without a metainfo export, imports them through cancellable metadata-only acquisition, and optionally discovers/announces peers over ephemeral UDP DHT. DHT is off by default; enabled actions can omit the initial peer. Strict v1/hash/path validation and explicit destination review remain in place. Real isolated-DHT discovery and transfer tests pass; public-WAN/NAT acceptance, persistence and resume remain open. See [desktop transfers](bittorrent.md) and [ADR 0017](decisions/0017-magnets-and-dht.md).

## Selected trackers — November 14, 2026

Transfers supports explicitly selected HTTP/HTTPS/UDP trackers for magnet metadata, payload discovery and generated torrent/magnet references. Imported endpoints are suggestions with opt-in off; only selected routes reach the engine. Exact torrent info bytes and read-only originals are preserved. Local HTTP/UDP tests transfer successfully without DHT or a manual peer and leave unselected trackers untouched. See [tracker operations](bittorrent.md) and [ADR 0018](decisions/0018-selected-trackers.md). Public tracker/WAN and independent-client acceptance remain open.

## Upload, seeding and peer observations — November 14, 2026

Transfers can explicitly continue seeding a completed download and now displays received/uploaded bytes, sampled rates, connected seeders/other peers and a conservative remote availability indicator. Inactive observations are unavailable; global swarm populations and exact combined-piece coverage are not invented. A real three-session handoff test proves the completed download uploads correctly after the original source goes offline. See [the current guide](bittorrent.md) and [ADR 0019](decisions/0019-transfer-observations-and-seeding.md).
