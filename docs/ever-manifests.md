# Creating, reading and verifying .g4e manifests

Status: native Library create/read and offline Rust CLI implemented November 13, 2026. This delivers reference files and verification, not an integrated network transfer manager.

## Library workflow

**Export originals .g4e** on a Library item checks the original source tree and generates a `.g4e` plus a uniquely named `.torrent` companion. It also displays the magnet link. Originals stay at the paths recorded during import; the app makes no `.g4`, staging copy, conversion or original-file write. SHA-256 and exact size must still match the catalog. A changed, missing, redirected or locked source fails instead of refreshing hashes or repairing files.

The item’s recorded import root must still exist. Single-file imports retain their filename; directories use multi-file torrent layout with their basename as the advisory root. Arbitrary content copies elsewhere are not substituted into a layout that would require moving them for an ordinary BitTorrent client. Restore retains source-root records, but files still need to be present and checked. Absolute source mappings are never serialized into the descriptor or companion.

The generated torrent is standard trackerless BitTorrent v1, with 1 MiB pieces, SHA-1 of concatenated source bytes in torrent file order and SHA-1 of the exact encoded info dictionary. Per-file preservation integrity remains SHA-256. Piece state crosses file boundaries and includes trailing/empty files without inventing a final empty piece. Local generation is bounded by the public descriptor limits and an 8 MiB companion metainfo limit; unsupported very large layouts fail before full hashing when predictable.

A user can distribute those metadata files/magnet and seed the existing files with a suitably configured BitTorrent client. The recipient still needs an active source and a reachable network path. Export does not start a torrent session, contact a tracker/DHT, open ports, or automatically upload anything. The external client's source/write behavior is its own responsibility; Game4∞ itself treats originals as read-only. Integrated Game4∞ seeding/downloads, private source mapping for dispersed roots and discovery remain open P2P work.

**Open .g4e** uses a native picker, validates strict UTF-8 JSON, version, schema/business rules, path/size bounds and magnet identities, then displays representation, counts and magnets. It does not dereference companions, verify original files or connect to peers. Its IPC summary deliberately uses `contentVerified:false`. Reference fields remain inert text, including magnet values; no executable or URL is launched automatically. Both workflows run in blocking workers with UI busy feedback and shared idle-library coordination for creation.

## Offline CLI

```powershell
npm.cmd run g4e -- create --manifest "C:\Collection\manifest.json" --root "C:\Collection" --output "C:\Exports\share.g4e"
npm.cmd run g4e -- create-package "C:\Exports\collection.g4" --output "C:\Exports\package-share.g4e"
npm.cmd run g4e -- read "C:\Exports\share.g4e"
npm.cmd run g4e -- verify --manifest "C:\Exports\share.g4e" --torrent "C:\Exports\game4-UUID.torrent" --root "C:\Collection"
```

`create` accepts an existing preservation manifest plus its original directory or single file. `create-package` explicitly references an existing, fully verified `.g4`; it does not create another package. `read` validates the descriptor only and prints its JSON. `verify` requires explicit descriptor, torrent and source arguments; it checks companion SHA-256/length when declared, exact bencoded info-hash, regular safe torrent paths, complete path/size binding, all piece digests and each source SHA-256. Packaged mode also checks the complete `.g4` and collection UUID binding. No CLI operation uses the network. Local verifier piece lengths must be powers of two from 16 KiB to 16 MiB; larger unsupported layouts fail explicitly.

Independent public-schema validation remains `npm.cmd run g4e:validate -- PATH.g4e`. It uses the separate Node implementation; full byte verification is the explicit Rust `verify` operation. A successful descriptor parse is not a claim of payload authenticity or availability.

## Publication and failure handling

Files are written as temporary siblings, synced and published without clobbering. The companion is published before the descriptor, whose relative reference contains only the new companion filename and whole-file hash. Both are prepared before publication; an already existing descriptor is rejected early. A late descriptor-publication failure can leave the newly published torrent companion. The app reports failure and does not delete paths after a possible race. Abrupt OS/process interruption may leave temporary files or a companion. Original payloads are never cleanup targets.

Library mutations/restore are excluded while the source snapshot and creation run; database locks are not held during payload reads. A failed source check does not update catalog reference hashes or stored copy-health observations. SQLite schema v4, private backup v2, `.g4` v1 and `.g4e` v1 remain unchanged.

## Compatibility and integrity evidence

Rust reads both independent Python `.g4e`/torrent examples and rejects the shared 21-document malformed corpus also used by Node. Tests cover strict duplicate keys/Unicode/versioning, original and optional package representations, multi-file piece boundaries, exact piece ends, empty/trailing-empty files, stale/missing/locked sources, unchanged source bytes/mtime, no staging payload, no-clobber output, companion/info/payload hash domains and package collection UUID mismatches. Existing `.g4` ZIP32/ZIP64 and preservation-workflow tests remain in the main suite.

`tools/torrent-probe/examples/inspect_metainfo.rs` offers a parser-only check using the independent evaluated librqbit implementation. It never creates a session; successful parsing alone does not certify a transfer or adopt that engine for the desktop app. End-to-end source transfer, native WAN/NAT and production engine gates remain open. See [the public specification](../packages/g4e-format/spec-v1.md).
