# BitTorrent engine evaluation

Date: November 4, 2026. Status: **local evidence collected; production selection and two-computer acceptance remain open**.

The owner initially lacked a second computer, then confirmed a second Windows PC on another network. Tailscale was chosen for the test and installed on PC A; device authentication/connectivity is still being prepared. No physical two-computer or cross-network result is claimed.

## Candidate comparison

| Candidate            | Evidence / integration fit                                                                 | Current disposition                                               |
| -------------------- | ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| librqbit 9.1.0       | Rust API, Windows build tested, direct embedding possible; crate declares Apache-2.0       | Selected for an isolated experiment only                          |
| libtorrent-rasterbar | C++ engine with documented API and BSD licensing; requires a Rust/C++ integration strategy | Alternative for further evaluation; not built or benchmarked here |

This comparison is an initial integration assessment, not a full dependency-license audit or an operational performance comparison. Review transitive dependencies, notices, security, and maintenance before production distribution.

## Local test design

The standalone `tools/torrent-probe` crate uses a pinned engine and its own Cargo lockfile. The desktop application has no engine dependency or new network behavior.

- Host: one Windows x64 computer, Rust 1.99.1, debug builds.
- Transport: IPv4 TCP through loopback, explicit initial peer.
- Fixture: deterministic synthetic `payload.bin`, 8,388,625 bytes, 262,144-byte pieces.
- Expected SHA-256: `ca1d758393b634d51aea5298b89b6d598403e41d5e72176e8114fc434b4dfe29`.
- Discovery: DHT, trackers, LSD, and UPnP disabled.
- Restart semantics: new engine session rechecks the same files; fastresume and automatic session persistence are disabled.
- No game files, credentials, public swarms, or firewall changes were involved.

## Results

| Check                        | Result                                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------------------- |
| Windows build                | Passed                                                                                            |
| Partial `.torrent` transfer  | Stopped at 524,305 verified bytes before completion                                               |
| New-session resume           | Reused 524,305 bytes; completed with matching SHA-256                                             |
| Corruption recovery          | Modified first byte; recheck invalidated a piece and redownload restored matching SHA-256         |
| Magnet transfer              | Metadata obtained from explicit seed; completed with matching SHA-256                             |
| Separate OS processes        | Seeder and downloader used different processes; another downloader process resumed partial data   |
| Independent hash             | Python hashlib matched expected SHA-256 after the separate-process run                            |
| Fixture safety tests         | Two tests passed: no overwrite, malformed filename/tracker rejection, and unmarked resume refusal |
| Exact-piece creator boundary | **Failed:** 262,144-byte input produced two hashes instead of one                                 |
| Two physical computers       | **Pending:** second computer unavailable                                                          |

Raw observations: [session report](evaluations/torrent-loopback-2026-11-04.json) and [process report](evaluations/torrent-processes-2026-11-04.json). Elapsed times are observations from a throttled synthetic debug run, not throughput benchmarks. These tests demonstrate recovery of the deliberately corrupted fixture; they are not an adversarial peer-security evaluation.

## Corrective implementation — November 4, 2026

The experiment now uses a Game4-owned streaming BitTorrent v1 metainfo adapter instead of librqbit's defective creator. It hashes the concatenated file stream, emits a final piece only when bytes remain, keeps a partial piece across file boundaries, and preserves zero-byte files within nonempty collections. Dictionary keys are encoded in canonical byte order. Input ordering is deterministic; unreadable/changing files, links/reparse points, invalid piece lengths, and empty datasets are rejected explicitly.

Seven probe tests pass, including exact and partial sizes, deterministic output, nested multi-file hashing with a trailing empty file, invalid options leaving no output, and an actual exact-boundary seed/download. Six desktop database tests also pass after a separate negative-schema-version fix.

Independent Python decoding verified canonical bencode, each SHA-1 piece, and the info hash for both the full synthetic fixture and the exact-boundary fixture. The full transfer/resume/magnet/corruption experiment was repeated successfully using adapter-created metainfo.

New evidence: [corrected-creator transport report](evaluations/torrent-fixed-creator-2026-11-04.json) and [independent metainfo report](evaluations/torrent-independent-metainfo-2026-11-04.json). The older reports above are preserved as historical observations.

The report's `creator_exact_piece_boundary.passed` describes the unmodified upstream diagnostic and remains false. Its `adapter_passed` is true. `candidate_approved` remains false because physical-network, independent-client, and other acceptance gates are still open. The adapter is experimental and not connected to desktop user imports.

## Upstream finding and local resolution: torrent creation

The exact-boundary probe reproduces an additional piece hash in `create_torrent` on librqbit 9.1.0. The upstream report describes the same issue for 9.0.1: [rqbit issue 698](https://github.com/ikatson/rqbit/issues/698). Our current-version result is local evidence, independent of the older report.

Do not adopt the upstream creator unchanged. The local adapter removes this dependency from fixture generation and covers the relevant local regressions. Independent-client interoperability is still required before production adoption. No registry source was patched and no upstream issue/comment was posted.

## Remaining acceptance gates

1. Repeat transfer, process restart, magnet, and SHA-256 checks on the two physical Windows PCs. For the current different-network setup, record VPN topology and direct/relay path explicitly; this does not validate native BitTorrent NAT traversal.
2. Test with an independent BitTorrent client and validate generated metainfo, including exact piece boundaries and multi-file collections.
3. Validate the corrected creator with an independent client and review file/path handling before accepting user torrents.
4. Evaluate DHT, trackers, disconnection behavior, peer limits, and cancellation independently.
5. Treat WAN/NAT as a separate topology requiring explicit connectivity planning; LAN success is insufficient.
6. Review dependency licensing/security, resource consumption, release builds, and integration with the desktop task model.

M3 remains in progress. The immediate local-library roadmap can continue while physical testing is unavailable.

## Primary references

- [rqbit repository](https://github.com/ikatson/rqbit)
- [Pinned librqbit 9.1.0 API](https://docs.rs/librqbit/9.1.0/librqbit/)
- [libtorrent overview and license](https://libtorrent.org/)

References checked on November 4, 2026; implementation was compiled against the downloaded pinned crate source.
