# Empty states, errors and in-app notifications

Implemented November 14, 2026 in English and Brazilian Portuguese. Notifications are part of the Game4∞ interface, not Windows system notifications; no notification permission, sound, background service or account is required.

## Feedback states

- **Loading** is distinct from an empty successful response. A failed transfer-list request does not leave the page indefinitely loading, and a failed source-list request does not tell the user to import games into an allegedly empty collection.
- **Empty collection/activity** explains the next step and provides a relevant Add files/Open Library action. Empty filtered transfers can clear their filter. Browser preview explains that local files require the desktop app.
- **Read errors** identify the affected area, show a sanitized localized backend error and provide a safe read-only retry. Library and Activity read failures are independent; Activity failure does not prevent an initial Library read. Previously loaded rows remain available when a later refresh fails.
- **Action errors** remain inline until dismissed or replaced by a new action, and automatic polling does not silently clear them. No raw exception stack or unsanitized backend string is rendered. Retrying a read does not replay a write, restore, resume or sharing operation.

## Notification center

The top bar provides Notifications and an unread count. At most three unread messages are shown in a scrollable floating area; the history retains the latest 30 messages across navigation until the app closes. Opening the history marks messages read, and each floating message can be dismissed. Messages do not disappear on a timer. Escape/close returns focus to the trigger; updates use a polite live region and do not play sounds or request system notifications. The interface translates stored message keys when the language changes.

Successful native actions acknowledge adding/checking files, saving preservation packages/references/file lists, preparing links/torrents, starting/pausing/stopping/resuming/removing transfers, saving/restoring backups, changing storage or speed settings and copying a download link. Cancellation of a native picker does not produce a success notification. Preparing sharing references is never announced as an upload; removing a transfer explicitly says files were kept. Finishing a file check does not claim that every copy matched.

## Activity observation

A serialized read-only poll about every 2.5 seconds observes up to the latest 50 tasks and the bounded saved transfer list. Only known status transitions produce activity completion/failure messages. First observations of historic completed or failed rows are a baseline, not new alerts; progress ticks do not generate messages. Newly accepted local imports/checks and starts/resumes register their IDs so fast completions can be observed. Completion also covers a download switching into continued seeding, without announcing it twice when the final stopped/completed snapshot arrives.

Event identities are deduplicated for the session (with a bounded recent identity set), and polling errors do not create repeated toasts. The page responsible for a read error retains its retry UI. Observations do not enable DHT, contact trackers, create sessions, resume transfers or mutate native data. They do not guarantee delivery of every transition between samples, notifications for work older than the recent task page, or background delivery after app exit. The monitor uses existing advertised capabilities; IPC contract v1, native schemas and public formats are unchanged.

## Verification

Tests cover bounded/deduplicated history, historic baselines, progress without repeated alerts, known task completion/failure, completed downloads/continued seeding, source seeding without a fabricated download completion, transfer failures and removal of stale observation IDs. Existing IPC and format tests remain in place. Manual visual validation and its boundaries are recorded in the project log.
