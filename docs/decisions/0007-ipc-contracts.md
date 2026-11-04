# ADR 0007 — Rust-owned versioned desktop IPC contracts

Date: November 4, 2026. Status: accepted for active commands; library/task contracts remain drafts.

## Context and alternatives

Frontend interfaces previously duplicated Rust response shapes, with no runtime decoding or protocol negotiation. Options included handwritten mirrored types, schema-first generation, or generation from Rust DTOs.

## Decision

Own DTOs and enum vocabulary in Rust; generate TypeScript with ts-rs and golden JSON examples with serde. Share them through a private npm workspace package. Validate incoming active responses and sanitize errors in the typed frontend client. Negotiate a major contract version and capabilities through app_info, keeping database schema version separate.

Use stable error codes plus typed nullable field names, with frontend English/Portuguese localization. Bound numeric summary counts to JavaScript's safe range and reserve decimal strings for future identifiers, byte totals, progress, and event revisions.

Define library/task requests, pagination, idempotency, cancellation, and snapshot-event semantics as draft contracts without registering fake handlers. Keep executable command and draft registries distinct.

## Consequences

Wire types are generated once and committed; drift fails Rust tests and CI checks. Runtime validators still require deliberate maintenance when active DTOs change. New draft features require real business validation and the corresponding persistence migrations before activation. The current UI continues to use the existing read-only commands, now through a validated client.

See [protocol specification](../ipc-contracts.md) for command mapping, compatibility rules, test strategy, and future prerequisites.
