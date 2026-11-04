# Game4∞ torrent probe

An isolated evaluation executable pinned to `librqbit 9.1.0`. It is not linked into the desktop application and does not access the library database or personal collections.

The probe only supports its generated 8 MiB + 17 byte single-file fixture. It uses explicit IPv4 TCP peers, with DHT, trackers, local service discovery, UPnP, and session persistence disabled. It does not expose an HTTP API. No firewall rules are changed.

From the repository root:

```powershell
npm.cmd run torrent:build
npm.cmd run torrent:test
npm.cmd run torrent:check
$run = Join-Path $env:TEMP ("game4-torrent-" + [guid]::NewGuid().ToString("N"))
npm.cmd run torrent:probe -- loopback --directory $run
```

`loopback` creates new fixture/output directories and writes `report.json`. It tests partial transfer, session recreation with disk recheck, SHA-256, corrupted-piece recovery, magnet metadata exchange, and exact-piece-boundary torrent creation. Its successful exit means the transport checks completed; inspect the separate creator check and `candidate_approved` field. It does not approve an engine or certify two-computer connectivity.

The upstream creator boundary diagnostic still records the defect in librqbit 9.1.0. Fixture creation now uses the Game4 streaming BEP 3 adapter in `src/metainfo.rs`, which passes exact/partial boundaries, multi-file hashing across file boundaries, trailing-empty-file, and deterministic encoding regressions. A real exact-boundary transfer and an independent Python metainfo/hash check passed. The report distinguishes the upstream `passed: false` diagnostic from `adapter_passed: true`; this is not production engine approval. No registry source is patched.

Use [the two-computer procedure](../../docs/torrent-validation.md) for separate seeder/downloader processes and future physical LAN testing. `seed` runs for a bounded duration or until Ctrl+C. `download` exits on completion, a requested partial threshold, or timeout. `--resume` is explicit and requires the fixture marker in the destination. Reports are printed as JSON; errors exit nonzero and retain partial data.

Invalid transfer thresholds and zero timeouts are rejected before output creation. Resume markers are retained for legitimate retries. Empty datasets and filesystem links/reparse points are rejected by the creator; empty files inside nonempty collections are supported.

Only use trusted, self-generated fixtures in this experimental tool. It is not a general-purpose torrent importer, security audit, production transfer manager, or anonymous sharing system. See [the evaluation record](../../docs/torrent-engine-evaluation.md).
