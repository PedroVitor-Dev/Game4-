# Project management and records

## Status on November 13, 2026

- Name: Game4∞.
- Stage: desktop foundation, SQLite v4, platform/format identification and exact-size browsing, versioned manifest export, full-library backup/restore, local import/hashing, task progress/recovery, and library browsing implemented; manual integrity rechecks and .g4 v1 package creation/reading implemented; .g4e v1 native create/read and explicit offline integrity verification implemented; P2P integration remains pending.
- Initial platform: Windows.
- Technical direction: Tauri 2, Rust, React, TypeScript, Tailwind CSS, and SQLite.
- Planned initial transport: BitTorrent; isolated librqbit transport and corrected Game4 metainfo creation verified locally. Two physical Windows PCs on different networks passed transfer/resume/magnet over Tailscale DERP relay. Production adoption and native-network gates remain open.
- Languages: English repository content; implemented English and Brazilian Portuguese interface.
- Workflow: relevant Conventional Commits; no agent push.
- Next delivery: task cancellation; import/hashing and browsing are implemented. M0 clean-machine verification and the first hosted CI run remain outstanding.

## Immediate backlog

| ID  | Work                         | Acceptance                                                          | Status                             |
| --- | ---------------------------- | ------------------------------------------------------------------- | ---------------------------------- |
| B01 | Inventory local tools        | Known versions and prerequisites                                    | Done                               |
| B02 | Initialize desktop           | Application launches on Windows as Game4∞                           | Done                               |
| B03 | Establish interface          | Library, tasks, settings, and clear empty states                    | Done                               |
| B04 | Configure checks             | Documented reproducible formatting, types, and build                | Done                               |
| B05 | Define local model           | Reviewed SQLite migration and import states                         | Done                               |
| B06 | Implement import and hashing | Demonstrated progress, cancellation, and error handling             | In progress (cancellation pending) |
| B07 | Establish localization       | English and Portuguese resources, persistent selector, and fallback | Done                               |

## Definition of done

A task is complete when behavior meets acceptance criteria, appropriate checks pass, limitations are recorded, and affected documentation is updated. Record tests actually executed; do not infer passing results. Commit a coherent delivery without secrets, build outputs, or personal collections. Delivered user-facing features must cover both supported locales.

## History

### November 3, 2026 — Documentation foundation

- Reviewed the README and initial repository state.
- Consolidated the local library, P2P, community, and long-term preservation direction.
- Planned Guardian Mode, rescue, signed catalogs, and offline reconstruction.
- Kept cryptocurrency integrations optional, without a native token or core custody.
- Created product plan, estimated schedule, architecture, preservation, security, risks, and initial ADR.
- No application functionality was implemented.
- Commit: `6657e23`.

### November 3, 2026 — Language policy

- Translated the README, plans, risk register, progress records, and ADRs into English.
- Established English as the initial language for repository and GitHub-facing content.
- Added planned English (`en`) and Brazilian Portuguese (`pt-BR`) application support to M0 and delivery acceptance.
- Documented persistent language selection, fallback, locale parity, and stable nonlocalized data identifiers.
- No application localization has been implemented; this change establishes policy and scope.

### November 4, 2026 — Desktop foundation

- Reused the existing repository and origin; no nested repository or push.
- Generated the official Tauri 2 React/TypeScript scaffold under `apps/desktop` and configured root npm workspaces.
- Added Tailwind CSS, a local navigation shell, honest library/task empty states, and language settings.
- Added English and Brazilian Portuguese resources using i18next, saved selection, English fallback, and locale parity checks.
- Added a read-only `app_info` Rust command and checked it from the packaged WebView2 application.
- Removed the unused opener plugin, limited capabilities, configured CSP, and bound Vite to loopback.
- Added lockfiles, formatting conventions, Windows CI configuration, and development instructions.
- Installed Rust stable 1.99.1 and Visual Studio 2022 C++ Build Tools; WebView2 was already available. Initial checks used Node 24.22.1 with temporary npm 11.21.1. Node.js LTS 26.13.0 and npm 11.20.0 were then installed for normal terminal use; reopen the terminal to refresh PATH.
- Checks passed: reproducible dependency installation with `npm ci` (no reported vulnerabilities), TypeScript, 27-key locale parity, Prettier, production frontend build, Rust formatting, Cargo check, Clippy with warnings denied, and native debug build without bundling.
- Browser checks passed: navigation, language switching, saved preference after reload, layout at 1180px and 720px, and no JavaScript runtime errors.
- Native smoke test passed: embedded React page, real Rust version response, and Portuguese switching.
- Limits: no clean-machine install test, signed release, release-mode build, NSIS installer test, or hosted GitHub Actions run yet. Scaffold icons remain placeholders. No SQLite or P2P implementation.

### November 4, 2026 — SQLite and modular folders

- Added bundled SQLite through rusqlite, initialized in Tauri local application data under `database/game4.sqlite3`.
- Added transaction-managed schema version 1 for content, physical locations, collection metadata, item-file links, and import/verification tasks.
- Enabled foreign keys, WAL, FULL synchronous mode, and a bounded busy timeout; future schemas are rejected without resets.
- Added read-only database summary IPC on a blocking worker, with stable error codes and no arbitrary SQL exposure.
- Organized Rust commands, database/migrations/tests, library, and tasks; reserved security and torrent boundaries with documentation.
- Organized React components, hooks, pages, and typed services; reserved manifest-schema and shared-fixture directories.
- Added five temporary-database tests and the `rust:test` command to Windows CI.
- Checks passed: five Rust tests, Clippy across all targets with warnings denied, TypeScript, 27-key locale parity, production frontend build, and native debug build.
- Native smoke checks passed: library/task/settings navigation, Portuguese switching, Rust version response, and SQLite summary reporting schema 1 with zero initial records. The on-disk database file was confirmed in LocalAppData.
- Updated setup instructions to use `npm.cmd` on PowerShell without changing execution policy.
- Limits: file import/hashing, user-facing catalog editing, task execution/recovery, path normalization, and automated backup/export remain unimplemented. No personal database is committed. Hosted CI has not run.
- Next step: implement cancellable import and SHA-256 hashing against the new schema.

### November 4, 2026 — Isolated BitTorrent engine experiment

- Owner confirmed that a second PC is not currently available; physical two-computer validation was not performed.
- Created a standalone librqbit 9.1.0 probe with pinned dependencies, synthetic data, explicit IPv4 peers, and no discovery or automatic port mapping.
- Passed local partial transfer, session recreation with reused pieces, corrupted-piece repair, magnet metadata exchange, and SHA-256 verification.
- Passed a separate-process seeder/downloader test; another downloader process resumed 524,305 verified bytes. Independent Python SHA-256 matched.
- Reproduced the creator's exact-boundary defect: one full 262,144-byte piece generated two piece hashes. Production engine selection remains open.
- Added two fixture-safety tests, strict Clippy checks, committed sanitized reports, a manual transport-smoke workflow, and a physical-LAN procedure/result template.
- No network behavior or dependency was added to the desktop app; no public swarms, firewall changes, upstream comments, or push.
- Limits: no physical LAN/WAN, DHT, tracker, multi-file, cross-client, or performance acceptance. The probe is not a general torrent importer.
- Next P2P actions: resolve creator correctness and execute the two-PC procedure when hardware is available. Local-library development can continue independently.

### November 4, 2026 — Restore application branding

- Fixed incorrectly transcoded UTF-8 infinity characters in the interface and native startup message.
- Verified the production frontend build and the rendered accessible Game4∞ brand and both infinity symbols in Microsoft Edge.

### November 4, 2026 — Reject invalid SQLite schema versions

- Rejected negative `user_version` values before converting them to a migration index.
- Added a regression proving that the invalid version and existing records remain unchanged on refusal.
- All six database tests and Clippy across all desktop targets passed; no personal database was modified by the tests.

### November 4, 2026 — Fix torrent creation and prepare physical test

- Replaced experiment fixture generation with a streaming Game4 BEP 3 creator; the upstream creator is retained only as a defect diagnostic.
- Seven probe tests passed, covering exact/partial piece counts, deterministic encoding, cross-file hashing and trailing empty files, invalid-output side effects, and a real exact-boundary transfer.
- Independent Python decoding/hash checks and a repeated full loopback transfer/resume/repair/magnet run passed with adapter-created metainfo.
- Rejected zero timeout/duration and invalid partial thresholds before network/output side effects where applicable. Fixed the probe's UTF-8 display text.
- Preserved original evidence; added corrected-creator and independent-validation reports. Production engine selection remains pending.
- Owner confirmed a second Windows PC on another network and selected Tailscale. Installed Tailscale on PC A, without accepting DNS changes or subnet routes; authentication/connectivity and the physical transfer remain pending.
- No public torrent, router forwarding, exit node, firewall rule, or upstream patch was configured.

### November 4, 2026 — Physical two-PC overlay validation

- Connected two Windows PCs on different networks through the owner-approved Tailscale setup; pings before and after the test used DERP relay in the sao region.
- Provided a source-traceable portable test package with metadata but no payload to PC B.
- Owner-reported PC B output confirmed an intentional partial stop at 524,305 bytes, a new process reusing those bytes, and completion of all 8,388,625 bytes with matching SHA-256.
- Independent PowerShell Get-FileHash on PC B matched the expected SHA-256. Magnet metadata exchange and full download also completed with matching SHA-256.
- Added sanitized structured and narrative evidence. No personal hostnames, account information, VPN IP addresses, or private paths are committed.
- Stopped the test seeder/package service and removed the temporary peer-scoped firewall exceptions. Tailscale remains installed and connected.
- Limits: the passing topology is BitTorrent over an encrypted overlay with relay, not native direct-WAN/NAT traversal. Independent-client, discovery, multi-file transfer, and production review remain open.

### November 4, 2026 — Frontend/backend IPC contracts

- Established contract v1 for app_info and database_info, with version/capability bootstrap and a shared error envelope.
- Centralized Rust DTOs; generated TypeScript and serde examples into a private npm workspace, with drift checks in Rust tests and CI.
- Replaced unchecked frontend response casts with runtime decoders, sanitized failures, and English/Portuguese error mappings.
- Separated internal database stats from wire data and enforced JavaScript-safe numeric counts.
- Defined draft import/verify/list/task/cancellation DTOs and event semantics without registering handlers or emitting events.
- Documented pagination, selection tokens, request idempotency, task revisions, precision, compatibility, and required future migrations.
- Seven frontend contract tests and ten Rust tests passed, alongside 37-key localization parity, generated-artifact checks, TypeScript, formatting, and Clippy. Production frontend and native debug builds passed. A real WebView2 smoke test confirmed contract v1, implemented capabilities, database payload, navigation, and Portuguese switching.
- No import, mutation, task worker, or P2P feature was activated by declaring these contracts.

### November 4, 2026 — Library file and folder import

- Implemented native multi-file/folder selection, source-name review, local SHA-256 hashing, recursive enumeration, atomic SQLite import, reimport, and paginated library browsing.
- Added schema v2 with durable request deduplication, task revisions/timestamps, source-to-item mapping, and restart interruption recovery.
- Activated five IPC capabilities with request validation, runtime response decoders, and bilingual error/task/import UI. Browser preview disables native actions.
- Preserved originals and separated identical content from physical copies. Failed selections leave no partial catalog; errors and progress remain durable.
- Documented canonical path policy, reparse rejection, limits, and read-time integrity semantics in ADR 0008 and the import guide.
- Automated checks: nine frontend IPC tests, seventeen Rust tests, 64 translation keys, TypeScript, generation drift, formatting, and Clippy. Native production-asset debug build passed.
- Native Windows smoke passed under a separate test app identifier: picker cancellation, recursive folder selection/review/import (2 files, 60 bytes), single-file selection/review/import (1 file, 25 bytes), completed-task display, and Portuguese library labels. An independent read of the closed SQLite catalog confirmed schema v2, 2 items, 2 content identities, 3 physical copies, 2 successful tasks, matching SHA-256 for every location, and integrity_check=ok. The test app was closed and the normal identifier restored in the final build.
- Remaining M1 gates: cancellation, standalone integrity rechecks, richer task history/search UI, and clean-machine validation. No P2P activity is triggered.

### November 4, 2026 — Background SHA-256 responsiveness

- Confirmed that native import is accepted asynchronously and SHA-256 runs on Tauri's blocking executor, outside the WebView and without holding SQLite's mutex during reads.
- Reused one 1 MiB hashing buffer across files and throttled intermediate progress persistence to 200 ms, avoiding a synchronous SQLite commit per tiny file. Terminal progress and catalog atomicity are preserved.
- Added a deterministic concurrency regression: pause a real background import mid-hash, complete task/library/database reads on another worker, release hashing, then verify the digest across multiple buffer boundaries and final counts.
- Validation passed: 18 Rust tests, including the concurrent mid-hash read regression, Rust formatting, and Clippy with warnings denied. No new IPC/schema fields or network functionality were introduced.

### November 4, 2026 — Versioned JSON manifests

- Implemented portable manifest v1, public JSON Schema 2020-12, synthetic example, shared rejection corpus, typed Rust serializer/parser, and structural/semantic validation.
- Added schema v3 for durable portable collection UUIDs, independent of local SQLite row IDs. Repeated snapshots remain stable across restart and reimport.
- Added Library JSON export with a parented native save dialog, blocking worker, validated IPC result, and both locales. Exports omit private locations/internal identifiers and reflect stored hashes.
- Added temporary-sibling/no-clobber writes, exact decimal sizes, Unicode path preservation, unsafe-path checks, version rejection, and documented 32 MiB/100,000-file limits.
- Validation passed: 23 Rust tests, 10 IPC tests, 3 public-schema tests, 71-key locale parity, generated-contract drift checks, TypeScript, formatting, Clippy with warnings denied, and the Windows native debug build. The offline semantic-validation CLI accepted both the example and the actual native export.
- Native smoke passed with the isolated test catalog: exported a two-file item through the save dialog, observed the Portuguese success message, and independently validated the written document against the public schema, relative paths, exact sizes, and SHA-256 of both source files. Closed the test app and rebuilt with the normal application identifier.
- Manifest import/reconstruction, fresh rechecks, signatures, and archive containers remain planned. No push or network sharing is performed.

### November 4, 2026 — Save and restore the local library

- Implemented private versioned full-catalog JSON backups in Settings, preserving metadata/notes, hashes, locations, relative mappings, history, source identities, and manifest UUIDs.
- Added validated native save/load pickers, a count preview, explicit replacement confirmation, five-minute catalog-bound tokens, import exclusion, and pre-restore recovery files.
- Restore uses trusted embedded schema validation and one transaction; file health resets to unverified, unfinished historical work is interrupted, and old picker tokens are invalidated. Game bytes remain untouched.
- Added integration coverage for roundtrip/reopen, metadata/identity, invalid files, bounds, constraints, stale/expired/replayed previews, active work, no-clobber writes, recovery-file failure, injected SQL rollback, and source preservation.
- Validation passed: 29 Rust tests, 11 IPC tests, 3 manifest-schema tests, 95 translation keys, TypeScript, generated-contract drift checks, formatting, Clippy with warnings denied, and Windows native debug builds.
- Native smoke in the isolated synthetic catalog confirmed saving through the native dialog, Portuguese success feedback, loading the saved file for preview (2 items / 2 content identities / 3 copies / 2 tasks), and a disabled restore button until explicit confirmation. Closed the preview without replacing the test catalog. Independent checks verified envelope/table counts and original file hashes/sizes. Actual restoration/rollback/reopen were exercised by integration tests. Restored the normal app identifier in the final build.
- Documented private paths/notes, no game-file inclusion, format/schema compatibility, restore replacement semantics, recovery location, and retained copies in ADR 0010 and the guide.

### November 4, 2026 — Platforms, formats, and readable sizes

- Added bounded signature detection during the existing SHA-256 pass, separate filename hints, explicit evidence/unknown states, and preservation of declared platform metadata.
- Added schema v4 content-bound identification observations, Library summary groups, paginated per-file details, localized IEC sizes, exact bytes, and SHA-256 display.
- Kept manifest v1 compatible through an optional namespaced extension. Added backup v2/schema v4 writes with the eighth table, retaining v1/schema v3 restore support without invented evidence.
- Tests cover signatures, endian variants, ambiguous/truncated/conflicting names, content deduplication, mixed folders, migration, pagination, legacy backup restoration, precise totals, and localization.
- Validation passed: 36 Rust tests, 13 IPC/frontend tests, 3 public-schema tests, 112 locale keys, TypeScript, generated-contract checks, formatting, Clippy with warnings denied, and native Windows debug builds. Signature metadata was also round-tripped through full-library backup/restore.
- Native smoke in the isolated catalog imported five synthetic files (16,442 bytes) and displayed per-file details in Portuguese. Verified NES header evidence, GBA extension evidence, a ZIP renamed as GBA without a false platform, empty/ambiguous files, exact bytes, IEC size, and SHA-256. Independent closed-catalog inspection confirmed schema v4, stored evidence, unchanged source hashes/sizes, and integrity_check=ok. The normal app identifier was restored for the final build.

## Template for future entries

```text
Date — delivery
Goal:
Changes:
Checks executed and results:
Limitations / risks:
Related decisions:
Next step:
Commit: add hash when known; Git history is the definitive reference.
```

### November 13, 2026 — Manual integrity verification

Added per-item background SHA-256 rechecks of all registered content copies, atomic copy-health observations, persistent task progress, request replay protection, and English/Portuguese Library counts. Reference hashes and original bytes are preserved. Existing schema v4 and backup v2 remain compatible. See [behavior and limits](library-integrity.md) and [ADR 0012](decisions/0012-manual-integrity.md).

Validation: 42 Rust tests, 14 frontend/IPC tests, and 3 public-schema tests passed (59 total). TypeScript, 128-key locale parity, Prettier, Rust formatting, Clippy with warnings denied, contract drift checks, frontend production build, and native Tauri debug build without bundling passed. No manual GUI smoke test was performed for this increment; user catalog files were not used as test fixtures. No push.

### November 13, 2026 — Public .g4 package format and implementation

Defined the [public .g4 v1 specification](../packages/g4-format/spec-v1.md), descriptor JSON Schema, fixed layout/limits, original-byte SHA-256 rules and ZIP/ZIP64 interoperability. Added streaming Rust creation/full reading, an offline CLI, and English/Portuguese Library actions to create packages or inspect and verify them. Original hashes and files remain intact; no extraction, execution or automatic catalog import occurs. No SQLite or backup format change is needed. See [ADR 0013](decisions/0013-g4-package.md).

Synthetic ZIP32 and ZIP64 fixtures created independently with Python are accepted by Rust. A package created by the Rust CLI was independently opened with Python zipfile: exact three-entry payload, 40 bytes, valid ZIP CRC and matching SHA-256, including Unicode paths and empty content. Rust CLI readback also succeeded. No personal library was used. Physical multi-gigabyte ZIP64 performance, manual GUI smoke testing, extraction and immutable package progress/cancellation remain outside this increment.

Validation: 56 Rust tests (14 package tests), 15 frontend/IPC tests and 5 schema tests passed (76 total), plus TypeScript, 143-key locale parity, Prettier, Rust formatting, Clippy with warnings denied, generated-contract drift checks and native Tauri debug build without bundling. Final CLI creation/readback with relative paths passed, and Python independently verified its bytes. Fixture generator reproduced both public binary fixtures exactly. No push.

### November 13, 2026 — Original-file contract and Game4 Ever Manifest

The owner clarified that .g4 is the optional open lossless preservation package; .g4e means Game4 Ever Manifest, a separate P2P discovery/sharing descriptor. Originals must be shareable directly without conversion or mandatory staging copies. Encryption, DRM and central servers are not prerequisites. The earlier conceptual .g4t name is superseded.

Added the [original preservation contract](original-preservation.md), a complete local-workflow regression checking source-tree bytes/names/mtime/attributes, and the [public .g4e v1 specification](../packages/g4e-format/spec-v1.md), JSON Schema, offline Node reference validator, original/package examples with independent trackerless torrent companions and a shared malformed corpus. The .g4 v1 bytes/layout and the SQLite/backup/IPC versions remain unchanged. See [ADR 0014](decisions/0014-ever-manifest-and-originals.md).

The validator checks descriptor structure and magnet/info-hash consistency without reading referenced payloads/companions or using the network. It does not certify external content or availability. Production direct sharing, desktop .g4e handling, source mappings, acquired-metainfo binding and explicit network controls remain open P2P implementation gates. The v1 descriptor profile references v1 swarms; v2/hybrid/padding need a later profile and fixtures.

Validation: 57 Rust tests, 15 frontend/IPC tests, 5 existing format/schema tests and 7 .g4e conformance tests passed (84 total), including 21 shared malformed descriptors. TypeScript, 143-key locale parity, Prettier, Rust formatting, Clippy with warnings denied and contract drift checks passed. Both .g4e CLI examples validated offline. Python regenerated all four public examples byte-for-byte and independently decoded the companion torrents to confirm exact manifest path/size sets. No production runtime/UI code changed, no network transfer was initiated, and no personal originals were used. No push.

### November 13, 2026 — Native .g4e creation/reading and direct-original references

Implemented native Library export/open and the Rust offline `g4e` CLI. Export checks existing original files and generates a trackerless .torrent/.g4e plus magnet without making .g4 or staging payload copies. Open validates and displays reference-only summaries; explicit CLI verification binds selected metainfo, complete paths/sizes, SHA-1 pieces and raw-file SHA-256. Existing-package references additionally check embedded .g4 integrity and collection UUID. Public format v1, SQLite v4 and backup v2 remain unchanged. See [workflow and limits](ever-manifests.md) and [ADR 0015](decisions/0015-ever-create-read.md).

Native smoke used a fresh isolated app identifier/catalog: imported 3 synthetic originals (40 bytes), exported both reference files through the native dialog, reopened the resulting .g4e, and confirmed the magnet, counts and reference-only validation message. English and Brazilian Portuguese rendering were observed. Closed the test app and restored the normal executable configuration. Python checked CLI/GUI companion hashes, exact original SHA-256/lengths, no absolute source-path leakage and no .g4/staging payload creation. The independent Node validator accepted both exported descriptors; librqbit's separate parser accepted the generated torrent and produced the same magnet without starting a session.

In-app seeding/download sessions, discovery and real transfer validation remain open. Reference export must not be presented as an active upload or as proof of source availability. No personal catalog or originals were used.

Validation: 67 Rust tests (10 .g4e tests), 16 frontend/IPC tests, 5 existing format/schema tests and 7 public .g4e tests passed (95 total). The final targeted .g4e checks also passed after adding the companion-digest and trailing-empty cases. TypeScript, 161-key locale parity, formatting, Clippy with warnings denied for desktop/probe, generated-contract checks and normal Tauri debug build without bundling passed. No push.

## 2026-11-13 — Real desktop BitTorrent engine and metainfo

Integrated pinned librqbit 9.1.0 with a dedicated handle-only storage adapter and explicit native network commands. Added offline public-v1 `.torrent` creation/import, bounded preview tokens, background source SHA-256 preparation, original-file seeding without staging, new-directory downloads, runtime progress and Stop. Extended IPC v1 capabilities/errors/DTOs and strict response decoders, with bilingual Transfers UI. Added loopback real-engine transfer and independent SHA-256 tests, read-only storage refusal/release checks and IPC boundary tests. Public guide and ADR 0016 record the experimental adoption scope and deferred persistence/discovery/WAN acceptance. No push is performed.

Validation: native isolated-catalog smoke confirmed offline import of three synthetic files (Unicode path and empty file), full piece verification, active seeding and explicit Stop with listening port removed. Normal debug build restored after the isolated smoke. Automated loopback transfer independently matched SHA-256; frontend contracts and bilingual key checks passed, with Clippy warning-free. No personal catalog was used for network testing.

## 2026-11-14 — Magnets and optional DHT peer discovery

Added capability-gated native magnet generation without `.torrent` export, strict hex/base32 v1 link normalization, cancellable metadata-only acquisition and file preview before payload selection. Added explicit per-action DHT options for metadata, seed and download, with no routing persistence or network startup on launch. Empty initial peers are accepted only with DHT enabled. Metadata connections run sequentially with bounded deadlines; original read-only storage and new-file download storage remain intact. Updated both locales, IPC DTOs/decoders, current operational guide and ADR 0017.

Validation includes actual direct-peer metadata acquisition without payload files, cancellation, hostile/ambiguous magnet rejection, compatibility guards, and a real isolated UDP DHT swarm that announces a seed, resolves metadata and transfers bytes without an initial peer, matching independent SHA-256. Isolated bootstrap routing must be populated reciprocally, and loopback announcement must be explicitly enabled for the test because upstream suppresses it by default. Public DHT/WAN acceptance remains open; no personal files were published to public DHT by these tests. No push is performed.

Final validation: 75 Rust tests and 31 JavaScript contract/format tests passed; 211 translation keys match across English and Brazilian Portuguese. The DHT acceptance test waits for an independent BEP 5 UDP observation of the seed announcement before the receiving lookup, avoiding a race with upstream node requery timing. Normal Windows debug build and generated-contract checks passed.

## 2026-11-14 — Explicit tracker support

Implemented opt-in HTTP/HTTPS/UDP tracker routing for seed, download and sequential magnet metadata acquisition. Added reviewable imported tracker suggestions, bounded native URL validation/capability, selected-trackers torrent/magnet generation, exact-info-byte routing rewrites, per-session tracker counts and backward-compatible wire defaults. No default tracker or automatic imported endpoint contact is added. Updated both locales, current operational guide and ADR 0018.

Real local HTTP and BEP 15 UDP tracker tests observe seed announces, resolve metadata and complete SHA-256-matched multi-file transfers without DHT or a manual peer. A separate unselected tracker embedded in imported metainfo receives no traffic. Endpoint/identity/IPC tests cover export stability and capability guards. HTTPS uses the pinned Rust TLS transport; public HTTPS tracker/WAN acceptance remains open. No push is performed.

Validation: 78 Rust tests and 32 JavaScript IPC/format tests passed; 215 translation keys match across both locales. HTTP and UDP network tests use only local synthetic trackers and content.

## 2026-11-14 — Transfer lifecycle, upload and local peer observations

Added explicit continued seeding after download while retaining default stop-on-completion. Complete downloads are flushed/synced off-thread and their shared storage adapter is sealed against writes before being marked as seeding. Stop is independently available during other long operations, preserves byte totals and clears live observations. Added received bytes, sampled upload/download rates and local connected seeder/other-peer observations with conservative availability wording, never substituting swarm-wide totals or a fabricated piece ratio. Added an additive capability and backward-compatible IPC validators, both locales, operational documentation and ADR 0019.

The real loopback handoff test observes live peer classes and positive rates under source throttling, verifies storage refuses writes after completion, stops the original source and downloads from the new seed to a third client with matching SHA-256. The original content is unchanged. Per-peer addresses are not exposed by these aggregate DTOs. No push is performed.

Validation: 79 Rust tests and 33 JavaScript IPC/format tests passed, including the three-session upload handoff and old default stop-on-completion coverage. Connected availability remains explicitly scoped to observed peers.

## 2026-11-14 — Pause, reverified resume and durable transfer state

Implemented a separate private SQLite transfer store v1, write-before-unpause lifecycle records and one-second progress checkpoints. Interrupted work reopens Paused, without sockets/discovery. Added Pause, Resume and record-only Forget controls, cancellation and generation guards, stable assigned ports, last-known cumulative counters and safe reopening of owned partial files without truncation. Original seeding sources refuse changed bytes and remain read-only. Ownership sidecars live beside payload roots and do not become torrent payload. Invalid/future state is preserved with transfers unavailable and Library still usable. Catalog SQLite v4/public formats/backup v2 are unchanged.

Tests cover cold partial recovery, corruption repair with SHA-256 match and no duplicate directory, idempotent request replay, stable source port, changed originals, unsafe owner/oversized files, concurrent/cancelled resume and record-only removal. The separate desktop-API process harness intentionally exits without cleanup after 1,048,593 verified bytes, then reopens the same record Paused, rechecks/reuses those pieces and completes 8,388,625 bytes. Independent SHA-256 is ca1d758393b634d51aea5298b89b6d598403e41d5e72176e8114fc434b4dfe29. This does not certify power-loss/WAN behavior. English guide, ADR 0020 and bilingual controls are included; no push is performed.

Final validation: 85 Rust tests and 34 JavaScript IPC/format tests passed. The final separate-checkpoint store also passed the abrupt-process recovery harness and independent SHA-256 comparison. Checkpoint write failure closes sharing, early v1 inline snapshots migrate without networking, and 233 English/Portuguese keys remain aligned.

## 2026-11-14 — Modern preservation dashboard

Added Overview as the desktop landing page, with a responsive dark archive identity, CSS package artwork, collection cards, indexed-size and saved-session metrics, recent task activity and working navigation shortcuts. The dashboard reads existing IPC services independently every five seconds; errors do not replace unrelated successful sections. Browser preview and unavailable data use explicit states rather than fabricated totals. Library metrics are scoped to the first 50 items and do not claim deduplicated disk usage; activity shows five of the latest 50 tasks. No network transfer starts from a dashboard read. English and Brazilian Portuguese remain aligned (267 keys).

Validation: frontend type checking, 34 JavaScript IPC/format tests, formatting and the Windows debug desktop build passed. Native visual inspection confirmed the empty-catalog dashboard and navigation to Library and back. The populated-card layout and narrower responsive breakpoints were implemented but were not manually exercised with fixture catalog data.

## 2026-11-14 — Library and transfer workspace screens

Extended the dashboard identity to Library and Transfers with responsive archive banners, scoped metrics and deliberate empty states. Library now offers title search and platform filters over loaded items, grid/list layouts, generated typographic archive covers, expandable platform/format/integrity observations and a grouped package reader. Existing import, verification and export actions remain connected to native services. Transfers prioritizes saved sessions, adds state filters and compact connection details, separates download/share-original composers and groups discovery settings without starting any network operation. Active controls and metadata lookup cancellation remain available while preparing work. DOM order follows the visual session-first layout for keyboard navigation. Both locales now have 299 aligned keys.

Validation: type checking, all 34 JavaScript IPC/format tests and the Windows debug build passed. Native visual inspection in Portuguese confirmed both empty workspaces and switching to the share-original composer. Populated catalog/session rows and narrow responsive breakpoints were not manually exercised in this pass. No push was performed.

## 2026-11-14 — Visible transfer speeds, progress and connected peers

Added a responsive per-session measurements panel with sampled download/upload rates in IEC bytes per second, connected peers, seeders and leechers/other peers. Verified progress uses exact integer arithmetic and floors to one decimal place so incomplete data never rounds to 100%. Payload received (including retransmissions) and uploaded totals remain separate from verified data. Inactive sessions, missing telemetry and failed polls show unavailable live observations rather than stale rates or fabricated zeros; saved totals remain labeled as last known after polling failure. Availability details preserve the connected-session scope and do not claim swarm totals. Both locales now have 309 aligned keys.

Validation: type checking, formatting and 36 JavaScript IPC/format tests passed, including large-integer/zero/complete progress and inactive/stale telemetry cases. The Windows debug build passed after closing the previously running executable normally. No new live cross-computer transfer or populated-panel visual test was performed for this presentation change. Native engine and IPC contracts are unchanged.

## 2026-11-14 — Per-transfer upload and download limits

Added payload limits before starting a seed/download and live edits for saved sessions. Independent directions use exact integer bytes/s natively and KiB/s in the bilingual UI; 0 is unlimited and the minimum nonzero value is 16 KiB/s to accommodate the engine's block/burst capacity. Settings are written atomically before live changes and reapplied on resume, with legacy private-v1 defaults and rollback on write failure. IPC v1 now advertises 33 commands including `transfer_set_limits`; nonzero caps require that capability before starting network work. No global cap, protocol overhead ceiling or instantaneous in-flight request cancellation is claimed. Reference creation and magnet metadata discovery remain separate.

Validation includes independent real-loopback upload/download throttling with completion timing and SHA-256 integrity, live configuration changes, persisted restart/resume, unlimited reset, conflicting request replay, legacy unlimited defaults and write-failure rollback. UI/IPC tests cover exact units, bounds and older-backend capability rejection. Documentation records the per-session scope and burst behavior. No push is performed.

Final checks: 89 Rust tests and 38 JavaScript IPC/format tests passed, along with strict Clippy (all targets), generated contract verification, locale alignment (318 keys), formatting and the Windows debug desktop build. The extended legacy/replay/rollback test passed after its final assertions were added. No manual populated-session UI inspection or WAN bandwidth certification was performed.

## 2026-11-14 — Preservation mission and intuitive download/share journeys

Reconciled the project owner's renewed Game For Ever plan with the implemented app in an English vision/experience baseline, linked from README, product plan, roadmap and documentation index. Current direct P2P, local preservation and controls are distinguished from future community discovery/publication, .g4lib, historical metadata, Guardians and optional IPFS/Bitcoin-related research. No community service, mandatory account, token or background sharing was added.

Home now leads with Download games, Share my collection and a clearly marked future-community overview. Separate download/share/monitoring routes avoid burying primary actions under session history and configuration. Library Share carries the selected collection, including items outside the first source-selection page. Sharing exposes a copyable download link after explicit startup, retaining selected tracker references; a manual-copy fallback is provided. A .g4e can supply a link without starting network lookup. Editing a link invalidates its old preview, and selecting a torrent clears the previous link. Technical identifiers, ports, trackers, reference tools and speed settings are optional disclosures, while network consent remains near the action. Original sources, integrity checking, pause/resume and bandwidth behavior are unchanged.

Integrated all four owner-supplied original PNG logos without modifying them; the dark app uses the transparent white version and README selects a suitable background variant. Added an artwork inventory. Both locales contain 373 aligned keys with user-focused verbs and plain connection labels rather than unexplained engine vocabulary.

Validation: type checking, formatting and 40 JavaScript IPC/format tests passed, including distinct intent routes and preservation of the selected sharing source. The Windows debug desktop build passed with the original logo bundled. Native visual inspection confirmed Home, Download, empty Share and the future-community page in Portuguese; English community/navigation and language switching were also verified, then Portuguese restored. Populated sharing/copy behavior, a new P2P transfer and narrow breakpoints were not manually exercised during this UX pass. No push was performed.

## 2026-11-14 — Configurable storage folders

Added Settings controls for independent Downloads, Preservation files and Library backups native-dialog defaults. Selection uses native pickers, validates existing absolute writable directories without links/junctions and persists private bounded JSON v1 with synced same-directory atomic replacement. Cancel preserves settings; Clear default keeps folders/files; missing drives are reported without fallback or recreation. New downloads, .g4/.g4e/.torrent/JSON exports and catalog backups use their configured starting locations while retaining explicit destination confirmation. Existing Library originals, transfer ownership roots, sidecars and fixed internal databases are unchanged. Preferences are excluded from public formats and backup v2.

IPC v1 now has 36 implemented commands, with typed storage-kind-only mutation requests, private read-only path observations and a storage error. English and Brazilian Portuguese have 393 aligned keys. Native coverage includes restart/reset/content retention, disconnected folders, malformed/future/bounded preferences, failed writes, Windows junction refusal and locked-file atomic-save rollback. IPC checks cover cancellation, unsafe supplied paths and older-backend capability rejection.

Validation: 96 Rust tests and 42 JavaScript IPC/format tests passed, plus generated contract verification, strict Clippy for all targets, locale alignment and the Windows debug desktop build. Native UI inspection confirmed the Portuguese storage controls and opening/cancelling the directory picker without changing defaults. Actual persistence/reset and missing/unsafe/locked paths were exercised in isolated native tests; no personal storage destination was selected during visual QA.

## 2026-11-14 — Windows preservation-file associations

Added .g4/.g4e Viewer file associations to the NSIS bundle, with distinct type names and existing app icon. Integrated the official Tauri single-instance plugin so native launch arguments queue in the main process and subsequent launches focus it. A bounded native inbox resolves relative filenames, preserves spaces/Unicode, filters supported extensions, rejects URL-like/oversized arguments, coalesces queued duplicates and exposes only opaque tokens to IPC. Shared inspection helpers serve both associations and existing native pickers.

The bilingual global preview validates .g4 payload integrity or .g4e descriptor information off-thread without extracting, executing, importing or starting P2P. Closing a preview advances the queue. A .g4e link can explicitly prefill Download while discovery stays opt-in. Added 2 IPC capabilities (38 implemented commands), 396 aligned locale keys and format/queue/token/capability tests. Public formats, catalog/transfer schemas and storage preferences are unchanged. No push is performed.

Validation: 101 Rust tests, 44 JavaScript IPC/format tests, strict all-target Clippy, generated contract verification, locale alignment and the x64 debug NSIS installer build passed. Local process/UI acceptance verified cold .g4 inspection, Unicode/spaces/ampersand paths, warm .g4e forwarding with only one lasting process, FIFO preview handling and explicit download-link prefill without starting lookup. Source hashes remained unchanged; generated registration/removal macros quote the executable and filename. A final presentation adjustment scrolls new previews into view. The installer was not installed, so installed Explorer/default-app/update/uninstall behavior remains an acceptance gate. No registry changes or push were performed.

## 2026-11-14 — Empty states, actionable errors and in-app notifications

Added an accessible bilingual notification center with an unread badge, three visible unread messages and a 30-message session history preserved across navigation. Native success acknowledgements cover collection/verification requests, file/reference exports, links, transfers, backups and storage/speed controls. A read-only global observer follows known task/transfer transitions without replaying historic results, announcing progress ticks or opening network sessions. Messages keep translation keys, avoid raw error output and require no OS alert permissions. No backend contract or schema change was made.

Added reusable empty/error feedback and contextual read retries. Library/Activity failures load independently, list failures stop showing endless loading and failed reads no longer masquerade as an empty collection/source list. Last-loaded rows remain visible after transient errors; action errors are distinct from polling failures and are explicitly dismissible. Browser preview, no activity, empty collections and filtered transfer states explain the next action. Unit coverage exercises event deduplication/bounds, historic baselines, status transitions and seeding completion semantics.

Validation: type checking, formatting, 47 JavaScript IPC/format tests and alignment of 438 locale keys passed. The normal Windows debug desktop build passed. An isolated QA identifier/profile confirmed empty notification history, the empty Library action and a synthetic local import with distinct start/completion notices and an unread badge. The installed app's catalog/session was left untouched. Live P2P completion/failure notifications and forced read failures were not manually exercised; observer transitions are covered by unit tests. No push was performed.

## 2026-11-14 — Historical archive foundation and public community catalogs

Added SQLite schema 5 with independent historical games/platform releases, commercial editions, textual technical revisions, explicit update/DLC base compatibility, acyclic content dependencies, preservation-file links, P2P references, attributed cached artwork, metadata sources, personal collections/preferences and imported catalog provenance. Existing physical originals/imports/tasks remain unchanged. Registered one typed, capability-advertised archive request command executing blocking work off the UI thread.

Added public .g4lib v1 specification, JSON Schema and synthetic metadata-only interoperability fixture. Native validation uses an isolated candidate database and enforces relationship integrity, duplicate identities, source normalization and private-reference exclusion. Explicit reviewed imports namespace snapshots, are idempotent for identical documents and never overwrite existing local metadata or start a transfer. Exported/imported authorization declarations are cleared. Private backup v3 includes archive/artwork/preferences while retaining v1/v2 readers and atomic replacement/recovery; restoring older backups intentionally restores no historical archive.

Validation: 110 Rust tests passed, including cross-edition update/DLC rejection, dependency cycles, non-destructive catalog roundtrip, malformed/public-private input rejection, backup/artwork/local-link preservation, database reopening and a bounded query over 10,000 synthetic games. Strict all-target Clippy passed; generated Rust/TypeScript artifacts and public fixture compatibility were checked. The frontend integration and native visual acceptance are recorded in the next entry. No push was performed.

## 2026-11-14 — Archive compatibility and snapshot hardening

Follow-up validation now rejects dependencies across commercial editions, makes repeated .g4e association idempotent for identical content hashes/references, and refuses oversized table snapshots instead of silently truncating backup/export rows. The public .g4lib compatibility profile documents same-edition dependencies explicitly.

Validation: 111 native tests passed, including repeated-manifest association and cross-edition dependency rejection; strict all-target Clippy passed. The normal Windows debug desktop build passed after isolated QA and was restored to the production identifier. No push was performed.

## 2026-11-14 — Visual Game Archive and local catalog workflows

Integrated Game Archive, Platforms, Collections, Preservation and the delivered local Community catalog browser into the existing launcher navigation. Cards use authorized local artwork or branded fallbacks, grid/compact/list preferences, server-paged search and release/region/genre/language/edition/year filters. Game details separates Overview, Versions, Updates, DLCs, Files, Preservation and Community; staged forms create/edit records and artwork attribution, associate existing imports without copying originals, attach .g4e manifests and prepare explicitly reviewed downloads. Favorites, saved metadata-only titles, recent visits and personal collections persist in SQLite. No online community service or decorative Join/Play action was added.

The existing transfer metrics component is reused for saved sessions matching catalog info hashes; its read-only observer cannot start discovery or sharing. Unknown availability stays unknown without a matching observation, and integrity badges do not claim preservation for unchecked files. UI response validation, 20-entry query cache, viewport-triggered raster fetches, bounded image retention, keyboard tabs/dialogs, reduced-motion support and bilingual feedback are integrated. Updated current product/experience documents to distinguish the delivered offline archive from future signed catalogs, subscriptions and remote collaboration.

Validation: 53 JavaScript IPC/format tests, type checking, formatting, 656 aligned locale keys, generated contract verification, strict all-target Clippy and the normal Windows debug desktop build passed. All 111 native tests passed. Isolated native UI acceptance confirmed empty archive/history, .g4lib selection/review/import, real catalog counts, fallback game covers, detail/update compatibility display, metadata editing form, two-step metadata-only game creation with automatic detail opening, persistence after restart and English/Portuguese navigation. Visual issues found in button spacing/dialog positioning were corrected and rechecked. No installed catalog was changed; the QA profile remains separate.

Scope boundaries: raster attachment/cache/backup and original-file links have native automated coverage; every native picker and responsive breakpoint was not manually exercised. The new catalog-to-transfer match is unit-tested; its live peer display was not manually tested against a new network transfer during this pass. Catalog signatures/subscriptions, editorial snapshot merging, multi-base compatibility, video/document attachments, global swarm statistics and archival verification snapshots remain follow-up work. Public original files, .g4/.g4e layouts and private transfer state are unchanged. No push was performed.

## 2026-11-14 — G4EP step 1: isolated architecture and service ports

Inspected the existing Tauri worker/IPC layer, file selection/import and streamed hashing, SQLite catalog schema 5 and backup v3, librqbit 9.1.0 transfer/discovery/recovery, React progress/errors/settings and .g4/.g4e/.g4lib formats. Added a standalone internal `g4ep` module with object-safe Encoder, Decoder, StorageNode, PeerDiscovery, FragmentTransfer, IntegrityVerification and RepairManager ports. Blocking codecs/local storage are separate from async network streams; contexts carry cancellation/deadlines, typed SHA-256 identities distinguish original objects/shards from torrent info hashes and codec IDs remain internal registration keys.

Added checked profile/layout arithmetic, structural manifest geometry and explicit bounded private enrollment; no codec, database migration, public serializer, IPC command, UI action, listener, storage constructor or repair worker is installed. The architecture and ADR 0021 document reuse/adapter boundaries, an independent future private state store, format compatibility, quotas/staging/authentication requirements and private two-machine acceptance. Proposed 8+4 / 1 MiB geometry is distinguished from a codec implementation and from host-failure tolerance. Existing .g4e advisory extensions are not reinterpreted as mandatory reconstruction metadata.

Validation: 117 Rust tests passed (six new contract/planning tests), strict all-target Clippy, Rust/Prettier formatting, generated IPC artifact verification and diff checks passed. Tests cover malformed/bounded profiles, 700 MB versus 700 MiB arithmetic without a large allocation, duplicate/missing/out-of-range shards, private peer enrollment/public address rejection, cancellation/deadline behavior and trait-object usability for all seven ports. No ~700 MB file, real node, repair or multi-computer G4EP transfer was executed; those are later acceptance gates. No dependency, current format, frontend contract, original file or existing database was changed. No push was performed.

## 2026-11-14 — G4EP step 2: streaming Reed–Solomon fragments

Implemented the existing Encoder port using pinned reed-solomon-simd 3.1.0, explicit Leopard GF(2^16) high-rate RS(8,4), one reusable shard buffer and one resettable library workspace. Each configurable group emits eight systematic data shards and four recovery shards with coordinates, exact length and SHA-256; final padding is zero and whole-object hashing excludes padding. The initial 8 MiB source group yields twelve independent 1 MiB fragments. The earlier GF(256) proposal was not an implemented format and has a distinct identity from the delivered codec. Added a fixed compatibility digest vector, scalar/default-engine parity comparison and third-party notices.

Added an offline encode/verify/verify-fragment CLI and new-directory-only local sink with source read protections, synced temporary fragment writes, no-clobber publication, bounded experimental metadata and an incomplete marker. Failed/cancelled output is not accepted as complete. The inventory is explicitly experimental and is not a new .g4e/.g4lib interpretation, published protocol schema, StorageNode or repair runtime. Existing catalogs, databases, frontend and network behavior are unchanged. Payload/inventory files are ignored by Git.

Validation: all 125 Rust tests passed, including the 495 possible eight-of-twelve shard selections on non-linear deterministic data, seven-shard rejection, multi-group/odd/empty/exact lengths, padding/order/hashes, short/interrupted/bounded reads, failed/ignored sinks, cancellation/deadlines, source retention, destination refusal, corruption and hostile inventories. Strict all-target Clippy, Rust/Prettier formatting, generated IPC artifact verification and diff checks passed.

A real synthetic authorized 700,000,000-byte input encoded to 84 groups and 1,008 fragments totaling 1,056,964,608 bytes before metadata. Independent Python and native SHA-256 agreed; the original hash remained unchanged. A separate native pass reread and verified every stored fragment. Observed debug-run encoding time was 73.3315908 s with 29,462,528 peak working-set bytes; verification took 22.2292674 s with 6,156,288 peak working-set bytes. These are one-run development-host observations, not performance guarantees. Machine-readable evidence contains no source paths or payloads. No whole-file decoder service, automatic reconstruction, remote node, DHT announcement or multi-computer G4EP transfer was executed. No push was performed.

## 2026-11-14 — G4EP step 3: reconstruction manifests

Implemented compatible `.g4e` v2 dispatch, complete codec/group/fragment metadata, private discovery references, deterministic metadata hashing and optional Ed25519 signing with explicit origin pinning. Added offline native and Node validation, shared signed/unsigned fixtures and 31 invalid cases, local export/inspection/fragment checks, and bilingual desktop previews that distinguish valid signatures from trusted publishers. No game bytes or absolute source paths enter the descriptor.

Validation: 131 native tests, frontend checks, contract generation consistency and Clippy; the retained 700,000,000-byte synthetic fixture exported 84 groups/1,008 fragments, validated in Rust and Node, and passed all fragment hashes. Reconstruction/network repair is not claimed by this milestone. See [specification](../packages/g4e-format/spec-v2.md) and [evidence](evaluations/g4ep-manifest-2026-11-14.json).

## Guardian storage — November 14, 2026

G4EP step 4 adds opt-in fragment admission, atomic storage, persistent inventory, quotas, directional limits and bilingual desktop controls. Participation reopens disabled. A real local two-store fragment copy passed integrity checks; authenticated network transport remains a separate milestone. See [implementation and limits](g4ep/guardian-node.md).

Guardian acceptance: 141 Rust and 60 JavaScript tests passed, contracts verified, strict Clippy passed, and the native Windows settings layout was reviewed in an isolated app instance. Real 1 MiB fragment import/export at 256 KiB/s took approximately 4.08/4.17 seconds, with matching SHA-256 and disabled participation after restart. See [recorded evidence](evaluations/guardian-node-2026-11-14.json).
