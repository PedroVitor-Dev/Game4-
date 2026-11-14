# ADR 0022: G4EP streaming RS(8,4) codec and experimental local output

- Date: 2026-11-14
- Status: Accepted experimental implementation for step 2

## Context

The requested encoder must handle arbitrary large files without whole-file buffering, emit 8 data + 4 recovery shards per configurable group and demonstrate recovery from any eight intact shards. Step 1 proposed a GF(256) codec but did not freeze an algorithm, public wire ID or library. The original rust-rse/reed-solomon-erasure upstream advertises a maintainer search; selecting it without addressing the maintenance requirement would be inappropriate.

## Decision

Pin reed-solomon-simd 3.1.0, retaining its MIT/BSD-3-Clause notices. Use its Leopard GF(2^16) **explicit high-rate** engine with codec ID `g4ep-rs16-leopard-high-8-4-v1`, never the draft GF(256) ID. DefaultEngine performs CPU dispatch only; the chosen coding rate/indexing remain explicit. Default groups contain 8 MiB of source data, encoded as twelve independent 1 MiB fragments. Configured shard lengths remain multiples of 64 for stable byte layout.

Implement the existing Encoder port with one library workspace and one reusable shard buffer, bounded reads, SHA-256 per fragment/whole object and context checkpoints. Data fragments remain systematic, parity uses indexes 8–11 and final padding is zero. Publish a typed manifest only after exact source EOF and successful sink consumption. A local CLI/file adapter makes the result usable now with a new-directory-only destination, synced no-clobber writes and a final inventory/incomplete-marker boundary. Its JSON inventory is explicitly experimental, not a new public .g4e interpretation or a completed storage-node protocol.

## Consequences and validation

Any eight valid distinct indexes recover a group under this codec; hashes must exclude corruption before a future decoder uses shards. Tests exercise all 495 selections, a pinned digest vector, scalar/default parity equality, multi-group/padding and failure/cancellation paths. The authorized 700 MB local file run demonstrates bounded memory and independent on-disk verification. Decoder service, resumed encoding, authenticated P2P, quota-managed nodes, public manifests and automated repair remain later work. See [encoding details and evidence](../g4ep/encoding.md).
