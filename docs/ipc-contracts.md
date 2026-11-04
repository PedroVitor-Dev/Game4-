# Frontend–backend IPC contract v1

Status: v1 implements app/database information, source selection, import, library/task listing, and task lookup. Verification, cancellation, and task events remain drafts. See [local import](library-import.md) for implemented behavior and limits.

## Ownership and boundary

Rust DTOs in `apps/desktop/src-tauri/src/contracts/` are the type source of truth. ts-rs generates TypeScript in `packages/ipc-contracts/src/generated.ts`; Rust serialization generates shared JSON examples. The typed frontend client is `apps/desktop/src/services/ipc-client.ts`, and `desktop.ts` is the only application adapter importing Tauri `invoke`.

The client treats IPC data as `unknown`, validates active responses, and returns typed values only after decoding. `invoke<T>` is not treated as runtime validation. Application pages/hooks should use service functions rather than native command strings.

Database internals return `DatabaseStats`; command code converts it into the public `DatabaseInfo`. Do not expose SQL rows, SQLite connections, arbitrary SQL execution, filesystem destinations, or internal exception strings as contracts.

## Active commands

| Tauri command   | Arguments | Resolved payload | Rejection                                     |
| --------------- | --------- | ---------------- | --------------------------------------------- |
| `app_info`      | None      | `AppInfo`        | Transport/decode failure normalized by client |
| `database_info` | None      | `DatabaseInfo`   | `IpcError`                                    |

Tauri resolves the payload directly, with no additional success envelope. A Rust `Result::Err` rejects the invocation with the serialized error object. The client wraps a recognized failure in `IpcFailure`; `.error` contains its stable wire data. An exception's display text is not user-facing copy.

Canonical application response:

```json
{
  "version": "0.1.0",
  "contractVersion": 1,
  "capabilities": [
    "app_info",
    "database_info",
    "library_select_sources",
    "library_import",
    "library_list",
    "tasks_list",
    "task_get"
  ]
}
```

`version` is the application version; `contractVersion` is the IPC major version and is independent of SQLite `schemaVersion`. Capabilities contain only implemented command names. Unknown capability names are ignored by older consumers.

The frontend caches one successful bootstrap handshake per client instance. Concurrent initial reads share it. Failed handshakes can be retried on a later explicit call. `getDatabaseInfo()` first checks the handshake and advertised capability; it never invokes the database command when the protocol is incompatible or the capability is absent. Browser preview does not invoke native commands.

Canonical database response:

```json
{
  "schemaVersion": 2,
  "fileCount": 0,
  "locationCount": 0,
  "itemCount": 0,
  "taskCount": 0
}
```

Schema version is a positive u32. Counts are integers from zero through `Number.MAX_SAFE_INTEGER`; Rust checks the range before serialization and the frontend checks it again. Negative, fractional, string-valued, unsafe, or missing counts are rejected.

## Stable errors and localization

```json
{
  "code": "DATABASE_UNAVAILABLE",
  "retryable": false,
  "field": null
}
```

| Code                     | Meaning                                                       |
| ------------------------ | ------------------------------------------------------------- |
| `IPC_UNAVAILABLE`        | Native bridge is unavailable, including browser mode          |
| `CONTRACT_MISMATCH`      | Unsupported IPC major version                                 |
| `INVALID_RESPONSE`       | Payload violates the active response contract                 |
| `UNEXPECTED_ERROR`       | Unrecognized or unstructured failure, sanitized by the client |
| `DATABASE_UNAVAILABLE`   | Local database operation failed                               |
| `DATABASE_TASK_FAILED`   | Blocking database worker could not finish                     |
| `INVALID_ARGUMENT`       | Handler rejected an input value                               |
| `NOT_FOUND`              | Handler could not find the requested entity                   |
| `TASK_NOT_CANCELLABLE`   | Planned task cannot accept cancellation                       |
| `CAPABILITY_UNAVAILABLE` | Requested feature is not advertised by this backend           |

`field` is null or one of `request`, `cursor`, `limit`, `taskId`, `itemId`, `selectionToken`, `requestId`. It identifies a field, not its supplied value. Never include absolute paths, database errors, tokens, stack traces, or user-provided secrets in errors.

Import also uses `SOURCE_UNAVAILABLE`, `SOURCE_CHANGED`, and `IMPORT_LIMIT_EXCEEDED`, without exposing failed paths.

All codes map to English and Brazilian Portuguese strings in the frontend. Backend errors contain no natural-language UI messages. Unknown codes, raw strings, and malformed error objects become `UNEXPECTED_ERROR`. `retryable` is a hint, not permission to automatically repeat a mutation; current backend errors conservatively return false. The UI uses these translations if bootstrap fails.

## Data conventions

- DTO field names use camelCase. Persisted domain enums use snake_case. Error codes use SCREAMING_SNAKE_CASE.
- Nullable fields serialize as explicit `null`; canonical requests include them. No `undefined`, NaN, Infinity, native Date, or BigInt values cross JSON.
- Entity IDs are positive SQLite identifiers encoded as canonical decimal strings (`^[1-9][0-9]*$`). Do not parse them through JavaScript Number.
- Byte totals, task progress counts, and revisions use decimal strings (`0` or a positive integer without leading zeroes). Use BigInt only after receiving/parsing JSON when exact arithmetic is needed.
- The existing bounded database counts remain numeric. Their explicit safe range is part of v1.
- Timestamps are UTC RFC 3339 strings, normally with milliseconds and a trailing `Z`; the UI formats them using its locale.
- Responses may include additional fields; current decoders ignore them. Required fields cannot be silently removed or change type. Names, paths, titles, and hashes are never translated.

## Library and task command mapping

Every argument-bearing handler uses one top-level `request` object, for example `{ request: { taskId: "42" } }`. Request/response names below refer to the generated DTOs.

| Command                  | Request                | Response             |
| ------------------------ | ---------------------- | -------------------- |
| `library_select_sources` | `SelectSourcesRequest` | `SourceSelection`    |
| `library_import`         | `StartImportRequest`   | `TaskAccepted`       |
| `library_verify`         | `StartVerifyRequest`   | `TaskAccepted`       |
| `library_list`           | `LibraryListRequest`   | `LibraryPage`        |
| `tasks_list`             | `PageRequest`          | `TaskPage`           |
| `task_get`               | `TaskIdRequest`        | `TaskSnapshot`       |
| `task_cancel`            | `TaskIdRequest`        | `CancelTaskResponse` |

The table includes both active and draft contracts. `library_verify` and `task_cancel` are drafts; all other rows are active. `DraftCommand` and `DRAFT_COMMANDS` are separate from the implemented registry. There are no executable stubs that report success. A draft becomes callable only after adding a real handler, input/business validation, tests, capability advertisement, runtime response decoder, and typed client method.

### Selection and start requests

`SelectSourcesRequest.kind` is `files` or `directories`. The native picker owns authorization. Cancellation returns `cancelled: true`, `selectionToken: null`, and an empty `displayNames` array. A successful selection returns an opaque token and display names, not arbitrary frontend-supplied absolute paths.

Tokens are scoped to the current app process and expire after five minutes. A new import consumes a valid selection token; repeating an already accepted request ID must resolve the existing task before attempting token consumption. The token store and native picker are implemented. Tokens are never accepted as filesystem paths.

`StartImportRequest` supplies `requestId` and `selectionToken`; `StartVerifyRequest` supplies `requestId` and `itemId`. `requestId` is a frontend-created UUID v4 reused for retries of the same action. Same command/ID and same payload must return the same task; a conflicting payload must reject with `INVALID_ARGUMENT`. Deduplication must be retained for at least the task record's lifetime. Persisting task creation and idempotency requires a transaction.

Active import handlers persist task creation/request deduplication in one insert under the admission lock. A token may need re-selection if persistence fails before acceptance.

The immediate response identifies an accepted task; it does not mean work has completed. A newly created task is queued. A duplicate can return that task's current state. No automatic sharing starts with import or verification.

### Pagination and validation requirements

`PageRequest` has explicit `cursor: string | null` and `limit: number` (1–100; UI default 50). `LibraryListRequest.query` is null or trimmed text up to 200 characters. Cursors are opaque, bound to the command/filter/sort, and validated by Rust; `nextCursor: null` means the end. Initial ordering is library ID ascending and task ID descending. Do not expose SQL offsets or construct queries from cursors.

Handlers must validate IDs, bounds, token expiry, semantic field combinations, and authorization before side effects. Generated TypeScript and Rust deserialization are not substitutes for these business rules. Active commands validate these rules. Verification/cancellation DTOs still describe shapes only.

## Task lifecycle and draft events

Task kinds are `import` and `verify`; statuses are `queued`, `running`, `succeeded`, `failed`, `cancelled`, and `interrupted`. Normal flow is queued → running → succeeded/failed/cancelled. A queued task may cancel immediately. Recovery must mark unfinished work interrupted before an explicit retry policy; do not silently report success after a restart.

`TaskSnapshot` contains an ID, kind, status, decimal-string progress, optional total, a persisted monotonic revision, UTC update time, and nullable `IpcError`. Unknown total is null, not zero. Progress cannot exceed a known total. Failed tasks require an error; successful/cancelled tasks have no error.

`task_cancel` acknowledges a cancellation request; `cancellationRequested: true` does not mean the worker has stopped. Repeated cancellation is idempotent. Already-terminal tasks return their current status and `cancellationRequested: false`; completion can win a race with cancellation.

Reserved event: `game4://task-updated`, payload `TaskUpdatedEvent` containing `contractVersion` and the full task snapshot. **No event is emitted in the current app.** Future consumers subscribe before querying snapshots, keep only increasing revisions per task, and reconcile through `task_get`/`tasks_list` after gaps or reconnects. Events are notifications, not durable storage or delivery guarantees. Await subscription registration and dispose the returned listener, including React unmount races.

SQLite schema v2 stores import request deduplication, revisions, and update timestamps. Startup interrupts queued/running work. Workers update durable snapshots; the frontend polls them. Cancellation requests are not stored, `task_cancel` is not registered, and no events are emitted. Events/cancellation require their own implementation and tests before activation.

## Versioning and verification

v1 is the first formal contract. Existing embedded frontend/backend builds must ship together. Adding a command/capability or ignorable response field is additive; removing required fields, changing types/semantics, or introducing enum states old clients cannot handle requires a new major version or explicitly negotiated capability. Do not silently weaken validation to accept incompatible payloads.

```powershell
npm.cmd run contracts:generate
npm.cmd run contracts:check
npm.cmd run check
npm.cmd run rust:test
```

Rust tests compare generated bindings/examples with committed artifacts. Frontend tests consume Rust-serialized examples and exercise malformed data, precision limits, version mismatch, capability checks, concurrent bootstrap, safe retry after failure, browser mode, and error localization. CI checks generation drift before frontend checks. Generated artifacts are never rewritten by tests.

Native smoke verification must confirm the current executable's version handshake, database payload, and UI boot. Hosted CI execution remains pending until the owner pushes.

Primary references: [Tauri command serialization](https://v2.tauri.app/develop/calling-rust/), [Tauri events](https://v2.tauri.app/develop/calling-frontend/), and [ts-rs](https://docs.rs/ts-rs/12.0.1/ts_rs/).
