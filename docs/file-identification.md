# Platform, format, and size identification

Status: implemented in local import, Library summaries, and **View files**. Metadata is computed locally; no online catalog lookup, game execution, archive extraction, or second read pass is performed.

## Evidence and ownership

Identification is an observation about a stored content identity or a filename suggestion. The UI distinguishes **Header signature**, **Extension hint**, and **Unknown**. Neither a matching header nor an extension proves that a complete image is valid, authentic, executable, or licensed. SHA-256 and copy verification remain separate concerns.

Signature observations belong to `(sha256, size_bytes)` content rows. Filename-based hints belong to each relative item path and are computed separately, so identical bytes named `game.gba` and `disc.iso` can receive different hints without corrupting content identity. Header evidence takes precedence over conflicting extensions. A ZIP renamed `.nes` stays ZIP with no NES suggestion. PE headers identify PE; Windows is only suggested when a compatible `.exe`/`.dll` filename is also present. Empty files remain unknown.

Declared `collection_items.platform` is preserved, displayed separately, and never automatically overwritten. Suggested platforms can be mixed within a folder. Generic ISO/CHD/CUE/BIN/ROM/archive names do not imply a console. `.md` is deliberately not treated as a Mega Drive ROM because it commonly names Markdown documents.

## Initial scope

| Evidence            | Recognized cases                                                                                                                                                                                                                                                                                                   |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Header signatures   | iNES/NES 2.0; N64 big-endian, byte-swapped, little-endian; PS-X EXE; Xbox XBE; ZIP local/empty headers; 7z; RAR 4/5; gzip; CHD; ISO 9660 descriptors; PE; ELF; Doom WAD                                                                                                                                            |
| Extension hints     | `.nes`, `.z64`, `.v64`, `.n64`, `.sfc`, `.smc`, `.gb`, `.gbc`, `.gba`, `.nds`, `.3ds`, `.cia`, `.nsp`, `.xci`, `.sms`, `.gg`, `.gen`, `.smd`, `.pce`, `.sgx`, `.adf`, `.d64`, `.exe`, `.dll`, `.elf`, `.xbe`, `.zip`, `.jar`, `.apk`, `.7z`, `.rar`, `.gz`, `.tgz`, `.chd`, `.iso`, `.cue`, `.wad`, `.bin`, `.rom` |
| Suggested platforms | NES, SNES, Game Boy/Color/Advance, Nintendo DS/3DS/Switch/64, Master System, Game Gear, Mega Drive/Genesis, PC Engine, Amiga, Commodore 64, Windows, PlayStation, Xbox, Android                                                                                                                                    |

Filename hints are case-insensitive. Unknown extensions remain unknown. This is an initial detector, not exhaustive ROM/image support. ISO checks the descriptor at byte 32768; PE follows the bounded DOS header offset. Truncated prefixes and out-of-window offsets safely fall back to filename evidence. Known headers are checked for minimum prefix lengths, not fully parsed/checksummed. Encrypted packages, archive contents, game title/region/revision, headers beyond the window, and scheduled reidentification remain future work.

## Reading and persistence

During the existing SHA-256 worker, capture at most the first **64 KiB** while each chunk is already in memory. A small prefix allocation is bounded by file size/window; the import still reuses its 1 MiB hash buffer and does no additional disk read. Metadata commits atomically with content/location mappings and task success. A failed import leaves no classification rows behind.

SQLite migration 0004 adds `file_identifications`, keyed by content ID, with checked format/platform codes and `detector_version=1`. It stores signature results or an explicit unknown result after a read. Extension hints are not persisted as content facts. Existing catalogs migrate without data loss and without opening game files; missing signature rows produce extension-only/unknown output with `detectorVersion:null`. Reimport explicitly recomputes observations. Old content observations remain with their identities when paths change to different bytes.

The detector version is independent of database schema v4, manifest v1, backup v2, IPC v1, and app version. Catalog browsing reads only SQLite/relative names, even if originals are missing. Restored classifications are historical observations for saved hashes; restore still resets location health to unverified. No fresh verification is implied.

## Library and exact sizes

Cards show declared/suggested platforms, format groups and evidence/counts, plus readable IEC sizes (B, KiB, MiB, GiB, …). **View files** expands an item and loads up to 50 entries at a time, showing relative name, format/evidence, platform/evidence, exact bytes, readable size, and SHA-256. Pagination is available through **Load more files**. Errors are localized and native actions stay unavailable in browser preview.

Content sizes remain exact nonnegative SQLite integers. Relative entries count independently, including duplicate-content copies. Totals use Rust u128 arithmetic rather than SQLite's signed-64-bit SUM, and cross JSON as canonical decimal strings. The frontend uses BigInt for unit selection/rounding and locale separators; it never converts the original byte count to Number. Exact bytes remain visible alongside rounded summaries. A summary refuses unexpected catalogs over 500,000 entries per item instead of silently truncating totals.

## Contract and compatibility

`library_list` adds `formats: FormatSummary[]` and `detectedPlatforms: PlatformSummary[]`, keeping declared `platform` and decimal `totalBytes`. Groups contain a stable format/platform ID, evidence, and file count. Older consumers ignore added fields; the current decoder also accepts absent arrays from older backends as empty.

New capability/command `library_item_files` accepts `{request:{itemId:"1",page:{cursor:null,limit:50}}}`. Response `LibraryItemFilesPage` contains `files` and nullable `nextCursor`. `LibraryFileInfo` includes `path`, `sizeBytes`, `sha256`, `format`, `formatEvidence`, nullable `platform`, `platformEvidence`, and nullable `detectorVersion`. IDs/enums derive from Rust; runtime decoders reject unknown codes and invalid evidence/size combinations. Limits are 1–100 rows; ordering is exact relative path, with command/item-bound cursors and parameterized queries. No physical paths or arbitrary frontend filesystem reads are exposed.

Manifest core schema remains v1. Exports add optional `org.game4.identification` under `extensions`, containing version 1 and per-relative-path metadata. It does not override core paths/sizes/hashes or declared platform; old consumers can ignore it. Existing standalone examples remain valid.

Full-library backups now write **backupVersion 2 / databaseSchemaVersion 4** and include `fileIdentifications`. The reader also accepts the original **v1/schema 3** pair, with no invented signature observations. Unsupported pairs are rejected. The eight-table snapshot, fingerprint, recovery-copy and transactional restore logic include classification rows. Recovery copies use the current format. See [backup compatibility](library-backup.md).

## Verification and references

Tests cover synthetic signature headers, N64 byte order, bounded/truncated PE, ambiguous/conflicting extensions, empty files, identical bytes with different names, preserved declared metadata, migration without scans, scoped file pagination, exact totals beyond signed-64-bit/JavaScript safe integers, both locales, manifest extension output, and legacy backup restoration.

Primary format references: [NESdev iNES](https://www.nesdev.org/wiki/INES), [N64 tool64 source](https://github.com/DylanJ/n64/blob/master/tool64.c), [Microsoft PE format](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format), [PKWARE ZIP specification](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT), [7-Zip format specification](https://github.com/ip7z/7zip/blob/main/DOC/7zFormat.txt), [MAME CHD definitions](https://github.com/mamedev/mame/blob/master/src/lib/util/chd.h), [PS-X EXE reference](https://psx-spx.consoledev.net/ps1/cdr/cdromfileformats/executables/), and [Xbox XBE reference](https://xboxdevwiki.net/Xbe).

Native Windows smoke confirmed the mixed five-file synthetic import, Portuguese metadata/detail rendering, header-vs-extension precedence, exact 16,442-byte total, 16.06 KiB localized summary, per-file hashes/sizes, and safe unknown states. An independent closed-catalog check verified schema v4, persisted observations, all original hashes, and integrity_check=ok.
