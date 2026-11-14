# Game4∞ vision and user experience

Updated November 14, 2026 against the project owner's renewed product plan and the implemented desktop app. This is the current product/experience baseline; older dated entries record the scope at that time.

## Purpose

Game4∞ (Game For Ever) connects people who want to preserve video game history. The long-term result is a worldwide, collaborative, decentralized ecosystem of independent collections, knowledge and available copies—not a single centrally owned download catalog. The infinity symbol describes continuity of community effort, not a guarantee of eternal storage.

A participant should be able to keep a local collection, choose what to share, obtain a permitted copy from another participant and eventually contribute to communities. Manuals, authorized artwork, development records, historical versions and translations belong alongside game files. Original bytes remain unchanged; .g4 packaging is optional.

## What the app supports today

| User goal               | Current support                                                                                                                     | Boundary                                                                                                          |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Keep my games           | Import files/folders, background SHA-256, platform/format hints, local browsing, checking registered copies, catalog backup/restore | Downloads are not automatically imported; historical records and imported originals remain separate entities      |
| Download a shared game  | Review a .torrent or magnet, use a link found in a .g4e, choose a destination and explicitly start                                  | Imported .g4lib snapshots are searchable offline; download still requires explicit source/destination review      |
| Share my collection     | Select originals from the Library, start read-only sharing, create/copy a magnet, optionally save .torrent/.g4e references          | No community publication is performed; reference files alone do not make payload available                        |
| Control participation   | Pause, reverified resume, private saved state, optional DHT/selected trackers and per-transfer bandwidth limits                     | Restart opens interrupted transfers paused; no global bandwidth quota or automatic background Guardian allocation |
| Open preservation files | Public .g4 and .g4e v1 with creation/inspection and integrity/compatibility tests                                                   | Inspection does not execute, extract or import a package                                                          |
| Join a community        | Import/browse independent .g4lib snapshots, record attribution and keep personal collections                                        | No online joining/publication/moderation, remote subscription or signature verification is claimed                |

The real transfer engine and local integrity/compatibility checks are implemented. Public-network/NAT acceptance and independent-client interoperability still need their own evidence; passing local tests does not certify all network conditions.

## Primary journeys

1. **Download games**: use a link or torrent supplied by someone else, choose an explicit discovery/direct route, review the file list and choose a folder. Opening a .g4e is offline; selecting its link populates the form without starting a lookup. Network discovery stays opt-in.
2. **Share my collection**: add a local collection if needed, choose it and start sharing. Library cards carry the exact selected collection into this flow even when it is outside the first source-selection page. A download link appears after successful sharing. Preparing references is optional and distinct from starting upload.
3. **Community and Game Archive**: browse reviewed local .g4lib snapshots, explore platforms and game details, create historical records independently from files and attach permitted P2P references. No fake Join button or automatic publication is presented.
4. **My transfers**: monitor progress, connected participants, received/uploaded totals and rates; pause, resume, change limits or remove a record while keeping files.

Home presents these journeys ahead of statistics. My Library, Activity and Settings remain available. Logo navigation returns Home. Configuration screens and route changes do not themselves start sharing.

## Language and interaction rules

- Lead with verbs and outcomes: Add my games, Download a game, Share, Check files, Copy link, Pause, Resume.
- Use the project owner's original logo files without repainting or generating replacements. The four supplied PNGs are theme/background variants, not language-specific banners; interface copy remains English and Brazilian Portuguese.
- Keep technical identifiers, port numbers, trackers, hashes, storage sidecars and export formats in optional details where possible. Keep necessary consent near the network action instead of hiding it in advanced settings.
- Use plain connection labels while retaining accurate scope. Connected participants and advertised complete copies describe this session, not every person or copy in the network. File consistency does not establish publisher authenticity.
- Never label .g4e creation as publication, a local index as an uploaded collection, or package inspection as a download. Do not imply that received games are automatically playable or launched.
- Preserve truthful empty, unavailable, loading and future-feature states. Library search/totals describe the loaded subset. Avoid placeholder games, made-up peer counts or preservation-risk scores without evidence.
- A modified download link invalidates its old preview. A newly selected torrent clears the old link so the reviewed file list remains tied to the chosen source.

## Next chapters

1. Extend the delivered historical archive with richer media/documentation, editorial merging and multi-base compatibility; keep the existing metadata/file distinction.
2. Extend the delivered .g4lib v1 snapshot format with a reviewed design for signatures, subscriptions and publisher identity; current unsigned import/export does not certify provenance.
3. Implement community discovery and reviewed publication of .g4e references, independent community catalogs and explicit participation. No mandatory central account should gate local preservation or direct P2P exchange.
4. Extend the delivered historical game pages and personal collections with permitted community thematic collections and explicit multi-community participation.
5. Add availability history and carefully scoped estimates, then Games at Risk and voluntary Guardians with storage/bandwidth quotas and verifiable contributions. Do not infer a global copy count from connected-peer observations.
6. Research complementary IPFS/CIDs, offline recovery and optional OpenTimestamps/Bitcoin proofs. Bitcoin is not game storage; IPFS does not guarantee persistence without participants maintaining data.
7. Evaluate optional Lightning donations and contribution recognition only after verification and operational needs are understood. No native token, mandatory wallet, DRM or cryptocurrency payment is required.

All stages preserve open formats, interoperable protocols, recoverable catalogs, explicit sharing consent, no automatic file execution and support for GPL-3.0-or-later subject to dependency compatibility. Distribution remains for homebrew, free software, public-domain and appropriately licensed/authorized materials. Community recognition should value curation, history, translations, verification and development as well as available copies.
