# ADR 0021: isolated G4EP private-MVP ports

- Date: 2026-11-14
- Status: Accepted for step 1 architecture; no codec/network runtime delivered

## Context

The requested G4EP MVP adds erasure-coded preservation for arbitrary files, starting with an authorized ~700 MB private-network experiment. Existing librqbit sessions handle BitTorrent pieces and recovery; they do not supply parity reconstruction, node quotas or a repair graph. Existing .g4e v1 metadata cannot be silently changed into a mandatory erasure-code manifest.

## Decision

Add a dependency-light internal `g4ep` Rust module with explicit Encoder, Decoder, StorageNode, PeerDiscovery, FragmentTransfer, IntegrityVerification and RepairManager ports. Keep blocking streaming codecs/storage separate from async network I/O, require bounded data/contexts, model original/fragment hashes independently and isolate peer discovery behind checked explicit private enrollment. No implementation adapter, IPC capability, UI action, public-format serialization or database migration is registered in step 1.

Reed–Solomon GF(256), 8+4 shards of 1 MiB and a separate private `g4ep/state.sqlite3` are proposed implementation starting points. Exact codec compatibility, storage schema, authenticated transport and public descriptor require later implementation/acceptance. Public DHT is not the default private discovery adapter. Existing .g4/.g4e/.g4lib formats, originals, catalog backups and transfer recovery remain intact.

## Consequences

Real code can implement/test the ports without Tauri, database or torrent-manager ownership leaking into codec logic. Validated planning values do not claim byte integrity, authenticated custody, successful reconstruction or host-failure tolerance. Private-network allowlists are a guard, not an implemented secure transport. Two-PC transfer tests and multi-host loss tolerance are distinct gates. See [architecture and acceptance plan](../g4ep/architecture.md).
