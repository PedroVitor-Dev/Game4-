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
