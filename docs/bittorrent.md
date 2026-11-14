# Desktop BitTorrent transfers

Implemented November 13, 2026 as an experimental desktop increment. The app embeds pinned `librqbit 9.1.0` with Rust TLS; it does not shell out to a torrent client. The catalog remains independent of engine state. This increment does not close the production adoption or direct-WAN acceptance gates in [the engine evaluation](torrent-engine-evaluation.md).

## Create and share

1. Import an original file or directory into Library and finish its SHA-256 task.
2. Open **Transfers**, select that collection and choose **Create .torrent**. The native save dialog writes new standard BitTorrent v1 metainfo, never overwriting an existing file. Cancellation writes nothing. The app also displays its magnet reference.
3. Choose a listening TCP port (0 selects an available port), then **Start seeding**. The app checks the catalog SHA-256 again in a background worker and the engine verifies pieces before serving them.
4. Send the `.torrent` and reachable IPv4 address plus the displayed port to the other participant. Use a private LAN or your configured Tailscale network for the initial experiment.
5. Choose **Stop** to release the session and original handles.

Creation is offline. Neither creation nor Library import starts sharing. No `.g4` conversion or payload staging copy is required. Existing `.g4e` companion torrents use the same original-file layout and can be imported here; `.g4e` reading itself remains offline, without automatic session creation.

## Import and download

1. Choose **Import .torrent** in Transfers. Review name, info hash, exact file list and total size. This step is offline and opens no listener.
2. Enter an initial peer as `IPv4:port`, then **Choose destination and download**. Select a destination parent in the native picker. Cancellation starts nothing.
3. The app creates a new `game4-download-<UUID>` child directory and writes only new files there. The torrent's display root is not an extra directory; its listed relative file paths appear inside that child.
4. Watch verified piece bytes, uploaded bytes and status. Downloads can upload acquired pieces while active. A completed download is flushed to disk and its session stops automatically. Downloads are not automatically imported into Library; import them explicitly for catalog SHA-256 and preservation metadata.

BitTorrent v1 validates SHA-1 piece hashes; it does not authenticate the publisher. A plain `.torrent` does not contain the catalog SHA-256. For full preservation verification use the trusted `.g4e`/manifest reference and explicit SHA-256 verification as described in [Ever manifests](ever-manifests.md). The automated transfer test independently compares source and received SHA-256.

## Connectivity and lifecycle

DHT, trackers, local service discovery and automatic UPnP mapping are disabled in this increment. Downloads require an explicit initial IPv4 peer. Public torrents retain the engine's peer exchange behavior, so an initial peer can introduce additional peers. Active sessions accept incoming IPv4 TCP connections and expose normal BitTorrent peer information. There is no Game4 relay, mandatory encryption, DRM or central account.

History and transfer state live only in memory. No session starts at app launch. Stop preserves partial download files; this increment does not resume them or automatically delete them. Closing the app cancels all engine sessions. Restart clears the session list and preview tokens, without removing received files. Pause/resume, persisted queues, speeds, bandwidth controls, magnet metadata acquisition, discovery and direct-WAN/NAT validation remain future work. Port/firewall/VPN reachability must already be configured by participants.

## Boundaries

- Public v1 metainfo only; private, v2/hybrid and attributed/padding layouts are rejected explicitly rather than silently reinterpreted.
- At most 8 MiB metainfo, 1,024 files and 1 TiB payload; piece length must be a supported power of two between 16 KiB and 16 MiB. Empty files are supported inside nonempty collections; zero-total torrents are rejected.
- Strict bounded bencode and safe portable paths: absolute paths, traversal, duplicate/case-conflicting paths, file/directory collisions and redirected source ancestors are rejected.
- Maximum four active/stopping sessions, 64 session records per run, eight peers per session and 16 offline preview tokens valid for five minutes.
- A separate handle-only storage adapter enforces read-only sources. It rejects engine write, resize and delete operations. On Windows, original handles also deny concurrent write/delete sharing until stopped. Downloads use `create_new` handles in a freshly owned directory and cannot overwrite existing files.
- Preparation and hashing run on blocking workers, with async session setup and short snapshot locks. Existing import/verification work prevents conflicting source preparation. Stop is independent of the startup gate. UUID request replay returns the accepted transfer; reuse with different parameters fails.

## Contracts and validation

The capability-gated IPC commands are `torrent_create`, `torrent_import`, `torrent_seed`, `torrent_download`, `transfers_list` and `transfer_stop`. Paths are selected natively and are not supplied by the webview. Responses expose filenames/relative paths, opaque preview and request IDs, exact decimal byte strings and runtime session summaries. Structured errors add `TORRENT_INVALID`, `TRANSFER_LIMIT_EXCEEDED`, `TRANSFER_FAILED` and `TRANSFER_UNAVAILABLE` to contract v1. Generated Rust/TypeScript DTOs and runtime decoders keep both boundaries aligned.

Run `npm.cmd run check`, `npm.cmd run contracts:check`, `npm.cmd run rust:test` and Cargo Clippy with `--all-targets -- -D warnings`. Native tests exercise actual loopback engine seeding/downloading, multi-file Unicode and empty-file handling, independent SHA-256, request replay, offline previews, source write/resize/delete refusal and handle release after Stop. These tests do not replace physical-machine or independent-client acceptance of this desktop integration. Earlier two-PC probe evidence is recorded separately in [torrent validation](torrent-validation.md).
