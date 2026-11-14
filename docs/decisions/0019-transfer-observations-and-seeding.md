# ADR 0019: Local transfer observations and explicit continued seeding

- Date: 2026-11-14
- Status: Accepted for the experimental desktop integration

## Decision

Keep real upload/download/source seeding on the integrated engine and add an explicit per-download opt-in to continue seeding at completion. Flush and sync complete files on a worker, seal the shared adapter against writes, and retain the listener until Stop/app exit. Preserve default stop-on-completion and no automatic restart sharing.

Expose received/uploaded counters, sampled rates and the engine's public aggregate live/seeder counts. Derive other connected peers by subtraction with a transient-counter clamp. Label scope and uncertainty explicitly: these are local observations, not global tracker populations. Advertised complete copies establish a lower bound on remote availability; absent seeders mean unknown combined availability. Do not fabricate fractional availability from download progress or claim byte integrity from peer bitfields. Avoid vendoring or probing peers solely to obtain missing piece-union telemetry.

Use nullable live telemetry, preserve final byte totals and invalidate peer/speed observations when stopping. Gate continued seeding by an additive capability. Keep Stop usable independently from long source preparation or metadata lookup.

## Validation

A real throttled loopback transfer observes peer classes and positive upload/download speeds. The completed receiver remains a seed with sealed storage. After the original source stops, a third client downloads from that receiver alone and independently matches SHA-256. Stop releases handles and live observations while retaining counters; old contract/default completion behavior remains covered. Runtime wire validators reject inconsistent peer sums, invalid rates and impossible seeding states. Full swarm availability, private tracker populations, durable queues and WAN acceptance remain outside this increment.
