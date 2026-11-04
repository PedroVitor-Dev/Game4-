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
