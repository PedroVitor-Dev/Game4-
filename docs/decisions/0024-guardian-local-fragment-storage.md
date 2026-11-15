# 0024: Opt-in Guardian fragment storage

Date: 2026-11-14. Status: accepted for private MVP step 4.

Implement the existing StorageNode port with a dedicated locked filesystem root and independent SQLite inventory. Explicit admission binds a selected `.g4e` v2 coordinate, expected length and SHA-256. Storage and sharing permissions are separate. Participation never resumes automatically after restart.

Serialize admission and quota changes; stream exact fragment bytes into synced host-owned staging files, then publish atomically without overwrite. Count crash orphans toward quota, recover stages on restart and require rehashing before reads. Support global directional throttling and live permission/activity checks. Expose native dialogs and bounded typed IPC through Settings, plus an offline CLI.

No public network service, automatic game admission, ROM reconstruction, catalog schema migration or change to existing BitTorrent sessions is implied. Node identity is persistent but not an authentication credential. Transport and enrolled peer authentication remain separate adapters.

See [Guardian implementation and operating guide](../g4ep/guardian-node.md).
