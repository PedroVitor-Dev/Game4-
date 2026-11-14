# Product plan

Current product/experience baseline (November 14, 2026): [vision and user experience](vision-and-experience.md). It reconciles the renewed community preservation plan with implemented local Library, direct P2P, recovery and speed controls; older dated scope notes below are historical. Community catalog/discovery and .g4lib remain planned.

## Vision and audiences

Game4∞ should help people and communities maintain verifiable, recoverable, usable collections even without services operated by the project's creators. The initial platform is Windows, with local functionality and no mandatory account.

| Audience  | Need                                                      |
| --------- | --------------------------------------------------------- |
| Collector | Catalog files and detect changes or damage                |
| Curator   | Document provenance, versions, and execution requirements |
| Community | Share collections and maintain independent copies         |
| Guardian  | Offer storage and bandwidth within explicit limits        |

## First delivery: local library

- Import files and folders with progress and cancellation.
- Record size, location, platform, version, and SHA-256.
- Detect exact duplicates without automatic deletion.
- Browse and edit local metadata stored in SQLite.
- Recheck files and distinguish intact, changed, missing, and unverified states.
- Export and import a versioned manifest offline.
- Explain when a file changes during reading.
- Support English and Brazilian Portuguese through shared localization infrastructure.

Acceptance: import a test collection, restart the app, verify integrity, export a manifest, and reconstruct the catalog in a clean installation. Errors must not silently stop the entire operation. Language selection must persist, and both locales must cover the delivered interface.

## P2P alpha

Add torrents and magnet links, torrent creation, queues, progress, speed limits, pause, resume, persistence, and piece verification. Sharing requires explicit action and an explanation of network exposure.

Acceptance: two installations transfer authorized content, resume after restart, and verify final files. The application remains useful when no peers are available.

## Later differentiators

1. Guardian Mode with collection selection, disk quotas, bandwidth limits, and schedules.
2. Rescue of content with low observed availability.
3. Signed catalogs and version history.
4. Execution documentation and offline reconstruction kits.
5. Redundancy, recovery, and scheduled verification.
6. Timestamp proofs and optional collection funding.

## Outside the initial scope

A native token, NFTs, financial custody, automatic seeding rewards, a marketplace, an embedded emulator, automatic execution, simultaneous multi-OS support, and complex proprietary formats.

## Quality indicators

Measure restoration success, corruption detection, import failures, resume reliability, and memory usage on test collections. Record collection size, hardware, and tool versions for comparison. Define numerical performance targets after initial measurements. No personal telemetry by default.

## Direct-sharing acceptance constraint — November 13, 2026

The P2P alpha must share originals from their current locations without conversion, mandatory payload duplication or creating .g4 packages. Source mappings remain private and read-only. .g4 creation is optional; .g4e (Game4 Ever Manifest) describes discovery/sharing references for either representation. No mandatory encryption, DRM, account or central service may gate core preservation/sharing. The public .g4e specification and offline validator are available; production desktop integration remains an open P2P gate.
