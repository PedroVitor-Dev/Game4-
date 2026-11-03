# Project management and records

## Status on November 3, 2026

- Name: Game4∞.
- Stage: planning and documentation; no implemented application.
- Initial platform: Windows.
- Technical direction: Tauri 2, Rust, React, TypeScript, Tailwind CSS, and SQLite.
- Planned initial transport: BitTorrent; engine still under evaluation.
- Languages: English repository content; planned English and Brazilian Portuguese application.
- Workflow: relevant Conventional Commits; no agent push.
- Next delivery: M0, a runnable and verifiable desktop foundation.

## Immediate backlog

| ID | Work | Acceptance | Status |
|---|---|---|---|
| B01 | Inventory local tools | Known versions and prerequisites | Pending |
| B02 | Initialize desktop | Application launches on Windows as Game4∞ | Pending |
| B03 | Establish interface | Library, tasks, settings, and clear empty states | Pending |
| B04 | Configure checks | Documented reproducible formatting, types, and build | Pending |
| B05 | Define local model | Reviewed SQLite migration and import states | Pending |
| B06 | Implement import and hashing | Demonstrated progress, cancellation, and error handling | Pending |
| B07 | Establish localization | English and Portuguese resources, persistent selector, and fallback | Pending |

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
