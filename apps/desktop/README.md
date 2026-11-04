# Game4∞ desktop

Tauri 2 shell with a Rust core and React / TypeScript frontend, built with Vite and Tailwind CSS. English and Brazilian Portuguese resources are managed with i18next and react-i18next.

Run commands from the repository root. See [development setup](../../docs/development.md).

The interface currently provides library and task empty states, language settings, and a read-only Rust connection check. SQLite storage is initialized by the Rust core at startup. File importing and P2P transfers are not implemented. See [SQLite storage](../../docs/database.md) and [project structure](../../docs/project-structure.md).
