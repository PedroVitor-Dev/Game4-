import {
  ERROR_CODES,
  ERROR_FIELDS,
  IPC_CONTRACT_VERSION,
} from "./generated.ts";
import type {
  AppInfo,
  DatabaseInfo,
  ErrorCode,
  ErrorField,
  IpcError,
  ExportManifestResult,
  SourceSelection,
  TaskAccepted,
  TaskSnapshot,
  TaskPage,
  LibraryPage,
} from "./generated.ts";

export class IpcFailure extends Error {
  readonly error: IpcError;
  constructor(error: IpcError) {
    super(error.code);
    this.name = "IpcFailure";
    this.error = error;
  }
}

export const failure = (code: ErrorCode) =>
  new IpcFailure({ code, retryable: false, field: null });
const record = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
export function normalizeIpcError(value: unknown): IpcError {
  if (value instanceof IpcFailure) return value.error;
  if (
    record(value) &&
    ERROR_CODES.some((code) => code === value.code) &&
    typeof value.retryable === "boolean" &&
    (value.field === null ||
      ERROR_FIELDS.some((field) => field === value.field))
  ) {
    return {
      code: value.code as ErrorCode,
      retryable: value.retryable,
      field: value.field as ErrorField | null,
    };
  }
  return { code: "UNEXPECTED_ERROR", retryable: false, field: null };
}

export function parseAppInfo(value: unknown): AppInfo {
  if (
    !record(value) ||
    typeof value.version !== "string" ||
    !value.version.trim() ||
    !Number.isSafeInteger(value.contractVersion) ||
    !Array.isArray(value.capabilities) ||
    !value.capabilities.every((capability) => typeof capability === "string")
  )
    throw failure("INVALID_RESPONSE");
  if (value.contractVersion !== IPC_CONTRACT_VERSION)
    throw failure("CONTRACT_MISMATCH");
  return {
    version: value.version,
    contractVersion: IPC_CONTRACT_VERSION,
    capabilities: [...value.capabilities],
  };
}

function safeCount(
  value: unknown,
  minimum = 0,
  maximum = Number.MAX_SAFE_INTEGER,
): number {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < minimum ||
    value > maximum
  )
    throw failure("INVALID_RESPONSE");
  return value;
}

export function parseDatabaseInfo(value: unknown): DatabaseInfo {
  if (!record(value)) throw failure("INVALID_RESPONSE");
  return {
    schemaVersion: safeCount(value.schemaVersion, 1, 4_294_967_295),
    fileCount: safeCount(value.fileCount),
    locationCount: safeCount(value.locationCount),
    itemCount: safeCount(value.itemCount),
    taskCount: safeCount(value.taskCount),
  };
}

function object(v: unknown): Record<string, unknown> {
  if (!record(v)) throw failure("INVALID_RESPONSE");
  return v;
}
function text(v: unknown): string {
  if (typeof v !== "string") throw failure("INVALID_RESPONSE");
  return v;
}
function decimal(v: unknown, positive = false): string {
  const s = text(v);
  if (!(positive ? /^[1-9][0-9]*$/ : /^(0|[1-9][0-9]*)$/).test(s))
    throw failure("INVALID_RESPONSE");
  return s;
}
function nullableText(v: unknown): string | null {
  return v === null ? null : text(v);
}
function values(v: unknown): unknown[] {
  if (!Array.isArray(v)) throw failure("INVALID_RESPONSE");
  return v;
}
function status(v: unknown): TaskSnapshot["status"] {
  if (
    ![
      "queued",
      "running",
      "succeeded",
      "failed",
      "cancelled",
      "interrupted",
    ].includes(text(v))
  )
    throw failure("INVALID_RESPONSE");
  return v as TaskSnapshot["status"];
}
export function parseSourceSelection(v: unknown): SourceSelection {
  const o = object(v);
  const token = nullableText(o.selectionToken);
  const names = values(o.displayNames).map(text);
  if (
    typeof o.cancelled !== "boolean" ||
    (o.cancelled
      ? token !== null || names.length !== 0
      : !token || names.length === 0)
  )
    throw failure("INVALID_RESPONSE");
  return { selectionToken: token, displayNames: names, cancelled: o.cancelled };
}
export function parseTaskAccepted(v: unknown): TaskAccepted {
  const o = object(v);
  return { taskId: decimal(o.taskId, true), status: status(o.status) };
}
export function parseTaskSnapshot(v: unknown): TaskSnapshot {
  const o = object(v);
  if (o.kind !== "import" && o.kind !== "verify")
    throw failure("INVALID_RESPONSE");
  const processed = decimal(o.processedCount);
  const total = o.totalCount === null ? null : decimal(o.totalCount);
  const state = status(o.status);
  if (total !== null && BigInt(processed) > BigInt(total))
    throw failure("INVALID_RESPONSE");
  const timestamp = text(o.updatedAt);
  if (
    !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/.test(timestamp) ||
    !Number.isFinite(Date.parse(timestamp))
  )
    throw failure("INVALID_RESPONSE");
  let error: IpcError | null = null;
  if (o.error !== null) {
    const raw = object(o.error);
    if (
      !ERROR_CODES.some((c) => c === raw.code) ||
      typeof raw.retryable !== "boolean" ||
      !(raw.field === null || ERROR_FIELDS.some((f) => f === raw.field))
    )
      throw failure("INVALID_RESPONSE");
    error = normalizeIpcError(raw);
  }
  if (
    (state === "failed" && !error) ||
    (["succeeded", "cancelled"].includes(state) && error)
  )
    throw failure("INVALID_RESPONSE");
  return {
    id: decimal(o.id, true),
    kind: o.kind,
    status: state,
    processedCount: processed,
    totalCount: total,
    revision: decimal(o.revision),
    updatedAt: timestamp,
    error,
  };
}
export function parseTaskPage(v: unknown): TaskPage {
  const o = object(v);
  return {
    items: values(o.items).map(parseTaskSnapshot),
    nextCursor: nullableText(o.nextCursor),
  };
}
export function parseLibraryPage(v: unknown): LibraryPage {
  const o = object(v);
  return {
    items: values(o.items).map((v) => {
      const i = object(v);
      return {
        id: decimal(i.id, true),
        title: text(i.title),
        platform: nullableText(i.platform),
        fileCount: safeCount(i.fileCount, 0, 4294967295),
        totalBytes: decimal(i.totalBytes),
      };
    }),
    nextCursor: nullableText(o.nextCursor),
  };
}

export function parseExportManifestResult(v: unknown): ExportManifestResult {
  const o = object(v);
  const fileName = nullableText(o.fileName);
  if (
    typeof o.cancelled !== "boolean" ||
    (o.cancelled ? fileName !== null : !fileName || /[\\/]/.test(fileName))
  )
    throw failure("INVALID_RESPONSE");
  return { cancelled: o.cancelled, fileName };
}
