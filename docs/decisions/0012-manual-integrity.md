# ADR 0012: Manual integrity of registered content copies

- Date: November 13, 2026
- Status: Accepted

## Context

Import-time hashes do not prove that originals remain readable or unchanged. Verification must keep the interface responsive, distinguish lost access from changed bytes, and preserve catalog reference identities.

## Decision

Check all distinct registered physical copies of the selected item’s content using the existing secure streaming reader and shared background worker. Store current copy observations and task progress in schema v4. Commit all findings with task completion atomically; operational failure leaves earlier observations untouched. Expose aggregate counts through a separate read command, keeping existing manifest and identification DTOs unchanged.

Use request UUIDs for admission idempotency across import and verification. Expose unlocated content explicitly and label succeeded tasks as completed checks, with findings visible in Library. Restore continues to invalidate copy observations.

## Alternatives and consequences

Checking only relative source paths would require a new source ownership model and could omit useful known copies. Storing immutable per-run reports would require a new migration and backup format; it is deferred. Current observations can be replaced by subsequent checks and are explicitly historical. No automatic repair or reference-hash updates occur.

## Validation

See [manual integrity](../library-integrity.md) for behavior, synthetic checks, and remaining limits.
