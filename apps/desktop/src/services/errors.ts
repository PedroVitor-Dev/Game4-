import { normalizeIpcError } from "@game4/ipc-contracts";
import type { ErrorCode } from "@game4/ipc-contracts";
import type en from "../locales/en.json";

const keys = {
  IPC_UNAVAILABLE: "ipcUnavailable",
  CONTRACT_MISMATCH: "contractMismatch",
  INVALID_RESPONSE: "invalidResponse",
  UNEXPECTED_ERROR: "unexpectedError",
  DATABASE_UNAVAILABLE: "databaseUnavailable",
  DATABASE_TASK_FAILED: "databaseTaskFailed",
  INVALID_ARGUMENT: "invalidArgument",
  NOT_FOUND: "notFound",
  TASK_NOT_CANCELLABLE: "taskNotCancellable",
  CAPABILITY_UNAVAILABLE: "capabilityUnavailable",
  SOURCE_UNAVAILABLE: "sourceUnavailable",
  SOURCE_CHANGED: "sourceChanged",
  IMPORT_LIMIT_EXCEEDED: "importLimitExceeded",
  VERIFICATION_LIMIT_EXCEEDED: "verificationLimitExceeded",
  MANIFEST_INVALID: "manifestInvalid",
  MANIFEST_VERSION_UNSUPPORTED: "manifestVersionUnsupported",
  MANIFEST_WRITE_FAILED: "manifestWriteFailed",
  BACKUP_INVALID: "backupInvalid",
  BACKUP_VERSION_UNSUPPORTED: "backupVersionUnsupported",
  BACKUP_WRITE_FAILED: "backupWriteFailed",
  LIBRARY_BUSY: "libraryBusy",
  RESTORE_STALE: "restoreStale",
  RESTORE_FAILED: "restoreFailed",
} as const satisfies Record<ErrorCode, keyof typeof en.errors>;

export function getIpcErrorKey(error: unknown) {
  return `errors.${keys[normalizeIpcError(error).code]}` as const;
}
