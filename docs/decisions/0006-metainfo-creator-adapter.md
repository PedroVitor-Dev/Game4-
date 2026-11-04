# ADR 0006 — Replace the experimental metainfo creator

Date: November 4, 2026. Status: accepted for the isolated experiment; production adoption pending.

## Context and alternatives

librqbit 9.1.0 transport passed local checks, but its creator appended an extra SHA-1 at exact piece boundaries. Options were waiting for an upstream release, patching/vendoring the library, or separating metainfo generation from transport.

## Decision

Use a small Game4-owned streaming BEP 3 v1 creator in the probe. Keep the upstream package pinned and unmodified, retaining its creator only for the diagnostic. Hash across file boundaries, emit partial final pieces only when nonempty, preserve empty files within a nonempty collection, and encode sorted dictionaries deterministically. Reject empty datasets and links/reparse points explicitly.

## Evidence and consequences

Boundary and multi-file regression tests, an exact-boundary transfer, independent bencode/SHA-1 checks, and repeated transfer/resume/magnet/repair checks passed locally. Preserve SHA-256 as the independent preservation check; SHA-1 is used for BitTorrent v1 compatibility only.

This adds code to maintain and does not close independent-client or physical-network acceptance gates. The adapter remains outside desktop user imports. The owner has a second Windows PC on another network and chose Tailscale for the next test; direct versus relayed overlay connectivity must be recorded accurately.

Reference: [BEP 3](https://www.bittorrent.org/beps/bep_0003.html) and [evaluation evidence](../torrent-engine-evaluation.md).
