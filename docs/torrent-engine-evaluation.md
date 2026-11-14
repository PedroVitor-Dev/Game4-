# BitTorrent engine evaluation

Date: November 4, 2026. Status: **local checks and physical two-PC overlay transfer/resume/magnet passed; production adoption and native-network acceptance remain open**.

The owner provided a second Windows PC on another network. Both PCs were connected through Tailscale and completed the physical test using a DERP relay in the sao region. This is an overlay transport result, not a native direct-WAN or NAT-traversal result. See [the physical test record](evaluations/two-computer-result-2026-11-04.md).

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
| Two physical computers       | Passed transfer, new-process resume, independent hash, and magnet through Tailscale DERP relay    |

Raw observations: [session report](evaluations/torrent-loopback-2026-11-04.json) and [process report](evaluations/torrent-processes-2026-11-04.json). Elapsed times are observations from a throttled synthetic debug run, not throughput benchmarks. These tests demonstrate recovery of the deliberately corrupted fixture; they are not an adversarial peer-security evaluation.

## Corrective implementation — November 4, 2026

The experiment now uses a Game4-owned streaming BitTorrent v1 metainfo adapter instead of librqbit's defective creator. It hashes the concatenated file stream, emits a final piece only when bytes remain, keeps a partial piece across file boundaries, and preserves zero-byte files within nonempty collections. Dictionary keys are encoded in canonical byte order. Input ordering is deterministic; unreadable/changing files, links/reparse points, invalid piece lengths, and empty datasets are rejected explicitly.

Seven probe tests pass, including exact and partial sizes, deterministic output, nested multi-file hashing with a trailing empty file, invalid options leaving no output, and an actual exact-boundary seed/download. Six desktop database tests also pass after a separate negative-schema-version fix.

Independent Python decoding verified canonical bencode, each SHA-1 piece, and the info hash for both the full synthetic fixture and the exact-boundary fixture. The full transfer/resume/magnet/corruption experiment was repeated successfully using adapter-created metainfo.

New evidence: [corrected-creator transport report](evaluations/torrent-fixed-creator-2026-11-04.json) and [independent metainfo report](evaluations/torrent-independent-metainfo-2026-11-04.json). The older reports above are preserved as historical observations.

The report's `creator_exact_piece_boundary.passed` describes the unmodified upstream diagnostic and remains false. Its `adapter_passed` is true. `candidate_approved` in that historical local-only report remains false. The subsequent physical two-PC overlay gate passed; independent-client, native-network, and production acceptance remain open. The adapter is experimental and not connected to desktop user imports.

## Upstream finding and local resolution: torrent creation

The exact-boundary probe reproduces an additional piece hash in `create_torrent` on librqbit 9.1.0. The upstream report describes the same issue for 9.0.1: [rqbit issue 698](https://github.com/ikatson/rqbit/issues/698). Our current-version result is local evidence, independent of the older report.

Do not adopt the upstream creator unchanged. The local adapter removes this dependency from fixture generation and covers the relevant local regressions. Independent-client interoperability is still required before production adoption. No registry source was patched and no upstream issue/comment was posted.

## Remaining acceptance gates

1. **Completed for the overlay topology:** transfer, process restart, magnet, and SHA-256 checks on two physical Windows PCs. The recorded path was Tailscale DERP relay; native BitTorrent NAT traversal is not validated.
2. Test with an independent BitTorrent client and validate generated metainfo, including exact piece boundaries and multi-file collections.
3. Validate the corrected creator with an independent client and review file/path handling before accepting user torrents.
4. Evaluate DHT, trackers, disconnection behavior, peer limits, and cancellation independently.
5. Treat WAN/NAT as a separate topology requiring explicit connectivity planning; LAN success is insufficient.
6. Review dependency licensing/security, resource consumption, release builds, and integration with the desktop task model.

The M3 physical transport experiment now has passing evidence for the tested overlay. Full engine adoption and desktop integration remain open. The local-library roadmap can continue independently.

## Primary references

- [rqbit repository](https://github.com/ikatson/rqbit)
- [Pinned librqbit 9.1.0 API](https://docs.rs/librqbit/9.1.0/librqbit/)
- [libtorrent overview and license](https://libtorrent.org/)

References checked on November 4, 2026; implementation was compiled against the downloaded pinned crate source.

## Desktop BitTorrent increment — November 13, 2026

The desktop now embeds real experimental BitTorrent sessions, creates and imports public v1 `.torrent` files, seeds original catalog files through read-only storage and downloads to new owned directories. Offline previews and explicit initial-peer actions are available in both locales. DHT/trackers/local discovery/UPnP are disabled; peer exchange remains enabled. Sessions are runtime-only, with Stop and completion cleanup; persistence, resume, discovery and production acceptance remain open. See [desktop transfers](bittorrent.md) and [ADR 0016](decisions/0016-desktop-bittorrent.md). Earlier dated experiment statements describe their original scope.

## Magnets and DHT — November 14, 2026

Transfers now generates magnets without a metainfo export, imports them through cancellable metadata-only acquisition, and optionally discovers/announces peers over ephemeral UDP DHT. DHT is off by default; enabled actions can omit the initial peer. Strict v1/hash/path validation and explicit destination review remain in place. Real isolated-DHT discovery and transfer tests pass; public-WAN/NAT acceptance, persistence and resume remain open. See [desktop transfers](bittorrent.md) and [ADR 0017](decisions/0017-magnets-and-dht.md).
