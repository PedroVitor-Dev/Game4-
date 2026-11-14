# ADR 0020: Private resumable transfer records

- Date: 2026-11-14
- Status: Accepted for the experimental desktop integration

## Decision

Persist transfer intent and sampled history in a separate versioned SQLite store, keeping the catalog schema and public formats unchanged. Save before unpausing and checkpoint approximately once per second. Reopen interrupted work Paused, never automatically reconnecting. Recreate sessions on explicit Resume with full piece verification, safe existing-file handles and no fast-resume bitmap trust. Retain assigned ports and cumulative last-known counters.

Pause closes sockets, syncs partial files and releases handles. Source seeding stays read-only and refuses changed content. Write authority for download repair is limited to the recorded application-owned directory and validated relative paths, with a matching adjacent ownership marker and creation stamp. Never truncate oversized files or adopt relocated/reparse targets. Keep Stop resumable and offer record-only forgetting to manage the persistent entry limit.

Serialize lifecycle changes and guard monitor generations/cancellation, so stale background work cannot affect a newer session. Fail closed on invalid/unavailable state without resetting records or preventing local Library operation. Keep file paths and tracker credentials private; catalog backup and public exports do not include transfer state.

## Validation and consequences

Tests cover pause/reopen/repair, original refusal, stable identity/port, ownership checks, oversized-file preservation, concurrent/cancelled resume and safe record forgetting. A separate interrupted client process reopens offline, rechecks more than 1 MiB of existing valid pieces and completes with matching SHA-256. Stored counters can lag abrupt interruption by one sample interval; a resume scan, not saved progress, establishes integrity. Power-loss guarantees, saved-route editing, relocation, automatic restart sharing and WAN acceptance are outside this increment. See [transfer persistence](../transfer-persistence.md).
