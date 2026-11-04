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

## Maintenance and status vocabulary

- Planned: development direction, not implemented functionality.
- Experimental: requires research and validation before adoption.
- Implemented: supported by code and recorded verification.
- Done: meets the delivery's acceptance criteria.

Update affected documents in the same commit as a relevant change. Record lasting decisions in ADRs and progress in the project log. Dates are estimates, not release promises.

Planning baseline: November 3, 2026. As of November 4, the desktop foundation, bilingual interface, and initial SQLite storage are implemented; file import and P2P features remain planned. GitHub-facing content is initially in English. See the [development setup](development.md) for commands and prerequisites.
