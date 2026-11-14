# Desktop BitTorrent transfers

Implemented November 13, 2026, with magnet import and optional DHT added November 14, as an experimental desktop increment. The app embeds pinned `librqbit 9.1.0` with Rust TLS; it does not shell out to a torrent client. The catalog remains independent of engine state. This increment does not close the production adoption or direct-WAN acceptance gates in [the engine evaluation](torrent-engine-evaluation.md).

## Create and share

1. Import an original file or directory into Library and finish its SHA-256 task.
2. Open **Transfers**, select that collection and choose **Generate magnet link** to obtain a reference without writing a file, or **Create .torrent** to export metainfo. The native save dialog writes new standard BitTorrent v1 metainfo, never overwriting an existing file. Cancellation writes nothing. The app also displays its magnet reference.
3. Choose a listening TCP port (0 selects an available port), optionally enable **DHT for the next network action**, then **Start seeding**. The app checks the catalog SHA-256 again in a background worker and the engine verifies pieces before serving them.
4. Send the magnet or `.torrent` to the other participant. With DHT disabled, also provide a reachable IPv4 address plus the displayed port. With DHT enabled, the session searches for peers and announces its listening port on the public DHT. Use a private LAN or your configured Tailscale network for the initial experiment.
5. Choose **Stop** to release the session and original handles.

Creation is offline. Neither creation nor Library import starts sharing. No `.g4` conversion or payload staging copy is required. Existing `.g4e` companion torrents use the same original-file layout and can be imported here; `.g4e` reading itself remains offline, without automatic session creation.

## Import and download

1. Choose **Import .torrent** in Transfers. Review name, info hash, exact file list and total size. This step is offline and opens no listener.
2. Enter an initial peer as `IPv4:port`, or enable **DHT** and leave the peer empty, then **Choose destination and download**. Select a destination parent in the native picker. Cancellation starts nothing.
3. The app creates a new `game4-download-<UUID>` child directory and writes only new files there. The torrent's display root is not an extra directory; its listed relative file paths appear inside that child.
4. Watch verified piece bytes, uploaded bytes and status. Downloads can upload acquired pieces while active. A completed download is flushed to disk and its session stops automatically. Downloads are not automatically imported into Library; import them explicitly for catalog SHA-256 and preservation metadata.

BitTorrent v1 validates SHA-1 piece hashes; it does not authenticate the publisher. A plain `.torrent` does not contain the catalog SHA-256. For full preservation verification use the trusted `.g4e`/manifest reference and explicit SHA-256 verification as described in [Ever manifests](ever-manifests.md). The automated transfer test independently compares source and received SHA-256.

## Import a magnet

Paste a public BitTorrent v1 magnet into Transfers. Select optional DHT discovery, or enter an initial reachable seeder, then choose **Fetch magnet metadata**. This explicit action starts network connections; pasting alone does not. The lookup lasts at most 60 seconds plus session cleanup and offers **Cancel metadata lookup**. Leaving the page requests cancellation, and app exit cancels lookup children as well.

The resolver obtains BEP 9 metadata only, stops its temporary sessions, verifies the exact info hash against the link, and applies the same strict path/layout/resource checks as `.torrent` import. It never creates payload files or adds a payload transfer at this stage. Review the resulting file list and choose a destination explicitly to start downloading. Preview tokens expire after five minutes, so repeat the lookup if necessary. No external URL handler or OS protocol association is installed.

One `urn:btih` identity is required, accepting 40 hexadecimal characters or 32 base32 characters and returning canonical lowercase hexadecimal. Duplicate identities and v2/hybrid identities are rejected. `dn`, `tr` and `xl` are discarded; web seeds, exact/acceptable sources, embedded peers and other parameters are rejected. Neither tracker URLs nor HTTP sources from imported magnets are followed. Generating a magnet from a Library source checks its SHA-256 in a background worker and creates no payload copy, `.g4` or `.torrent` file; it does not start sharing. Runtime session cards also expose their canonical magnet reference.

## Connectivity and lifecycle

DHT is opt-in per network action; it is off on app launch and changing the checkbox does not reconfigure running sessions. Enabling it starts an ephemeral UDP DHT node with the engine's public bootstrap defaults (`dht.transmissionbt.com:6881` and `dht.libtorrent.org:25401`). There is no persisted routing table, central Game4 directory or mandatory account. Active sessions query and announce the info hash and their TCP listening port. Metadata-only lookups query without announcing a payload listener. Public DHT traffic exposes network address and requested hashes to participating nodes.

Trackers, local service discovery and automatic UPnP mapping remain disabled. Downloads can use DHT with no initial peer, an explicit initial IPv4 peer with DHT off, or both. Public torrents retain the engine's peer exchange behavior. DHT discovers peers; it does not make peers behind NAT/CGNAT or firewalls reachable. A reachable seeder is still required. Active payload sessions accept incoming IPv4 TCP connections. There is no Game4 relay, mandatory encryption, DRM or central account.

History and transfer state live only in memory. No session starts at app launch. Stop preserves partial download files; this increment does not resume them or automatically delete them. Closing the app cancels all engine sessions. Restart clears the session list and preview tokens, without removing received files. Pause/resume, persisted queues, speeds, bandwidth controls, tracker/local discovery and direct-WAN/NAT validation remain future work. Port/firewall/VPN reachability must already be configured by participants.

## Boundaries

- Public v1 metainfo only; private, v2/hybrid and attributed/padding layouts are rejected explicitly rather than silently reinterpreted.
- At most 8 MiB metainfo, 1,024 files and 1 TiB payload; piece length must be a supported power of two between 16 KiB and 16 MiB. Empty files are supported inside nonempty collections; zero-total torrents are rejected.
- Strict bounded bencode and safe portable paths: absolute paths, traversal, duplicate/case-conflicting paths, file/directory collisions and redirected source ancestors are rejected.
- Maximum four active/stopping payload sessions, 64 session records per run, eight payload peers per session and 16 preview tokens valid for five minutes. One metadata lookup can run concurrently. It uses separate ephemeral discovery and metadata-only sessions: one metadata peer at a time, at most 128 candidate peers, eight seconds per attempt and a 60-second total lookup deadline. The upstream receiver bounds each metadata buffer at 32 MiB; Game4 accepts at most 8 MiB of final metainfo. No payload storage is created before preview acceptance.
- A separate handle-only storage adapter enforces read-only sources. It rejects engine write, resize and delete operations. On Windows, original handles also deny concurrent write/delete sharing until stopped. Downloads use `create_new` handles in a freshly owned directory and cannot overwrite existing files.
- Preparation and hashing run on blocking workers, with async session setup and short snapshot locks. Existing import/verification work prevents conflicting source preparation. Stop is independent of the startup gate. UUID request replay returns the accepted transfer; reuse with different parameters fails.

## Contracts and validation

The capability-gated IPC commands are `torrent_create`, `torrent_import`, `torrent_seed`, `torrent_download`, `transfers_list`, `transfer_stop`, `magnet_generate`, `magnet_resolve` and `magnet_cancel`. Paths are selected natively and are not supplied by the webview. Responses expose filenames/relative paths, opaque preview and request IDs, exact decimal byte strings and runtime session summaries. Structured errors add `TORRENT_INVALID`, `TRANSFER_LIMIT_EXCEEDED`, `TRANSFER_FAILED` and `TRANSFER_UNAVAILABLE` to contract v1. Generated Rust/TypeScript DTOs and runtime decoders keep both boundaries aligned. Seed/download requests add optional `dht` (default false for older callers); snapshots report the choice. The new `magnet_resolve` capability also gates DHT requests in the client, so older v1 backends cannot silently ignore DHT. Missing `dht` in older snapshots decodes as false. Metadata resolution takes a UUID request ID for targeted cancellation, a bounded magnet, optional peer string and explicit DHT boolean, returning the existing preview/cancelled union.

Run `npm.cmd run check`, `npm.cmd run contracts:check`, `npm.cmd run rust:test` and Cargo Clippy with `--all-targets -- -D warnings`. Native tests exercise actual loopback engine seeding/downloading, multi-file Unicode and empty-file handling, independent SHA-256, request replay, offline previews, source write/resize/delete refusal and handle release after Stop. Additional tests resolve real magnet metadata without payload writes, cancel blocked lookups and perform actual UDP DHT announcements, discovery, magnet resolution and SHA-256-checked transfer without an initial peer through two isolated local bootstrap nodes. No public DHT is needed by this test. These tests do not replace physical-machine or independent-client acceptance of this desktop integration. Earlier two-PC probe evidence is recorded separately in [torrent validation](torrent-validation.md).
