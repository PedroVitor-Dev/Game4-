# Project structure

```text
Game4-/
├── .github/workflows/             Windows CI checks
├── apps/desktop/
│   ├── src/
│   │   ├── components/            Reusable interface components
│   │   ├── hooks/                 React lifecycle and state integration
│   │   ├── pages/                 Workspace page content
│   │   ├── services/              Typed Tauri IPC adapters
│   │   ├── locales/               English and Brazilian Portuguese resources
│   │   ├── App.tsx                Navigation and application shell
│   │   └── i18n.ts                Localization configuration
│   └── src-tauri/
│       ├── src/
│       │   ├── commands/          Validated Tauri entry points
│       │   ├── database/          SQLite connection, migrations, and tests
│       │   ├── library/           Collection domain vocabulary
│       │   ├── tasks/             Task kind and status vocabulary
│       │   ├── security/          Reserved validation boundary (documentation only)
│       │   └── torrent/           Reserved transfer boundary (documentation only)
│       ├── capabilities/          Window capabilities
│       └── icons/                 Temporary scaffold icons
├── packages/manifest-schema/      Reserved public-schema workspace
├── tools/torrent-probe/           Standalone experimental BitTorrent CLI
├── tests/fixtures/                Shared-fixture policy; no personal collections
├── docs/                          Plans, architecture, decisions, and project log
├── package.json                   Root npm workspace and commands
└── package-lock.json              Shared frontend dependency lock
```

Only npm workspaces with a package manifest are application packages. Reserved directories contain documentation, not implemented adapters or stable formats. Database tests currently live beside the Rust database module and create their own temporary fixtures.

Runtime data is outside the repository. See [SQLite storage](database.md) for its location and lifecycle. Build outputs, dependencies, personal databases, and secrets must remain untracked.
