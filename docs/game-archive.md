# Game Archive architecture and implementation

The historical Game Archive is separate from My Library's imported originals. Both live in the existing local SQLite database; metadata never implies permission, physical ownership, integrity or network availability. No central login, scraping, game execution or automatic transfer was introduced.

## Reused foundation

React/TypeScript, existing CSS/Tailwind toolchain, i18next, Tauri/Rust, SQLite, background SHA-256 tasks, native pickers, feedback/notifications, direct-original preservation, .g4/.g4e and the existing BitTorrent download/share workflows remain in place. No dependency was added. Local authorized artwork uses SQLite BLOB storage and lazy blob URLs; CSP permits blob images but no arbitrary remote image host.

## Schema 5

Migration `0005_archive.sql` adds normalized `archive_` tables: platforms, games, releases, editions, contents, dependencies, preservation_files, p2p_sources, assets, catalogs, catalog_entries, collections, collection_entries, preferences and metadata_sources. Existing files, locations, collections, tasks and content identities are unchanged. Initial platform categories have no invented years, images, histories or game counts and are editable in the app.

A game has many platform releases. Releases own region/language/title identifiers and editions; editions own technical base revisions, updates and DLCs. Content kinds remain separate entities/rows, rendered in separate tabs. Version identifiers are strings. Updates/DLCs require a concrete base revision in the same edition (therefore the same platform and release region). Additional content dependencies must belong to the same game and cannot form a cycle. Parent identities cannot be silently moved during editing. This first compatibility profile selects one exact base revision per content row; multiple compatibility ranges/alternative bases are not inferred.

Preservation-file rows refer to the existing content hashes and optional local item/file identities. Linking an import does not copy originals or auto-create a game. Public references have no local item/file IDs. Multiple P2P sources attach to content without duplicating its identity. Historical sources, contributions and revision notes are distinct from file verification observations. SQL is parameterized; public files cannot supply executable SQL.

## User journeys

Game Archive exposes real local totals, featured platforms, recently added/updated games and a paginated explorer. Platforms opens a console catalog with editable history/artwork. Game details exposes Overview, Versions, Updates, DLCs, Files, Preservation and Community. Forms create/edit metadata, platform releases, editions, content, compatibility dependencies, attribution and distribution references. New games open directly in their details so their first platform release can be added even when created from a platform filter.

Files can link an existing import, read a .g4e offline, or store a normalized magnet with a license declaration. Native .g4e creation and sharing reuse the existing controls for selected local imports. Download references require a permission acknowledgement and prefill the existing review screen; they never bypass its destination/network choices. A reference is not announced as a live seeder. Active metrics remain in Transfers; no background network lookup occurs in the archive.

Collections supports favorites, saved metadata-only titles, recent visits and named personal collections. Community browses imported .g4lib snapshots without requiring a central service. Artwork includes source, license and attribution and is chosen manually via a native picker; missing/broken images use a Game4 fallback. No third-party art is fetched automatically.

## Performance and accessibility

Search/filter queries debounce 180 ms, execute off the UI thread and return at most 48 cards. Indexed relationship queries and bounded detail requests avoid loading the complete game catalog. Explicit paging bounds mounted game cards rather than adding a virtualization dependency. The in-memory query cache holds 20 entries for 30 seconds and invalidates after writes. Image fetching starts near the viewport, deduplicates in-flight requests and retains up to 48 inactive cached images (visible images can remain referenced). Layout/sort preferences survive restart. Per-game long content/file/revision lists progressively display 24 rows. Tabs support arrow keys and roving focus; native HTML dialogs trap focus and restore the triggering control. Reduced-motion preferences disable transitions.

## Backups and interoperability

Private backup v3/schema 5 includes the archive, cached images, personal preferences, collections and links. Readers still accept v1/schema 3 and v2/schema 4. Restore remains a full replacement with a recovery backup and atomic rollback; old backups contain no historical archive. UI confirmation explicitly warns that historical metadata and artwork are replaced. Restored physical copies require re-verification. .g4 and .g4e public formats do not change.

Public .g4lib v1 is documented in `packages/g4lib-format/spec-v1.md`. Imports are validated in a temporary schema, reviewed, then committed atomically. Snapshot-scoped IDs prevent cross-catalog/local overwrites; importing identical content is idempotent. Different revisions remain separate snapshots, not an automatic merge. Imported authorization flags are cleared. Signing, remote subscriptions, publisher identity validation, popularity metrics and long-term preservation claims are deliberately not fabricated.

## Delivery checkpoints

1. Schema/typed service and migration/relationship tests.
2. Offline .g4lib contract and private backup compatibility.
3. Platform/archive/detail views and metadata/artwork administration.
4. Existing-file and P2P integration, personal collections and feedback.
5. Automated checks, isolated native UI acceptance and follow-up limits recorded in the project log.

Next extensions require explicit formats/decisions: signed catalog provenance and subscription updates, conflict-aware editorial merges, multi-base compatibility, attached authorized video/documentation assets, historical verification snapshots beyond the existing task history, and observed network aggregates connected to concrete sessions. Catalogs are useful before these services exist.
