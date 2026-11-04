# Project management and records

## Status on November 4, 2026

- Name: Game4∞.
- Stage: desktop foundation and SQLite startup persistence implemented; importing, hashing, and P2P remain pending.
- Initial platform: Windows.
- Technical direction: Tauri 2, Rust, React, TypeScript, Tailwind CSS, and SQLite.
- Planned initial transport: BitTorrent; isolated librqbit transport and corrected Game4 metainfo creation verified locally. Two physical Windows PCs on different networks passed transfer/resume/magnet over Tailscale DERP relay. Production adoption and native-network gates remain open.
- Languages: English repository content; implemented English and Brazilian Portuguese interface.
- Workflow: relevant Conventional Commits; no agent push.
- Next delivery: implement local-library import and hashing using the SQLite model and task vocabulary. M0 clean-machine verification and the first hosted CI run remain outstanding.

## Immediate backlog

| ID  | Work                         | Acceptance                                                          | Status  |
| --- | ---------------------------- | ------------------------------------------------------------------- | ------- |
| B01 | Inventory local tools        | Known versions and prerequisites                                    | Done    |
| B02 | Initialize desktop           | Application launches on Windows as Game4∞                           | Done    |
| B03 | Establish interface          | Library, tasks, settings, and clear empty states                    | Done    |
| B04 | Configure checks             | Documented reproducible formatting, types, and build                | Done    |
| B05 | Define local model           | Reviewed SQLite migration and import states                         | Done    |
| B06 | Implement import and hashing | Demonstrated progress, cancellation, and error handling             | Pending |
| B07 | Establish localization       | English and Portuguese resources, persistent selector, and fallback | Done    |

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
