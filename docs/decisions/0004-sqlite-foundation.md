# ADR 0004 — Local SQLite foundation

Date: November 4, 2026. Status: accepted.

## Context and alternatives

The next local-library work needs durable content identities, physical locations, editorial records, and task states. Options include a frontend SQL plugin, an async SQL framework, or a small Rust-owned SQLite layer.

## Decision

Use rusqlite with bundled SQLite and an application-managed mutex-protected connection. Resolve local application data through Tauri, initialize on startup, and execute database IPC on a blocking worker. Do not expose arbitrary SQL to the frontend.

Use ordered embedded SQL migrations with transactional `user_version` tracking, foreign keys, WAL, a bounded busy timeout, and FULL synchronous mode. Reject newer schemas without destructive reset. Separate content identity from physical-copy verification and editorial identity. Add only the import/verification task vocabulary needed by the local library; transfers and manifests remain deferred.

Organize frontend code into components, pages, hooks, and services; organize Rust into commands, database, library, and tasks. Reserve security, torrent, schema, and fixture boundaries with explicit documentation instead of fake implementations.

## Consequences

No separate SQLite installation or generic SQL plugin is required. Serialized access is sufficient for the initial scope; revisit connection pooling only after measured contention. WAL requires an explicit backup strategy. The importer must still validate and normalize paths. A task table does not implement task execution or recovery. Future migrations must preserve user data and be tested before release.

## Validation

Use temporary-database tests for persistence, constraints, rollback, and newer-schema refusal. Verify native startup and the summary IPC against the on-disk application database. Record executed results in the project log.
