<div align="center">

# 🎮 Game4∞

### Preserve the Past. Share the Future.

**An open, decentralized network for video game preservation.**

![Status](https://img.shields.io/badge/status-in%20development-yellow)
![License](https://img.shields.io/badge/license-GPLv3-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Open Source](https://img.shields.io/badge/open%20source-❤️-red)

**Preserve. Share. Decentralize.**

</div>

## 📖 Project documentation

See the [documentation index](docs/README.md) for the product plan, estimated schedule, architecture, preservation strategy, security, risks, decisions, and progress records.

The features below are planned. See the [project log](docs/project-log.md) for actual implementation status.

## 🌌 About

**Game4∞** is an open-source project dedicated to digital video game preservation through decentralized technologies and peer-to-peer (P2P) networks.

Our mission is to help communities preserve, organize, verify, and share digital files independently, without relying exclusively on centralized servers or commercial platforms.

Video game history deserves to survive for future generations. Game4∞ aims to provide open infrastructure for long-term preservation, beyond file downloads.

> **Our vision:** build technology that keeps working even if its original creators stop maintaining it.

## 🎯 Goals

- Support the historical preservation of video games.
- Develop decentralized infrastructure and open preservation formats.
- Verify file integrity using cryptographic hashes.
- Enable communities to maintain their own collections.
- Provide free, open-source tools.
- Reduce dependence on centralized servers and organizations.
- Encourage collaboration and interoperability.

## 🚧 Development status

**Current stage: planning and initial development toward v0.1.0-alpha.**

The desktop foundation is implemented: Tauri 2, React, TypeScript, Rust, Tailwind CSS, navigation, and persistent language selection. The native Windows debug executable has been built and smoke-tested. SQLite storage and versioned migrations are initialized at startup. Local file/folder import, streamed SHA-256, durable tasks, and library browsing are implemented. Experimental [desktop BitTorrent transfers](docs/bittorrent.md) now support `.torrent` creation/import, read-only seeding of originals and downloads with an explicit initial IPv4 peer. Session persistence, resume and discovery remain planned. See [library import](docs/library-import.md). Versioned [JSON manifest v1](docs/manifest-v1.md) exports are also implemented, with a public schema and offline semantic validator. Settings also provides [private full-library backup and restore](docs/library-backup.md), with a recovery copy before catalog replacement. [Platform/format identification and readable sizes](docs/file-identification.md) are available in Library summaries and file details.

To run the project, follow the [development setup](docs/development.md). The remaining features below are development goals.

The first application targets Windows, starting with a local library and progressing to P2P transfers.

## 🌍 Languages

The desktop foundation supports **English (`en`) and Brazilian Portuguese (`pt-BR`)**. English is the default and fallback language, with a persistent language selector. Future features must maintain both locales.

Repository documentation, GitHub-facing content, code identifiers, and commit messages use English initially. Portuguese application support remains part of the product scope. See the [language policy](docs/localization.md).

## ⚡ Planned features

### 📚 Local library

- Import files and directories.
- Organize by platform and category.
- Identify file sizes and formats.
- Generate SHA-256 hashes.
- Store metadata locally.
- Verify integrity and export manifests.

### 🌐 P2P sharing

Game4∞ plans to use technologies compatible with the BitTorrent ecosystem for decentralized transfers:

- Torrent creation and import, plus magnet links.
- Peer discovery through DHT and tracker compatibility.
- Simultaneous downloads and uploads.
- Seeder and leecher information.
- Pause, resume, transfer persistence, and speed monitoring.
- Upload and download limits.
- Piece integrity verification.

Files will move directly between participants when connectivity allows, without relying on Game4∞ servers to carry their contents.

### 🔐 Privacy and control

- No mandatory account or personal telemetry by default.
- Explicit action to start sharing.
- Control over shared files and local settings.
- Clear information about P2P connections.

**Note:** BitTorrent connections can expose participants' public IP addresses to other peers.

## 📦 Game4∞ formats

The project defines open, documented formats for preservation and sharing. `.g4` v1 is implemented; `.g4e` v1 has native creation/reading and offline verification; `.g4lib` remains a proposal. See [preservation and formats](docs/preservation.md).

### `.g4` — Game4 Package

A portable preservation package that may contain original files, a preservation manifest, historical information, documentation, authorized artwork, and integrity hashes.

```text
example.g4
├── game4.json
├── manifest.json
└── data/
    ├── game/original-file.bin
    ├── artwork/cover.png
    └── docs/manual.txt
```

Implemented v1 uses stored ZIP/ZIP64, manifest v1 and full SHA-256 validation. Create packages from Library items or open them for verification. See the [public specification](packages/g4-format/spec-v1.md) and [offline CLI](packages/g4-format/README.md). No automatic extraction, execution or sharing occurs.

### `.g4e` — Game4 Ever Manifest

A separate JSON descriptor for discovery and P2P references: preservation metadata, raw-file SHA-256, BitTorrent v1 info-hashes and matching magnet links. The [public v1 specification](packages/g4e-format/spec-v1.md), native Library creation/reading, [offline CLI](docs/ever-manifests.md), schema, examples and independent validators are available. Original-file torrent references are created without conversion or payload copies. In-app torrent sessions remain pending. This replaces the earlier conceptual `.g4t` name.

Sharing originals directly must remain available without creating a `.g4`, converting files or staging a mandatory duplicate. Package creation is optional. See the [original preservation contract](docs/original-preservation.md). Neither format requires password encryption, DRM or a central server.

### `.g4lib` — Game4 Library

A future catalog format for library export, catalog import, community mirroring, metadata synchronization, and digital signature verification.

All formats should prioritize public specifications, interoperability, verifiable integrity, original file preservation, independence from proprietary services, and future compatibility.

## 🏗️ Planned architecture

| Component      | Technology           |
| -------------- | -------------------- |
| Desktop shell  | Tauri 2              |
| Local backend  | Rust                 |
| Frontend       | React and TypeScript |
| Styling        | Tailwind CSS         |
| Database       | SQLite               |
| P2P transfers  | BitTorrent           |
| Peer discovery | DHT / trackers       |
| Integrity      | SHA-256              |
| Metadata       | JSON                 |
| Distribution   | GitHub Releases      |

The BitTorrent engine will be selected after compatibility, performance, security, and licensing evaluation. Components should evolve independently.

### Proposed structure

```text
Game4Infinity/
├── apps/
│   └── desktop/
│       ├── src/
│       │   ├── components/
│       │   ├── pages/
│       │   ├── hooks/
│       │   └── services/
│       └── src-tauri/
│           └── src/
│               ├── commands/
│               ├── database/
│               ├── library/
│               ├── torrent/
│               └── security/
├── packages/
│   └── manifest-schema/
├── docs/
├── tests/
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

This tree includes planned directories and files that do not exist yet.

## 🗺️ Roadmap

See the [estimated schedule and acceptance criteria](docs/roadmap.md). Custom formats are subject to specification and validation; they are not prerequisites for the P2P prototype.

### Phase 1 — Local foundation and P2P

- [x] Set up Tauri, React, Rust, and the initial interface.
- [x] Establish English and Portuguese localization.
- [x] Initialize SQLite storage and versioned migrations.
- [ ] Build the local library and SHA-256 hashing.
- [ ] Create manifests and support offline catalog reconstruction.
- [x] Specify and implement initial `.g4` v1 creation and full reading/verification.
- [x] Specify `.g4e` v1 with schema, examples and offline validation.
- [x] Create/read `.g4e` in the desktop app and generate original-file torrent references without `.g4` conversion.
- [x] Test compatibility and integrity of manifests, packages and torrent references.
- [ ] Integrate in-app seeding, downloads and P2P discovery for original files.
- [ ] Integrate and validate a BitTorrent engine.
- [ ] Support torrents, magnet links, DHT, and P2P uploads/downloads.
- [ ] Provide peer information, transfer persistence, and integrity checks.
- [ ] Complete security testing and prepare a Windows alpha.

### Phase 2 — Decentralized communities

- [ ] Evaluate the `.g4lib` format.
- [ ] Support community catalogs, import, export, and discovery.
- [ ] Add digital signatures and metadata synchronization.
- [ ] Develop community moderation and library mirroring.
- [ ] Support interoperability between communities.

### Phase 3 — Long-term preservation

- [ ] Add optional IPFS integration.
- [ ] Develop guardian nodes and voluntary replication.
- [ ] Show observed availability and identify at-risk content.
- [ ] Schedule integrity checks and support recovery and redundancy.
- [ ] Add optional OpenTimestamps proofs anchored to Bitcoin.
- [ ] Research community storage incentives.

## ♾️ Decentralized preservation

Digital preservation should not depend on a single website, server, or organization. Game4∞ aims to support replicated files, exportable and reconstructable catalogs, independently verifiable metadata, community infrastructure, and software that survives its original maintainers.

**Decentralization does not guarantee permanent storage.** Availability depends on accessible copies, data integrity, and continued community participation.

## 🔗 Bitcoin and decentralized technologies

- **Bitcoin / OpenTimestamps:** optional timestamp proofs for manifests without storing game files on the blockchain. These proofs do not establish authorship or availability.
- **IPFS:** optional content addressing and complementary replication, requiring actual retention by nodes.
- **Funding:** future optional donations and collection sponsorship; paid decentralized storage remains experimental.

Game4∞ will not require cryptocurrency, mining, or payments for essential features. A native token, NFTs, custody, and automatic seeding rewards are outside the initial scope. See the [decentralization plan](docs/decentralization.md).

## ⚖️ Copyright

Game4∞ is a general-purpose tool for digital preservation and P2P sharing. It does not grant reproduction or distribution rights over copyrighted works. Users are responsible for checking authorization or another applicable legal basis for sharing content.

The project encourages preservation and distribution of open-source games, homebrew, public-domain works, games released by their rights holders, content licensed for redistribution, and lawfully distributed historical documentation.

A game's age, rarity, or commercial unavailability does not by itself authorize its distribution.

## 🤝 Contributions

Game4∞ is intended to grow through open collaboration: features, bug fixes, performance improvements, security testing, documentation, translations, design, user experience, and preservation research.

Contribution guidelines will be documented in `CONTRIBUTING.md`. Public repository contributions should initially use English; application translations are an exception.

## 🔒 Security

Planned safeguards include no automatic execution of received files, path validation, prevention of writes outside authorized destinations, download integrity verification, explicit sharing consent, minimal data collection, dependency maintenance, and documented vulnerability handling.

See the [security plan](docs/security.md) for controls and release checks.

## 📜 License

**Game4∞** is licensed under the **GNU General Public License version 3 or later (GPL-3.0-or-later)**.

You may use, study, modify, and redistribute the software, including commercially, subject to the GPL and its applicable corresponding-source requirements. See [LICENSE](LICENSE) for the full text.

The software license does not automatically apply to games or files shared by users.

## 💜 Philosophy

Video games are part of the history of technology, art, music, and digital culture. Preserving that history requires open, interoperable tools that can survive technological change. Digital preservation is a shared responsibility.

**No single server should be the sole guardian of history.**

<div align="center">

### 🎮 Game4∞

**Preserve the Past. Share the Future.**

_Built to preserve. Designed to last._

**Open Source • P2P • Digital Preservation**

</div>
