# ADR 0016: Experimental desktop BitTorrent sessions

- Date: 2026-11-13
- Status: Accepted for the bounded desktop experiment; production acceptance remains open

## Decision

Embed pinned librqbit 9.1.0 behind native session commands and custom handle storage. Reuse the independently checked Game4 metainfo creator instead of the upstream creator that failed exact-piece-boundary tests. Create and import standard public v1 `.torrent` files offline; require an explicit action to seed or download. Seed catalog originals through read-only handles and receive into a new owned directory without overwrites. Keep engine state out of SQLite and preservation formats in this first increment.

Disable DHT, trackers, local discovery and UPnP. Require an initial IPv4 peer for downloads and retain documented engine peer exchange. Bound metadata, payload layout, peers, sessions and preview lifetime. Cancel sessions at application exit and explicitly stop completed downloads. Keep existing `.g4e` descriptors compatible without making their inspection start networking.

## Consequences

The desktop can perform real transfers without `.g4` conversion or staging originals. Dedicated source storage prevents the engine from truncating, modifying or deleting preservation files. Network and task state remain separate. Runtime-only sessions do not offer restart/resume, and interrupted files remain for the user. Discovery, magnet acquisition, durable queues, independent-client testing, native WAN/NAT behavior, dependency security review and release acceptance remain open. Read [the operational guide](../bittorrent.md) for the precise supported profile and limits.
