# Original-file preservation policy

Status: enforced by the implemented local workflows; formalized November 13, 2026.

## Preservation contract

Original files are the user-selected source bytes, at their existing names and locations. Import, identification, manual verification, manifest export, package creation/reading and library backup/restore MUST NOT write, rename, move, delete, patch, normalize or transcode those files. The application MUST NOT change source permissions or timestamps to make a check succeed. A failed read is a finding or error, never permission to repair an original.

Content identity is SHA-256 plus exact byte length. A later mismatch MUST NOT silently replace a reference hash. Reimport may record a newly observed content identity while retaining existing identities according to the catalog model; it must not edit source bytes. A changed, missing or inaccessible original must remain in that state until a separate explicit user action outside these read-only workflows changes it.

Any future transformation, repair or runnable copy MUST use a separate destination, preserve the source, and record its own identity and provenance. No automatic conversion, patching or execution is part of importing or reading a package.

## Implemented boundaries

- Rust opens originals through the shared read-only `library::open_source` helper. It rejects non-regular sources, links and Windows reparse points, including redirected ancestors; Windows handles permit concurrent reads while denying writes/deletion for the handle lifetime.
- Hashing uses bounded streaming reads and checks for observed source changes. Native commands run blocking work off the UI thread.
- `.g4` creation compares source bytes against reference hashes, checks them again while copying and publishes a new destination without clobbering an existing file. The original file's exact byte sequence becomes its `data/<manifest path>` payload; ZIP metadata timestamps/modes are package metadata and do not replace source filesystem metadata.
- `.g4` reading checks structure and every payload hash without extracting or executing content. Manifest export reads catalog metadata. Backups contain catalog rows, not original bytes; restoration replaces catalog rows and resets copy health, leaving original paths/files untouched.
- Normal write failures discard temporary export output and retain sources. An abrupt process/OS failure may leave a temporary output sibling. It does not authorize modification of originals.

The contract concerns application writes and byte preservation. The operating system may update access times as a side effect of reading. Antivirus, cloud sync, filesystem behavior and other programs can change files independently. The application does not promise immutable storage, metadata-forensic imaging, prevention of external changes, or file authenticity.

## Verification

Synthetic regression tests compare the complete source tree before and after import, manifest export, `.g4` creation/readback, manual integrity verification, backup save/restore and rejected exports. They check original relative names, directory entries, bytes, sizes, modification times and read-only attributes. Other tests cover source changes, locks, junctions and unchanged reference hashes. No personal collection is used.

See [manual integrity](library-integrity.md), [.g4 v1](../packages/g4-format/spec-v1.md), and [backup semantics](library-backup.md) for individual workflows.
