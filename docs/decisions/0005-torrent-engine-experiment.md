# ADR 0005 — Isolated torrent engine experiment

Date: November 4, 2026. Status: accepted experiment; production engine selection pending.

## Context

The owner requested validation between two computers but confirmed that the second PC is not yet available. Local transport evidence can be collected without adding an unvalidated engine to the desktop.

## Decision

Evaluate pinned librqbit 9.1.0 in a standalone crate using synthetic content and explicit loopback TCP peers. Keep public discovery, automatic port mapping, the desktop catalog, and production engine integration outside the experiment. Record raw results and prepare a physical-LAN procedure for later execution.

## Evidence and consequences

Local sessions and separate OS processes passed transfer, resume, magnet, corruption recovery, and hash checks. Exact-piece-boundary creation produced an extra hash. The creator must be fixed or replaced and independently validated before adoption. Physical LAN, cross-client, multi-file, discovery, and WAN tests remain open. The probe's successful transport exit does not imply production approval.

See [evaluation and evidence](../torrent-engine-evaluation.md) for results, alternatives, references, and remaining gates. No upstream registry code was patched and no production engine decision was made.
