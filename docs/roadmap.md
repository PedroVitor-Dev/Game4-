# Schedule and milestones

Baseline: November 3, 2026. Initial estimate: 16 active weeks for a P2P alpha, followed by a 4-week contingency. Assumes one main developer with regular availability and project-owner review. The estimate has not been validated against historical delivery velocity.

Weeks start when implementation actually begins. Holidays, availability, incompatibilities, and scope changes may shift dates. Review estimates after each milestone; acceptance criteria take precedence over dates.

| Milestone               | Weeks | Delivery                                                             | Exit criteria                                                                             |
| ----------------------- | ----- | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| M0 — Foundation         | 1–2   | Tauri, React, TypeScript, Rust, CI, base interface, and localization | Windows build and launch; basic automated checks; persistent English/Portuguese selection |
| M1 — Library            | 3–5   | Import, SQLite, hashes, and browsing                                 | Persistence after restart; cancellation and problematic files handled                     |
| M2 — Local preservation | 6–7   | Manifest v0, export, and verification                                | Catalog reconstruction and corruption detection demonstrated                              |
| M3 — P2P prototype      | 8–9   | BitTorrent engine experiment                                         | Transfer between two installations; license and integration assessed                      |
| M4 — Transfers          | 10–12 | Queue, torrents, magnets, limits, and resume                         | State survives restart; final integrity verified                                          |
| M5 — Windows alpha      | 13–16 | Integration, tests, installer, and documentation                     | Clean installation, restoration, security review, and both locales verified               |
| Contingency             | 17–20 | Fixes and compatibility                                              | Use for observed risks, not additional features                                           |

## Current milestone status — November 4, 2026

M0 foundation code is implemented and locally verified through a Windows native debug build and WebView2 smoke test. English/Portuguese switching and persistence work. The CI workflow is configured but has not run on GitHub. A clean-machine build and installation check remains required before declaring the full M0 exit criterion met. Later milestones remain planned; no delivery-velocity claim is made from this initial setup.

## Dependencies and alternatives

- M0 requires available Windows tools and SDKs. Localization is part of M0; review its effect on estimates when implementation starts.
- M1 defines the model used by M2. M3 may affect integration, but the catalog must remain independent of the engine.
- M3 is a decision gate: if no engine meets requirements, release a local-library version first and replan P2P.
- `.g4` and `.g4t` require specifications and fixtures before implementation. They do not block the P2P prototype; use `.torrent` files and JSON manifests first.
- Alpha does not imply stable formats. Document migrations and limitations.

## Later horizon without committed dates

| Phase       | Scope                                             | Prerequisite                                           |
| ----------- | ------------------------------------------------- | ------------------------------------------------------ |
| Communities | Signed catalogs, history, and mirroring           | Usable alpha and validated manifest format             |
| Guardians   | Quotas, scheduling, health, and rescue            | Reliable availability observation and sharing controls |
| Long term   | Recovery, offline kits, IPFS, and timestamps      | Restoration tests and compatibility policy             |
| Funding     | Donations, sponsorship, and paid-storage research | Demonstrated need, costs, and responsibilities         |

## Cadence

Plan one small increment at a time. On completion, validate it, update the log, record new risks, and commit. At each milestone, compare estimates with observed effort, review the backlog, and present the next delivery. Publication and push remain under the project owner's control.
