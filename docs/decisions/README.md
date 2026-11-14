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

| [0009](0009-versioned-manifests.md) | Accepted initial format | Versioned portable JSON manifests and safe Library export |

| [0010](0010-library-backup-restore.md) | Accepted | Validated private library backups and transactional restoration |

| [0011](0011-file-identification.md) | Accepted initial detector | Content signatures, filename hints, exact sizes, and compatibility |

| [0012](0012-manual-integrity.md) | Accepted | Background manual integrity checks and atomic copy observations |

| [0013](0013-g4-package.md) | Accepted initial format | Public stored ZIP/ZIP64 .g4 packages, creation and full read validation |

| [0014](0014-ever-manifest-and-originals.md) | Accepted initial specification | Original-file preservation and public .g4e Game4 Ever Manifest |

| [0015](0015-ever-create-read.md) | Accepted reference implementation | Native .g4e create/read, original-file torrents and explicit verification |

| [0016](0016-desktop-bittorrent.md) | Accepted experimental integration | Real desktop BitTorrent, offline .torrent actions and original-safe storage |
