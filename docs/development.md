# Development setup

## Repository

Use the existing Git repository and its configured origin. Do not initialize a nested repository under `apps/desktop`. Workspaces share the root `package-lock.json`; the Rust application tracks its own `Cargo.lock`.

## Windows prerequisites

- Node.js 24 or newer with npm 11 or newer (Node 24 is used in CI).
- Rust stable with the MSVC target, Cargo, rustfmt, and Clippy.
- Microsoft Visual Studio 2022 Build Tools with Desktop development with C++, including the Windows SDK.
- Microsoft Edge WebView2 Runtime.

Follow the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Restart the terminal after tool installation so PATH changes are available. No mobile or non-Windows development target is configured yet.

## Commands from the repository root

```powershell
npm.cmd ci
npm.cmd run desktop:dev
```

On Windows PowerShell, use `npm.cmd` if the execution policy blocks `npm.ps1`; changing the policy is unnecessary. Commands below use the generic `npm` spelling; `npm.cmd` works for each on Windows.

| Command                                | Purpose                                                     |
| -------------------------------------- | ----------------------------------------------------------- |
| `npm run dev`                          | Browser-only frontend preview on port 1420; no Rust backend |
| `npm run desktop:dev`                  | Native Tauri development window with Rust                   |
| `npm run build`                        | TypeScript check and production frontend build              |
| `npm run check`                        | TypeScript, locale parity, and formatting checks            |
| `npm run format`                       | Format supported source and documentation files             |
| `npm run rust:fmt`                     | Check Rust formatting                                       |
| `npm run rust:check`                   | Check Rust with the committed lockfile                      |
| `npm run desktop:build -- --no-bundle` | Build the native executable without an installer            |
| `npm run desktop:build`                | Build an unsigned NSIS installer                            |

The development server binds to the loopback interface and fails if port 1420 is occupied. The application uses a restrictive production CSP and no filesystem, shell, or external opener plugin. Custom Rust commands provide read-only application-version and database-summary queries. SQLite is owned by the Rust core and initialized in local application data, not exposed through a generic SQL plugin.

Run `npm.cmd run rust:test` for the database persistence and migration tests. See [SQLite storage](database.md) for schema and backup limitations.

## Localization

Edit `apps/desktop/src/locales/en.json` and `pt-BR.json` together. Locale checks validate key and interpolation parity. English is the default and fallback; selection is saved locally. Browser preview explicitly reports that the desktop backend is unavailable by design. See [language policy](localization.md).

## Experimental torrent tool

Use `npm.cmd run torrent:build`, `npm.cmd run torrent:test`, and `npm.cmd run torrent:check` for the separate Rust probe. See [the test procedure](torrent-validation.md). It is not linked into the desktop and does not make M3 complete. The optional GitHub transport workflow requires manual dispatch after push.

## Current limits

This is the desktop foundation, not a library or transfer implementation. SQLite startup persistence is implemented. No game import, network sharing, automatic updater, or signed distribution is included. Generated scaffold icons are placeholders pending visual identity work. CI configuration is committed but is only executed by GitHub after the owner pushes the changes.
