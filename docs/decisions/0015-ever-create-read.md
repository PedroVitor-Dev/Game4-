# ADR 0015: Native .g4e creation, reading and explicit verification

- Date: November 13, 2026
- Status: Accepted reference implementation; in-app P2P session integration pending

## Context

The public .g4e v1 format needs executable create/read behavior and compatibility evidence. Sharing-reference generation must preserve existing source files, remain responsive and never imply that descriptor parsing has verified remote content or started sharing.

## Decision

Implement Rust strict parsing and serialization against the published version, with shared malformed/independent fixtures. Generate a trackerless BitTorrent v1 companion directly from the imported root's existing files, streaming SHA-256 and cross-file piece SHA-1 through bounded buffers. Keep original paths private and do not make .g4 or staging payload copies. Use read-only source handles and the existing serialized worker/idle gate.

Native Library export writes .g4e and a unique companion; native open returns a reference-only summary and inert magnet text. The offline CLI also supports explicit existing-package references and selected descriptor/torrent/root verification, separating format validity from byte verification. Pair publication is no-clobber with descriptor last; a late failure can leave a new companion and must be reported honestly rather than racing to delete it.

## Consequences and validation

No format, database or backup version change is required. Ordinary torrents let clients outside Game4∞ use the originals without an obligatory package. They still require seeding and connectivity. This implementation never starts sessions or adopts librqbit into desktop runtime; a parser-only independent check is available in the evaluation tool.

See [implemented workflows and evidence](../ever-manifests.md). In-app seeding, source mapping for dispersed roots, discovery, transfer queues and real network validation remain distinct work; generating references must not mark those milestones complete.
