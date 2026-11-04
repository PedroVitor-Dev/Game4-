# Game4∞ desktop

Tauri 2 shell with a Rust core and React / TypeScript frontend, built with Vite and Tailwind CSS. English and Brazilian Portuguese resources are managed with i18next and react-i18next.

Run commands from the repository root. See [development setup](../../docs/development.md).

The interface provides local file/folder imports with SHA-256, paginated library cards, durable task progress, language settings, and a Rust connection check. SQLite schema v4 is migrated at startup. Cancellation, standalone integrity rechecks, and P2P integration remain planned. See [local import](../../docs/library-import.md). See [SQLite storage](../../docs/database.md) and [project structure](../../docs/project-structure.md).

Settings includes [full local library backup and restore](../../docs/library-backup.md), with private JSON snapshots, a reviewed replacement step, and pre-restore recovery copies.

Library cards and file details show [platform/format evidence and exact/readable sizes](../../docs/file-identification.md).
