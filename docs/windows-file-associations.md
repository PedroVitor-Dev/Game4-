# Windows .g4 and .g4e file associations

Implemented November 14, 2026 for the Windows NSIS installer. The associations use Tauri's [file-association configuration](https://v2.tauri.app/reference/config/#fileassociation) and official [single-instance plugin](https://v2.tauri.app/plugin/single-instance/) (locked at 2.5.5).

| Extension | Windows type                                  | Opening behavior                                                                                                          |
| --------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `.g4`     | Game4 preservation package (`Game4.Package`)  | Read/validate the ZIP container, public descriptors and every included SHA-256; show a read-only summary                  |
| `.g4e`    | Game4 sharing manifest (`Game4.EverManifest`) | Read/validate the descriptor offline and show metadata/download links; referenced payload is not authenticated or fetched |

## Installation and use

Install a build made with `npm.cmd run desktop:build -- --bundles nsis` (or `--debug --bundles nsis` for a local unsigned debug installer). The installer registers the supported file types; a development/no-bundle executable alone does not install file associations. NSIS supplies a quoted executable and quoted `%1` filename, preserving paths with spaces. The bundled association entries use Viewer behavior and the app's existing icon.

After installation, double-click a .g4/.g4e or choose Game4∞ from Windows **Open with**. Windows may ask the user to choose a preferred/default app if another handler is already selected. The app does not write protected Windows `UserChoice` keys, run `assoc`/`ftype`, add a custom protocol handler or take over unrelated extensions. Tauri's installer/uninstaller templates own registration/removal; update/uninstall/default-choice behavior should also be exercised in a clean Windows test VM before release.

A cold start queues the native filename until the frontend is ready. If Game4∞ is already open, the official plugin forwards arguments/current directory to that process, restores/focuses its main window and exits the second process. This avoids a second app session opening the same catalog. No shell is used to interpret incoming filenames.

## Inspection and queue

Files appear in **File opened from Windows** above the current page. Validation runs in a blocking worker, leaving the interface responsive. Close the preview to inspect the next queued file. Errors appear in the same card and can be dismissed without blocking subsequent files.

A .g4e link offers **Use this download link**. This pre-fills the Download screen and imported tracker suggestions, with DHT/trackers still opt-in; it does not fetch metadata or payload. The usual review, destination confirmation and explicit download action remain required. Nothing is extracted, executed, imported into the Library or shared just by opening an associated file.

The same native inspection helpers serve existing file-picker actions, avoiding divergent format or integrity rules. No arbitrary path is accepted through frontend IPC: the local native inbox exposes an opaque single-use token, basename and format; the inspection command consumes only the current token. Paths stay in native memory, not app logs, public manifests or saved transfer settings.

Only .g4/.g4e extensions are considered (case-insensitive). Relative paths resolve against the launcher working directory; unknown extensions, flags, URL-like arguments and oversized paths are ignored. The queue accepts up to 16 pending files, examines at most 32 arguments per invocation, coalesces duplicate pending paths and does not persist across app exits. At capacity, extra files are ignored; reopen them after dismissing pending previews. Parser/container/path/link/resource checks remain those of the existing formats. Association alone is not proof of publisher authenticity.

IPC v1 adds the advertised `associated_file_next` and `associated_file_read` capabilities (38 implemented commands total), typed kind-specific results and backward-compatible frontend capability checks.

## Validation boundary

Native tests cover spaces/Unicode, uppercase extensions, bounded/deduplicated queues, token mismatch/replay, malformed/missing files, continuation after failure, matching picker/association summaries and source bytes unchanged. IPC tests enforce format-specific results and capability gating. Installer registration and already-running forwarding must be validated with the generated Windows artifacts; unit tests alone do not certify Explorer/default-app behavior on an installed system.

The local acceptance run built an x64 debug NSIS installer and checked registration/removal commands and quoting in the generated script. A synthetic .g4 with spaces, Unicode and `&` opened on cold launch; a .g4e sent through a second launch queued in the same process, with the second process exiting. Closing the first preview revealed the queued descriptor; choosing its link prefilled Download while review remained disabled without opt-in discovery/direct connectivity. Source hashes were unchanged. The installer was not installed and no Explorer/default-app registry state was changed or certified. See [the acceptance record](evaluations/windows-file-associations-2026-11-14.json).
