# ADR 0018: Explicitly selected tracker routing

- Date: 2026-11-14
- Status: Accepted for the experimental desktop integration

## Decision

Support selected HTTP/HTTPS/UDP trackers for payload sessions, bounded sequential magnet acquisition, standard torrent export and generated magnets. Keep opt-in off by default, supply no public default tracker, and expose imported routing as reviewable suggestions. Do not automatically contact metainfo or magnet endpoints. Rebuild the torrent routing envelope with only the selected list while preserving exact info bytes, ignoring unrelated network-source metadata.

Normalize and bound endpoint lists, reject unsupported schemes/credentials/fragments/UDP query options and gate new behavior with an offline native capability. Preserve old v1 callers with empty-list defaults and old response decoding with zero/empty defaults. Keep catalog identity and read-only original storage independent of routing.

Use pinned engine tracker machinery for payloads and the matching tracker-comms library for temporary metadata discovery. Feed its discovered peers into the existing sequential metadata resolver, retaining cancellation and deadlines. Do not promise ordered tier failover, private-torrent support, persistence or guaranteed stopped-event delivery.

## Validation

Local HTTP and real BEP 15 UDP mock trackers observe the seed announce, discover metadata and complete a real payload transfer with DHT disabled and no initial peer; received SHA-256 matches source. An embedded but unselected tracker receives no requests. Tests also cover endpoint policy, exported info-hash stability, generated magnet routing, IPC capability guards and backward-compatible fields. HTTPS uses Rust TLS with normal certificate validation; public HTTPS/WAN and independent-client acceptance remain open. See [the operational guide](../bittorrent.md).
