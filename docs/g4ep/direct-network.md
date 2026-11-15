# G4EP direct private networking — protocol v1

Status: implemented manual private discovery and direct fragment exchange. Public DHT discovery remains future work. This adapter implements `PeerDiscovery` and `FragmentTransfer` and composes the existing Guardian `StorageNode`. It does not change `.g4e` schema v2, `.g4`, `.g4lib`, the catalog database or existing BitTorrent sessions.

## Architecture and reuse

The existing BitTorrent engine provides general game/torrent sessions but does not establish enrolled Guardian identities or per-fragment admission by itself. Routing Guardian storage through it would not automatically enforce the private boundary. The direct adapter reuses Tokio, the project's Rustls stack, cancellation contexts, protected source opening, Guardian permissions/atomic storage and aggregate bandwidth gates. There is no invented cryptographic handshake: Rustls negotiates mutually authenticated TLS 1.3. Later DHT discovery may supply candidate endpoints, but must not confer trust or bypass certificate enrollment.

There is no transfer broker, relay or central registry. No DNS lookup, public DHT, tracker, PEX, UPnP, broadcast or multicast announcement is enabled. Bind and peer endpoints must be canonical literal private IP sockets: loopback, RFC1918, link-local, IPv6 ULA or CGNAT 100.64/10 such as a private overlay. Wildcard/public/zero-port endpoints, duplicates, self-enrollment and more than 32 peers are rejected. Loopback works only on one computer; separate private networks require an independently configured route such as the already chosen Tailscale network. The app does not open firewall exceptions or configure routers automatically.

## Identity and explicit enrollment

A node retains its existing storage node ID and generates a persistent local self-signed TLS identity. TLS private keys are stored separately from fragments under the app's private `guardian-identities` directory; the CLI requires an explicit private key directory. Restrict filesystem access to that directory. This uses normal local filesystem protection, not a Windows credential vault. Never share private identity files or the complete key directory.

Public peer profile v1 contains `version`, `nodeId`, `address`, `certificateSha256` and lowercase hex `certificateDer`. It contains the certificate and endpoint only, never the private key. Exporting it does not start networking. Exchange public profiles through a trusted channel and compare certificate fingerprints independently. Each side explicitly enrolls the other. Changing a certificate or deleting/replacing the private identity requires explicit re-enrollment. A manifest node ID/address alone cannot authenticate a peer.

Rustls validates the peer chain/signature against the enrolled certificate and the fixed SAN `guardian.game4.local`, without resolving that name. Client and server both prove key possession. TLS 1.3 and ALPN `game4-g4ep-private/1` are required; early data is disabled. The application additionally checks the exact leaf-certificate SHA-256, enrolled node ID and, inbound, the observed source IP against the enrolled endpoint's IP. The source TCP port may be ephemeral. Identity does not establish publisher copyright, legal authority or long-term fragment durability.

Primary library references: [Rustls verifier API](https://docs.rs/rustls/latest/src/rustls/verify.rs.html) and [rcgen certificate creation](https://docs.rs/rcgen/latest/rcgen/fn.generate_simple_self_signed.html). Dependencies are pinned in Cargo.lock; no custom signature verification bypass is used.

## Discovery and availability

Manual discovery probes the explicitly enrolled IP:port and returns a peer only after authenticated protocol exchange. This is an online observation, not guaranteed future availability. Peers are not learned recursively from strangers.

Availability announcements are request-scoped responses for up to 100 exact fragment descriptors. The node checks sharing permission, size and stored SHA-256 before answering true. Unknown, unapproved, missing or corrupt fragments are not offered. The protocol never lists the complete local catalog, filenames, private paths or arbitrary disk files. No background inventory broadcast occurs. A stale positive response cannot replace hash verification during transfer.

## Transfer and resource controls

Both nodes must authorize the same `.g4e` coordinate/size/hash. To send, the source must hold intact bytes and allow sharing. To receive, the target must authorize storage and have quota. A peer cannot create admissions, delete files or change local settings. Received files pass through Guardian staging, exact-length SHA-256 verification, sync and atomic no-overwrite publication. Remote upload acknowledgement follows that check and commit; it is still a remote custody claim rather than proof of permanent preservation.

The integrated fetch/push paths stream through 64 KiB duplex bridges into blocking storage workers. Existing upload and download limits apply across local and network operations in each direction. TCP/TLS/OS buffers permit a bounded initial burst; limits govern fragment payload, not control packets or exact wire overhead. Generic transfer-port calls use a bounded replay buffer of at most one 4 MiB fragment, verify before output, and use the same limits. They never buffer an entire ROM.

A semaphore bounds combined incoming and outgoing operations (default 4, configurable 1–16). Saturated inbound sockets are closed; excess local operations fail rather than grow an unbounded queue. Before TLS, inbound source IP must match an enrolled private peer. Handshakes and application operations have finite deadlines. Connect timeout is capped at 5 seconds, handshake at 10 seconds and each attempt at the configured 2–600 seconds. Caller cancellation and overall deadline take precedence. Stalled operations release their slot and cannot publish partial data.

Retries are configurable from 0 to 3 after the first attempt (at most four attempts), with bounded exponential delays of 100, 200 and 400 ms. Connection I/O failures and timeouts may retry. Hash mismatch, enrollment/authentication rejection, invalid protocol, disabled participation and quota/permission rejection do not trigger admission-changing fallbacks. A retry restarts the fragment; it does not claim byte-resume support. If an acknowledgement is lost after remote commit, the retry validates/deduplicates the exact existing content.

Stopping the network cancels its active operations and closes the listener. Disabling Guardian participation also stops serving; existing bounded readers/writers observe cancellation/activity checks. Already delivered bytes cannot be recalled. The network and Guardian participation reopen stopped/disabled after app restart; saved profiles/settings do not reconnect automatically.

## Desktop workflow for two Windows computers

1. In Settings → Guardian Node, configure storage/quota/limits and authorize the same group/index from the same valid `.g4e` v2 file on each computer. On the source, allow sharing and store its `.g4frag`. The receiver needs only that permission and free quota.
2. In Direct Guardian connections, set each computer's actual LAN or Tailscale IP plus an available TCP port, e.g. `192.168.1.10:42430` and `192.168.1.11:42430`. Save while stopped.
3. Save each public profile and exchange those files through a trusted channel. Compare the shown certificate SHA-256 fingerprints. On each computer, choose the other computer's public profile. An optional peer address override changes routing while retaining the pinned certificate.
4. Enable Guardian participation for the session, then start the private network on both computers. The chosen inbound TCP ports must be reachable through your existing network/firewall setup.
5. Choose an authorized fragment and test the peer connection. Check availability, then Receive fragment or Send fragment. The result reports attempts and the verified byte count/remote acknowledgement. Refresh/check stored fragments to audit local integrity.
6. Stop network and transfers when finished. Stop before changing peers, listener or network limits. Removing a peer prevents its use in the next session; it does not delete any fragment. Revoking a fragment's sharing permission is a separate storage control.

The fragment selector uses the current permission page (100 entries), not a hidden scan of the full library. Empty peer and inventory states are explicit. TLS trust failures, timeouts, quotas and connection limits have separate localized errors. Native file dialogs choose public profiles; no terminal commands, secrets or private key paths are required in the UI.

## CLI workflow

Use absolute store and **private** key directories. Do not point the CLI at a store currently open in the app: the exclusive ownership lock intentionally refuses that. Use the same binaries/version on both computers. An existing valid manifest and fragment are inputs, not downloaded by these commands.

On PC A (adapt the IP):

```powershell
npm.cmd run guardian -- "C:/GuardianA" approve "C:/Test/preservation.g4e" 0 0 --share
npm.cmd run guardian -- "C:/GuardianA" import 1 "C:/Test/fragment.g4frag" --enable
npm.cmd run guardian:network -- "C:/GuardianA" "C:/PrivateKeysA" configure 192.168.1.10:42430 4 120 2
npm.cmd run guardian:network -- "C:/GuardianA" "C:/PrivateKeysA" profile 192.168.1.10:42430 "C:/Test/pc-a-public.json"
```

On PC B:

```powershell
npm.cmd run guardian -- "C:/GuardianB" approve "C:/Test/preservation.g4e" 0 0
npm.cmd run guardian:network -- "C:/GuardianB" "C:/PrivateKeysB" configure 192.168.1.11:42430 4 120 2
npm.cmd run guardian:network -- "C:/GuardianB" "C:/PrivateKeysB" profile 192.168.1.11:42430 "C:/Test/pc-b-public.json"
```

Exchange **only** the public profiles. Enroll B's profile on A and A's profile on B:

```powershell
# PC A
npm.cmd run guardian:network -- "C:/GuardianA" "C:/PrivateKeysA" enroll "C:/Test/pc-b-public.json"
# PC B
npm.cmd run guardian:network -- "C:/GuardianB" "C:/PrivateKeysB" enroll "C:/Test/pc-a-public.json"
```

On PC A, keep the source listener running:

```powershell
npm.cmd run guardian:network -- "C:/GuardianA" "C:/PrivateKeysA" serve --enable
```

On PC B, use the nodeId from A's public profile (public, not a password):

```powershell
$peer = (Get-Content "C:/Test/pc-a-public.json" -Raw | ConvertFrom-Json).nodeId
npm.cmd run guardian:network -- "C:/GuardianB" "C:/PrivateKeysB" probe $peer --enable
npm.cmd run guardian:network -- "C:/GuardianB" "C:/PrivateKeysB" available $peer 1 --enable
npm.cmd run guardian:network -- "C:/GuardianB" "C:/PrivateKeysB" fetch $peer 1 --enable
npm.cmd run guardian -- "C:/GuardianB" verify
```

For push, run the receiver's listener and invoke `push PEER_ID ENTRY_ID --enable` on the source. CLI listeners are stopped with Ctrl+C. `guardian_net.exe` accepts the same arguments directly when testing a copied binary without Node/Cargo. Entry IDs come from local Guardian status and need not match across computers.

## Acceptance and remaining scope

Automated tests use real TCP sockets and mutual TLS, including wrong certificates, unknown/unapproved fragments, quota rejection, truncated/corrupt payloads, retry after refusal, slot limits, timeouts, cancellation and directional throttling. A separate [process-level evaluation](../evaluations/g4ep-direct-network-2026-11-14.json) records push/fetch of an authorized 1 MiB shard from the retained synthetic 700 MB fixture between independent Windows CLI processes.

Loopback tests establish real transport behavior on one machine. They do not establish reachability between two physical machines, NAT traversal or firewall readiness. Record the actual two-PC addresses/routing conditions and results separately before claiming that acceptance. Public DHT discovery, decentralized candidate discovery, repair scheduling and whole-original reconstruction remain future milestones.
