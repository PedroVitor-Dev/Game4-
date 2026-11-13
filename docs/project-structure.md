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
│       │   ├── contracts/         Versioned DTOs and TypeScript exporter
│       │   ├── database/          SQLite connection, migrations, and tests
│       │   ├── library/           Local import, hashing, catalog queries, and tests
│       │   ├── manifests/         Versioned JSON snapshots, validation, and export
│       │   ├── tasks/             Task kind and status vocabulary
│       │   ├── security/          Reserved validation boundary (documentation only)
│       │   └── torrent/           Reserved transfer boundary (documentation only)
│       ├── capabilities/          Window capabilities
│       └── icons/                 Temporary scaffold icons
├── packages/ipc-contracts/        Rust-generated wire types, examples, and runtime decoders
├── packages/manifest-schema/      Reserved public-schema workspace
├── tools/torrent-probe/           Standalone experimental BitTorrent CLI
├── tests/fixtures/                Shared-fixture policy; no personal collections
├── docs/                          Plans, architecture, decisions, and project log
├── package.json                   Root npm workspace and commands
└── package-lock.json              Shared frontend dependency lock
```

Only npm workspaces with a package manifest are application packages. Reserved directories contain documentation, not implemented adapters or stable formats. Database tests currently live beside the Rust database module and create their own temporary fixtures.

Runtime data is outside the repository. See [SQLite storage](database.md) for its location and lifecycle. Build outputs, dependencies, personal databases, and secrets must remain untracked.

The `packages/manifest-schema` folder now contains the public v1 schema, synthetic example, shared rejection corpus, and offline schema tests. Rust manifest tests exercise the same data. This format folder is not an npm application workspace.

The Rust `backups` module owns full-catalog JSON snapshots, validation, preview tokens, and transactional restoration. `LibraryBackup.tsx` provides the bilingual Settings workflow. See [library backup](library-backup.md).

`packages/g4-format/` contains the public package specification, descriptor schema and synthetic interoperability fixtures. The Rust package reader/writer is in `apps/desktop/src-tauri/src/packages/`; `examples/g4.rs` supplies the offline CLI.

`packages/g4e-format/` contains the public Game4 Ever Manifest specification/schema, offline semantic validator, examples and conformance tests. `docs/original-preservation.md` defines the cross-workflow read-only input contract.

Rust `apps/desktop/src-tauri/src/ever/` implements .g4e and torrent binding. `examples/ever.rs` supplies the offline CLI. `tools/torrent-probe/examples/inspect_metainfo.rs` is a parser-only independent compatibility check, not a seeding service.
