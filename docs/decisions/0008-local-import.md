# ADR 0008: Local-only, atomic file import

Date: November 4, 2026

Status: Accepted

## Context

Library imports must preserve original files, separate content identity from physical copies, keep the UI responsive, and recover honestly from failure. The IPC draft already defines picker tokens, durable request IDs, and tasks.

## Decision

Use native Rust file/folder pickers and five-minute process-local tokens. Stream SHA-256 on serialized blocking workers. Persist request deduplication and progress in schema v2. Stage metadata in memory (bounded file count); commit the complete selection and successful task atomically. Reimport the same canonical root into its existing item. Retain earlier content identities. Reject links/reparse points, unsupported objects, inaccessible files, and empty folders rather than silently skipping them. Mark unfinished tasks interrupted at startup.

Use typed, runtime-validated IPC responses and polling snapshots initially. Events, cancellation, independent rechecks, search UI, and partial-success imports remain separate work. Indexing does not start sharing.

## Alternatives

Copying everything into managed storage would consume additional disk space and needs a separate ownership policy. Committing every file immediately would expose partially successful selections. Silent skipping would hide missing material from preservation users. Lowercasing every Windows path could merge distinct case-sensitive files. Each was rejected for this increment.

## Consequences and verification

Large imports retain bounded metadata until commit and report file-based progress. Case/namespace aliases may produce multiple location/item rows, while content identity still deduplicates. Read-time fingerprints cannot guarantee that files remain unchanged later. The importer is not an adversarial filesystem sandbox. Cancellation remains an M1 gate.

See [library import](../library-import.md) for behavior, limits, tests, and future work.
