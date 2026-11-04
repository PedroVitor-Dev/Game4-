# Two-computer torrent result — November 4, 2026

Status: **passed for the tested topology: two physical Windows PCs on different networks, using Tailscale DERP relay**.

## Build and evidence

- Probe source commit: `b84024da3926a56b9ae797d8bd57c96be0af3cc2`.
- Engine: librqbit 9.1.0; creator: Game4 streaming BEP 3 v1 adapter.
- Windows x64 debug executable. PC B used the portable package without Rust or Node installation.
- Package contained metainfo and expected hashes, but no payload; the payload remained on PC A.
- Receiver output and Get-FileHash evidence were supplied by the owner from PC B. Seeder readiness and VPN path were observed on PC A.
- Personal hostnames, account details, actual VPN IPs, and filesystem paths are omitted from committed evidence.

## Observations

| Check              | Result                                                                                   |
| ------------------ | ---------------------------------------------------------------------------------------- |
| Initial transfer   | Stopped at 524,305 of 8,388,625 bytes as requested                                       |
| New-process resume | Reused 524,305 verified bytes and completed                                              |
| Resume integrity   | Engine SHA-256 matched; independent PowerShell Get-FileHash also matched                 |
| Magnet             | Completed all 8,388,625 bytes; SHA-256 matched                                           |
| Network path       | DERP relay in the sao region before and after the test; no direct connection established |

Expected and received SHA-256: `ca1d758393b634d51aea5298b89b6d598403e41d5e72176e8114fc434b4dfe29`.

The receiver reported 9.259114 seconds for resumed completion and 10.2650709 seconds for magnet completion. These are observations from one synthetic, rate-limited debug test, not throughput benchmarks.

## Scope and cleanup

This closes the physical two-PC transfer/resume/magnet criterion for an explicitly connected overlay. It does not demonstrate native BitTorrent NAT traversal, direct WAN connectivity, DHT, trackers, independent-client compatibility, or multi-file transfer.

The seed and package services were stopped after the test; the narrow temporary firewall exceptions were removed. Tailscale remains installed and connected for future work. No router port forwarding, exit node, public swarm, or production desktop sharing was enabled.

See [structured observations](torrent-two-pc-vpn-2026-11-04.json) and [the engine evaluation](../torrent-engine-evaluation.md). Production engine adoption remains subject to the remaining acceptance gates.
