# Storage folders

Implemented November 14, 2026. Open **Settings → Storage folders** to configure three independent native-dialog defaults:

| Folder             | Used by                                                                              |
| ------------------ | ------------------------------------------------------------------------------------ |
| Downloads          | The parent-folder picker for a new BitTorrent download                               |
| Preservation files | Save dialogs for .g4, .g4e (and its companion torrent), .torrent and JSON file lists |
| Library backups    | The save dialog for an explicit catalog backup                                       |

The configured directory is the **initial location**, not an automatic write destination. Each operation still opens its native dialog so the user confirms the actual path and may choose another location. No directory preference starts a download, uploads data or publishes references. Empty defaults retain the operating system/dialog behavior of choosing each time.

**Choose folder** and **Change folder** use a native directory picker. A selected folder must be an existing absolute directory, pass link/junction checks and allow a unique temporary write/sync/cleanup probe. Cancel keeps the old preference. **Clear default** removes only that preference, never the folder or its contents. A missing drive is shown unavailable; operations using that default fail with a storage error instead of silently redirecting writes or recreating the missing folder. Reconnect it, replace the default or clear it.

## Existing files and private data

Library originals stay at their imported locations. Active, paused and resumed transfers retain their owned download directories and sidecars; new folder preferences apply to future dialogs only. Completed downloads are not automatically imported. There is no content migration, automatic backup, storage quota, disk-capacity guarantee or global Guardian allocation in this increment.

The app's fixed private root is displayed under **Where Game4∞ keeps its internal data**. On Windows it is normally `%LOCALAPPDATA%/io.github.pedrovitor-dev.game4`. Catalog SQLite v4, transfer SQLite v1 and recovery backups remain managed there. Moving that internal root is not exposed as a setting.

Preferences are separate private JSON v1 at `settings/storage.json` under that root. The file contains local paths, is limited to 64 KiB, rejects duplicate/unknown fields, relative paths and unsupported versions, and is written using a synced temporary file plus atomic replacement in the same directory. Concurrent settings operations serialize in-process. No power-loss guarantee or cross-process settings locking is claimed. Existing malformed/future settings are retained and reported rather than reset; restore a supported valid settings file after closing the app if the file was externally corrupted. Such an error does not reset the Library or saved transfers. Folder permissions/availability can change after validation.

Preferences are excluded from public preservation formats and catalog backup v2. They are not encrypted and should be treated as private machine-local configuration. Native paths are returned only to the local settings interface. Frontend mutation requests accept a directory kind, never an arbitrary filesystem write path.

## IPC and validation

Contract v1 adds the advertised `storage_settings_get`, `storage_directory_choose` and `storage_directory_reset` commands (36 total implemented commands). Directory kinds are `downloads`, `exports`, `backups`; responses expose configured path/availability and the fixed app-data root. Older backends fail capability checks before opening a picker or modifying settings.

Native tests cover defaults without side effects, independent choices across restart, resets retaining content, disconnected/moved directories, invalid inputs, future/duplicate/oversized JSON, failed writes, Windows junction rejection and atomic replacement failure with a locked old file. IPC tests cover malformed responses, cancellation, capability gating and rejection of frontend-supplied writable paths.
