# ADR 0013: Public .g4 package v1

- Date: November 13, 2026
- Status: Accepted initial public format

## Context

Manifest export records identities but leaves original files behind. Preservation needs a documented self-contained package readable offline with ordinary tools, without recompression or an opaque Game4-only codec.

## Decision

Use single-volume ZIP/ZIP64 with Stored entries only, `game4.json` descriptor, existing manifest v1 and `data/<manifest path>` payload. Require exact declared entry sets and full size, CRC and SHA-256 validation. Publish a versioned descriptor schema, normative limits, independent Python fixtures and a reference offline CLI. The existing manifest contract remains unchanged.

The native app exports per Library item or opens a package for full verification and summary. Use background workers, stable localized errors and no-clobber publication. Source selection may fall back to another registered matching copy. Recheck bytes while copying, preserve reference hashes and never execute package content. Read operations do not extract files or mutate the library.

## Alternatives and consequences

A custom binary framing would make independent recovery harder. Compression can save space but complicates seek/resume behavior and introduces decompression resource concerns; v1 excludes it. TAR is widely available but ZIP offers a central index and familiar Windows tooling; ZIP64 addresses size/count limits. The ZIP crate has all default compression/encryption features disabled.

This duplicates identical logical payload entries rather than introducing content-addressed reconstruction. Packages use more disk than compressed archives and creation reads selected inputs twice to select valid copies and verify the copied bytes. Busy indicators have no byte-level progress or cancellation yet. Signing, extraction, catalog reconstruction and very large physical-file validation remain separate increments.

## Validation

See the [public specification](../../packages/g4-format/spec-v1.md) and project log for executed checks. ZIP64 small fixtures do not establish performance at multi-gigabyte scale.
