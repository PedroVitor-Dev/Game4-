# Project management and records

## Status on November 13, 2026

- Name: Game4∞.
- Stage: desktop foundation, SQLite v4, platform/format identification and exact-size browsing, versioned manifest export, full-library backup/restore, local import/hashing, task progress/recovery, and library browsing implemented; manual integrity rechecks implemented; P2P integration remains pending.
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
