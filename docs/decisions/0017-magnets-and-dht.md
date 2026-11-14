# ADR 0017: Explicit magnet acquisition and optional DHT

- Date: 2026-11-14
- Status: Accepted for the experimental desktop integration

## Decision

Add offline magnet generation from checked catalog originals and explicit cancellable metadata acquisition. Support one public v1 BTIH identity, normalized from hex/base32; ignore advisory tracker/name/length parameters and reject remote source or embedded-peer parameters. Resolve metadata through a separate temporary list-only engine session, verify identity and the existing portable torrent profile, then require file review and a native destination selection before payload download.

Make DHT an opt-in per network action for metadata lookup, original seeding and downloads. Use the pinned engine's UDP DHT implementation and public bootstrap defaults without persistence. A manual peer remains available and is required only when DHT is off. Keep trackers, local discovery and UPnP off. Report each payload session's DHT choice; changing the checkbox affects subsequent actions only. DHT lookup does not bypass connectivity restrictions.

Limit to one metadata lookup, one metadata peer at a time, eight seconds per candidate and 60 seconds overall, plus cleanup. Separate discovery from metainfo connection to avoid the upstream 128-peer metadata fan-out. Reuse safe storage for payload sessions and preserve originals. Cancel temporary network children on explicit cancellation/page departure/app exit.

## Validation and limits

An isolated real UDP DHT swarm with reciprocal bootstrap routing entries validates a seeder announcement, peer discovery, magnet acquisition and payload transfer with independent SHA-256, without an explicit seeder address on the receiving side. Direct-peer metadata and cancellation tests also pass. Public-DHT availability, real WAN/NAT behavior and independent-client acceptance remain separate gates. Durable queues/resume and protocol-handler registration are outside this increment. See [the current guide](../bittorrent.md) for limits and operational behavior.
