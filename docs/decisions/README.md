# Architecture decisions

ADRs capture context, decisions, and consequences. An accepted decision may be superseded; retain the record and link its replacement.

| ADR                                | Status                     | Subject                                          |
| ---------------------------------- | -------------------------- | ------------------------------------------------ |
| [0001](0001-product-foundation.md) | Accepted initial direction | Local foundation, P2P, and optional integrations |
| [0002](0002-language-policy.md)    | Accepted                   | English repository and bilingual application     |

| [0003](0003-desktop-tooling.md) | Accepted | Desktop tooling and localization foundation |

| [0004](0004-sqlite-foundation.md) | Accepted | SQLite persistence and modular folder structure |

| [0005](0005-torrent-engine-experiment.md) | Accepted experiment | Isolated torrent evaluation; production selection pending |

| [0006](0006-metainfo-creator-adapter.md) | Accepted experiment | Streaming BEP 3 creator replaces defective upstream generator |

| [0007](0007-ipc-contracts.md) | Accepted | Rust-owned IPC contracts and validated frontend client |

| [0008](0008-local-import.md) | Accepted | Atomic local import, SHA-256, task recovery, and reimport policy |

Upcoming decisions: BitTorrent engine, manifest v0, `.g4` container, catalog identity and signatures, distribution, and updates.

Template: title, date, status, context, alternatives, decision, consequences, and validation evidence. Do not mark experimental technology as accepted before its experiment.
