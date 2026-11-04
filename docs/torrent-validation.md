# Reproducing torrent validation

Status: the local checks have passed, with a separate creator defect recorded in [the evaluation](torrent-engine-evaluation.md). **Two physical computers have not been tested.** This procedure is ready for when the second PC becomes available.

## Build on each test PC

Use the same repository revision on both Windows PCs, with Rust MSVC and C++ Build Tools available. From the repository root:

```powershell
cargo build --locked --manifest-path tools/torrent-probe/Cargo.toml
$probe = (Resolve-Path .\tools\torrent-probe\target\debug\game4-torrent-probe.exe).Path
```

No desktop application or SQLite database is needed. Use trusted synthetic fixtures only. Do not use personal folders as download destinations. The tool refuses a nonempty output directory unless `--resume` is explicit and its fixture marker matches.

## PC A: prepare the synthetic collection

```powershell
$kit = Join-Path $env:TEMP ("game4-kit-" + [guid]::NewGuid().ToString("N"))
& $probe prepare --directory $kit
```

This creates `seed/payload.bin`, `fixture.torrent`, and `fixture.json`. Transfer **only `fixture.torrent` and `fixture.json`** to a directory on PC B, for example `$env:TEMP/game4-metadata`. Do not copy `seed/payload.bin` to PC B: the payload must arrive through BitTorrent for this to be a meaningful test. Keep the expected hash from PC A for independent comparison.

## PC A: seed on the private LAN

Find PC A's private IPv4 address for the intended LAN. Replace the example address below with that address; do not use `127.0.0.1` for a two-PC test.

```powershell
& $probe seed --directory $kit --listen 192.168.1.10:42420 --seconds 1800
```

Wait for `Seed ready`. Keep this terminal open. The command stops after 30 minutes or Ctrl+C. If Windows asks for firewall permission, limit access to the intended private network. An existing firewall policy must allow inbound TCP 42420 on PC A; the probe does not change it. Do not disable the firewall. Routers with client isolation may prevent communication even on the same Wi-Fi.

## PC B: verify connectivity, interrupt, and resume

In the following commands, `$kit` is the metadata-only directory on PC B. Use PC A's actual LAN address.

```powershell
$kit = Join-Path $env:TEMP "game4-metadata"
$out = Join-Path $env:TEMP ("game4-download-" + [guid]::NewGuid().ToString("N"))
$peer = "192.168.1.10:42420"
Test-NetConnection 192.168.1.10 -Port 42420
& $probe download --directory $kit --output $out --peer $peer --stop-after-bytes 524288 --download-bps 262144
& $probe download --directory $kit --output $out --peer $peer --resume
Get-FileHash (Join-Path $out "payload.bin") -Algorithm SHA256
```

The first download must stop with `completed: false` and positive progress. The second invocation is a new OS process; it must report positive `resumed_bytes`, finish, and return `sha256_matches: true`. Compare `Get-FileHash` with the independently recorded expected hash from PC A. A TCP port check alone does not validate a torrent transfer.

For magnet metadata exchange, use a fresh destination:

```powershell
& $probe download --directory $kit --output ($out + "-magnet") --peer $peer --magnet
```

The engine receives the magnet, not `.torrent` bytes, in this mode. The probe still reads the local metainfo to validate the expected fixture and magnet before networking.

Save command output and fill [the result template](evaluations/two-computer-result-template.md). Errors exit nonzero; partial data is retained. Resume here means disk recheck after a new session/process, not an automatic saved transfer queue. Hard-kill/power-loss behavior is a separate test.

## Local repeat without a second PC

```powershell
$run = Join-Path $env:TEMP ("game4-loopback-" + [guid]::NewGuid().ToString("N"))
npm.cmd run torrent:probe -- loopback --directory $run
Get-Content (Join-Path $run "report.json")
```

This runs two engine sessions on one host, checks repair and magnets, and records the creator boundary defect. A successful command means transport checks passed; it does not override `candidate_approved: false` or the creator result. Choose a fresh directory for every run; no recursive cleanup is performed by the probe.

## Boundaries

Discovery services and automatic port mapping are disabled. This guide validates direct private-LAN TCP only. For different networks, first design an explicit reachable-peer topology and assess IP exposure; this guide does not solve NAT or CGNAT. Do not present local results as two-PC, WAN, anonymous, or permanent-storage validation.
