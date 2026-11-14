# 0023: Versioned G4EP reconstruction manifests

Date: 2026-11-14. Status: accepted for the experimental private MVP.

## Decision

Use `.g4e` schema v2 for G4EP protocol v1 fragment descriptors. Keep the complete v1 wire format and payload operations unchanged. Dispatch readers by explicit version and reject unknown versions. Extend existing native open/import previews without a database migration or network side effects.

Bind original metadata, exact Leopard codec parameters, contiguous groups, every fragment digest and explicit private discovery through deterministic SHA-256. Support optional Ed25519 signatures with domain separation and independently pinned public-key fingerprints. Included signatures alone never imply publisher trust. Do not persist private signing keys or absolute source paths.

## Consequences

Rust and Node validators enforce structural and semantic constraints and share signed, unsigned and malformed fixtures. The local exporter verifies all fragments before writing the descriptor. Existing v1 torrent/archive operations fail closed on v2: reconstruction metadata must not become an original-file magnet download. Distributed decoding, availability discovery and repair remain separate implementation steps.

See the [public specification](../../packages/g4e-format/spec-v2.md) for exact canonicalization, bounds, codec and trust rules.
