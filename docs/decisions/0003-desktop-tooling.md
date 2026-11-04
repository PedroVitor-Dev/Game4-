# ADR 0003 — Desktop tooling and localization foundation

Date: November 4, 2026. Status: accepted.

## Context

The project needs a runnable Windows foundation that follows the established stack and bilingual product policy without implementing library or network features prematurely.

## Decision

Use the official Tauri 2 React/TypeScript scaffold under `apps/desktop`, Vite, and Tailwind CSS. Use npm workspaces with a root package lock and a committed application Cargo lock. Keep the existing Git repository and remote.

Use i18next and react-i18next for English and Brazilian Portuguese, with explicit saved selection and English fallback. Use a locale parity script and TypeScript checks. Use Prettier for supported repository files and rustfmt / Clippy for Rust.

Keep only core capabilities and a read-only Rust version command. No filesystem, shell, or opener plugin is enabled. Use loopback for development and a CSP for embedded content. Generate NSIS installers only when distribution is tested; initial verification builds the executable without bundling.

## Alternatives

Separate repositories or package managers per component would add overhead at this stage. A custom translation implementation would duplicate mature fallback and interpolation facilities. The generated opener plugin is unnecessary for the current interface.

## Consequences

Root commands provide one entry point for development and checks. Windows requires Rust MSVC, C++ Build Tools, and WebView2. Lockfiles must be updated deliberately. CI remains a proposed validation path until the owner pushes and observes a successful run. Generated icons are temporary and must be replaced before release.

## Validation evidence

Local frontend checks and build, Rust check / formatting / Clippy, native Windows debug build, browser navigation and locale persistence, and a WebView2 smoke test of the real Rust command passed on November 4, 2026. Release packaging and clean-machine validation remain outstanding.
