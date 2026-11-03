# Planned security

This document describes intended controls, not an audit or a claim of implemented security.

| Threat | Required control |
|---|---|
| Malicious paths and destination escape | Validate relative paths, Windows special paths, and actual destination; block traversal and unsafe redirection |
| Oversized or malformed packages | Bound size, count, depth, and decompression; parse defensively |
| Executable content | No automatic execution; explicit opening actions |
| Tampered manifests | Validate schemas and hashes; future signatures with trust policy |
| Disk or memory exhaustion | Chunked reads, quotas, space checks, and cancellation |
| Corruption or interruption | Persistent states, temporary writes where applicable, and crash recovery |
| Excessive interface privileges | Minimal Tauri commands and Rust validation; avoid privileged remote HTML |
| Unintended network exposure | Explicit sharing and notice that peers may see the user's IP |
| Logs and exports leaking data | Remove secrets and avoid personal paths by default |
| Compromised dependencies | Lockfiles, dependency review, and release checks |

## Before the P2P alpha

Check malicious names and paths, cancellation, changes during hashing, disk exhaustion, interrupted writes, invalid manifests, restart, and corrupted transfers. Review ports, DHT, trackers, and background sharing. Do not promise anonymity. Both supported languages must explain security-sensitive actions consistently.

## Data and rights

Metadata may contain personal information; export and publication must be deliberate. The software license does not authorize collection redistribution. Use synthetic, owned, or redistributable content in tests and demos.

## Vulnerability reporting

A private channel is not defined yet. Before public distribution, create `SECURITY.md` with a verified contact and response process. Do not invent a support address or ask for secrets in public issues.
