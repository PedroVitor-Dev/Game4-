# Project management and records

## Status on November 4, 2026

- Name: Game4∞.
- Stage: desktop foundation implemented; local library and P2P remain pending.
- Initial platform: Windows.
- Technical direction: Tauri 2, Rust, React, TypeScript, Tailwind CSS, and SQLite.
- Planned initial transport: BitTorrent; engine still under evaluation.
- Languages: English repository content; implemented English and Brazilian Portuguese interface.
- Workflow: relevant Conventional Commits; no agent push.
- Next delivery: define the SQLite model and import task states, then implement the local library. M0 clean-machine verification and the first hosted CI run remain outstanding.

## Immediate backlog

| ID  | Work                         | Acceptance                                                          | Status  |
| --- | ---------------------------- | ------------------------------------------------------------------- | ------- |
| B01 | Inventory local tools        | Known versions and prerequisites                                    | Done    |
| B02 | Initialize desktop           | Application launches on Windows as Game4∞                           | Done    |
| B03 | Establish interface          | Library, tasks, settings, and clear empty states                    | Done    |
| B04 | Configure checks             | Documented reproducible formatting, types, and build                | Done    |
| B05 | Define local model           | Reviewed SQLite migration and import states                         | Pending |
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
