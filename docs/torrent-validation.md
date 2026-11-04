# Reproducing torrent validation

Status: local transport checks and corrected-creator regressions have passed; the unmodified upstream creator defect remains documented in [the evaluation](torrent-engine-evaluation.md). **Two physical Windows PCs on different networks passed transfer, new-process resume, independent SHA-256, and magnet through Tailscale DERP relay.** See [the result record](evaluations/two-computer-result-2026-11-04.md). The procedure below is retained for reproduction.

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

## Two different networks: current VPN test plan

The owner chose Tailscale. Install it on both Windows PCs, sign into the same Tailscale account/network, and confirm both devices are online. No matching Windows account is required. Do not share passwords, auth keys, or login links in committed reports.

On each machine, Tailscale's `ip -4` command shows its overlay IPv4 address. From PC B, `tailscale ping <PC-A-overlay-IP>` checks the path; record whether it is direct or relay. A relayed connection is a valid overlay transport test but not proof of a direct native WAN BitTorrent connection.

Use the same seed/download commands above with PC A's overlay address in `--listen` and `--peer`, instead of its LAN address. This test does not require router port forwarding. If an app-specific firewall exception is needed, restrict it to the probe executable, TCP 42420, and the second device's overlay address; do not disable the firewall or open it to all remote addresses. Remove any temporary rule after the test.

The optional portable test package contains the probe, its source/license notices, and metadata only; the payload remains on PC A so PC B must receive it via BitTorrent. No account credentials are included.

References: [Windows installation](https://tailscale.com/docs/install/windows), [device connections](https://tailscale.com/docs/how-to/connect-to-devices), and [direct/relay connection types](https://tailscale.com/docs/reference/connection-types).

## Local repeat without a second PC

```powershell
$run = Join-Path $env:TEMP ("game4-loopback-" + [guid]::NewGuid().ToString("N"))
npm.cmd run torrent:probe -- loopback --directory $run
Get-Content (Join-Path $run "report.json")
```

This runs two engine sessions on one host, checks repair and magnets, and records both the upstream creator defect and the corrected adapter result. A successful command means transport checks passed; it does not override `candidate_approved: false` or the creator result. Choose a fresh directory for every run; no recursive cleanup is performed by the probe.

## Boundaries

Discovery services and automatic port mapping are disabled. The LAN procedure targets direct private-LAN TCP; the separate VPN procedure records overlay transport, including relay. For different networks, the current plan uses an explicit VPN topology; this does not validate the engine's native NAT or CGNAT traversal. Do not present local results as two-PC, WAN, anonymous, or permanent-storage validation.
