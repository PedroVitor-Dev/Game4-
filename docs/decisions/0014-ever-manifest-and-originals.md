# ADR 0014: Original preservation and Game4 Ever Manifest

- Date: November 13, 2026
- Status: Accepted initial public specification; production P2P integration pending

## Context

The owner clarified that .g4 is an open lossless preservation package, while .g4e means Game4 Ever Manifest: metadata for P2P discovery/sharing. Packaging must be optional. Users must be able to share their existing originals without conversion, mandatory staging copies, encryption, DRM or central-server dependency.

## Decision

Keep .g4 v1's ZIP/ZIP64 and manifest SHA-256 rules unchanged. Make read-only preservation an explicit cross-workflow contract and validate it with a complete local-workflow regression test. Define .g4e v1 as strict UTF-8 JSON, with an embedded manifest describing transmitted bytes, an explicit original_files/g4_package representation, alternative BitTorrent v1 identities, matching magnet URIs and optional companion metainfo references. The earlier .g4t concept is superseded.

Separate raw-file SHA-256, SHA-1 torrent info identity and whole-metainfo SHA-256. Require metadata/payload binding before transfers. Validate descriptors offline; never connect, read referenced payloads or seed merely because a descriptor was parsed. Trackers/companions are optional; no identity, access or availability service is mandatory.

## Alternatives and consequences

Embedding payload bytes would duplicate .g4 and obstruct direct sharing. An executable/emulation descriptor or encrypted package would solve a different problem. A magnet alone cannot record the original per-file SHA-256 and preservation metadata; .g4e binds those claims to explicit representations but does not sign them or prove availability.

The initial profile targets the v1 swarm metadata used by the evaluated probe, with v2/hybrid/padding support deferred until specified and tested. Desktop native-file source mappings, metadata acquisition, transfer queue and user sharing controls remain required before claiming production direct sharing. The schema/validator do not add that capability automatically.

## Validation

See the [public specification](../../packages/g4e-format/spec-v1.md), [original preservation policy](../original-preservation.md) and project log for executed evidence and remaining gates.
