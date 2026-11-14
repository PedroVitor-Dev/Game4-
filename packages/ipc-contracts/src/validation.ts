import {
  ERROR_CODES,
  ERROR_FIELDS,
  IPC_CONTRACT_VERSION,
  FILE_FORMATS,
  PLATFORM_IDS,
  IDENTIFICATION_EVIDENCE,
} from "./generated.ts";
import type {
  AppInfo,
  DatabaseInfo,
  ErrorCode,
  ErrorField,
  IpcError,
  ExportManifestResult,
  LibraryBackupResult,
  LibraryRestorePreview,
  LibraryPrepareRestoreResult,
  LibraryRestoreResult,
  FileFormat,
  PlatformId,
  IdentificationEvidence,
  FormatSummary,
  PlatformSummary,
  LibraryFileInfo,
  LibraryItemFilesPage,
  SourceSelection,
  TaskAccepted,
  TaskSnapshot,
  TaskPage,
  LibraryPage,
  LibraryIntegrity,
  PackageOpenResult,
  EverOpenResult,
  EverExportResult,
  TrackerList,
  MagnetLink,
  CancelMagnetResult,
  TorrentCreateResult,
  TorrentImportResult,
  TransferTelemetry,
  TransferMetricsInfo,
  TransferSnapshot,
  TransferStartResult,
  TransferPage,
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
        formats:
          i.formats === undefined
            ? []
            : values(i.formats).map(parseFormatSummary),
        detectedPlatforms:
          i.detectedPlatforms === undefined
            ? []
            : values(i.detectedPlatforms).map(parsePlatformSummary),
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

export function parseLibraryBackupResult(v: unknown): LibraryBackupResult {
  return parseExportManifestResult(v);
}
function backupCounts(o: Record<string, unknown>) {
  return {
    itemCount: safeCount(o.itemCount, 0, 500000),
    fileCount: safeCount(o.fileCount, 0, 500000),
    locationCount: safeCount(o.locationCount, 0, 500000),
    taskCount: safeCount(o.taskCount, 0, 500000),
  };
}
function filename(v: unknown): string {
  const name = text(v);
  if (!name.trim() || /[\\/]/.test(name)) throw failure("INVALID_RESPONSE");
  return name;
}
export function parseLibraryPrepareRestoreResult(
  v: unknown,
): LibraryPrepareRestoreResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  if (o.cancelled) {
    if (o.preview !== null) throw failure("INVALID_RESPONSE");
    return { cancelled: true, preview: null };
  }
  const p = object(o.preview);
  const restoreToken = text(p.restoreToken);
  if (
    !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(
      restoreToken,
    )
  )
    throw failure("INVALID_RESPONSE");
  const preview: LibraryRestorePreview = {
    restoreToken,
    fileName: filename(p.fileName),
    ...backupCounts(p),
  };
  return { cancelled: false, preview };
}
export function parseLibraryRestoreResult(v: unknown): LibraryRestoreResult {
  const o = object(v);
  return { ...backupCounts(o), recoveryFileName: filename(o.recoveryFileName) };
}

function enumValue<T extends string>(v: unknown, choices: readonly T[]): T {
  if (!choices.some((c) => c === v)) throw failure("INVALID_RESPONSE");
  return v as T;
}
export function parseFormatSummary(v: unknown): FormatSummary {
  const o = object(v);
  return {
    format: enumValue<FileFormat>(o.format, FILE_FORMATS),
    evidence: enumValue<IdentificationEvidence>(
      o.evidence,
      IDENTIFICATION_EVIDENCE,
    ),
    fileCount: safeCount(o.fileCount, 1, 4294967295),
  };
}
export function parsePlatformSummary(v: unknown): PlatformSummary {
  const o = object(v);
  return {
    platform: enumValue<PlatformId>(o.platform, PLATFORM_IDS),
    evidence: enumValue<IdentificationEvidence>(
      o.evidence,
      IDENTIFICATION_EVIDENCE,
    ),
    fileCount: safeCount(o.fileCount, 1, 4294967295),
  };
}
export function parseLibraryFileInfo(v: unknown): LibraryFileInfo {
  const o = object(v);
  const path = text(o.path);
  const sha256 = text(o.sha256);
  const format = enumValue<FileFormat>(o.format, FILE_FORMATS);
  const formatEvidence = enumValue<IdentificationEvidence>(
    o.formatEvidence,
    IDENTIFICATION_EVIDENCE,
  );
  const platform =
    o.platform === null
      ? null
      : enumValue<PlatformId>(o.platform, PLATFORM_IDS);
  const platformEvidence = enumValue<IdentificationEvidence>(
    o.platformEvidence,
    IDENTIFICATION_EVIDENCE,
  );
  const detectorVersion =
    o.detectorVersion === null
      ? null
      : safeCount(o.detectorVersion, 1, 4294967295);
  if (
    !path ||
    !/^[0-9a-f]{64}$/.test(sha256) ||
    (platform === null) !== (platformEvidence === "unknown") ||
    (format === "unknown") !== (formatEvidence === "unknown") ||
    (formatEvidence === "signature" && detectorVersion === null)
  )
    throw failure("INVALID_RESPONSE");
  return {
    path,
    sizeBytes: decimal(o.sizeBytes),
    sha256,
    format,
    formatEvidence,
    platform,
    platformEvidence,
    detectorVersion,
  };
}
export function parseLibraryItemFilesPage(v: unknown): LibraryItemFilesPage {
  const o = object(v);
  const files = values(o.files).map(parseLibraryFileInfo);
  const nextCursor = nullableText(o.nextCursor);
  if (files.length > 100 || (nextCursor !== null && files.length === 0))
    throw failure("INVALID_RESPONSE");
  return { files, nextCursor };
}

export function parseLibraryIntegrity(v: unknown): LibraryIntegrity {
  const o = object(v);
  const lastCheckedAt = nullableText(o.lastCheckedAt);
  if (
    lastCheckedAt !== null &&
    (!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z$/.test(lastCheckedAt) ||
      !Number.isFinite(Date.parse(lastCheckedAt)))
  )
    throw failure("INVALID_RESPONSE");
  return {
    verified: safeCount(o.verified, 0, 4294967295),
    changed: safeCount(o.changed, 0, 4294967295),
    missing: safeCount(o.missing, 0, 4294967295),
    inaccessible: safeCount(o.inaccessible, 0, 4294967295),
    unverified: safeCount(o.unverified, 0, 4294967295),
    unlocated: safeCount(o.unlocated, 0, 4294967295),
    lastCheckedAt,
  };
}

export function parsePackageOpenResult(v: unknown): PackageOpenResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  if (o.cancelled) {
    if (o.package !== null) throw failure("INVALID_RESPONSE");
    return { cancelled: true, package: null };
  }
  const p = object(o.package);
  const collectionId = text(p.collectionId);
  const title = text(p.title);
  const totalBytes = decimal(p.totalBytes);
  if (
    p.verified !== true ||
    p.packageVersion !== 1 ||
    !title.trim() ||
    title.length > 1024 ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(
      collectionId,
    ) ||
    BigInt(totalBytes) > 1099511627776n
  )
    throw failure("INVALID_RESPONSE");
  return {
    cancelled: false,
    package: {
      fileName: filename(p.fileName),
      packageVersion: 1,
      collectionId,
      title,
      totalBytes,
      fileCount: safeCount(p.fileCount, 1, 100000),
      verified: true,
    },
  };
}

function everMagnet(v: unknown): string {
  const s = text(v);
  if (s.length > 16384 || !/^magnet:\?[^\s\u0080-\uffff]*$/.test(s))
    throw failure("INVALID_RESPONSE");
  let u: URL;
  try {
    u = new URL(s);
  } catch {
    throw failure("INVALID_RESPONSE");
  }
  const xt = u.searchParams.getAll("xt");
  if (
    u.host ||
    u.pathname ||
    u.hash ||
    xt.length !== 1 ||
    !/^urn:btih:[0-9a-f]{40}$/.test(xt[0])
  )
    throw failure("INVALID_RESPONSE");
  return s;
}
export function parseEverExportResult(v: unknown): EverExportResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  if (o.cancelled) {
    if (o.fileName !== null || o.torrentFileName !== null || o.magnet !== null)
      throw failure("INVALID_RESPONSE");
    return {
      cancelled: true,
      fileName: null,
      torrentFileName: null,
      magnet: null,
    };
  }
  const fileName = filename(o.fileName);
  const torrentFileName = filename(o.torrentFileName);
  if (!/\.g4e$/i.test(fileName) || !/\.torrent$/i.test(torrentFileName))
    throw failure("INVALID_RESPONSE");
  return {
    cancelled: false,
    fileName,
    torrentFileName,
    magnet: everMagnet(o.magnet),
  };
}
export function parseEverOpenResult(v: unknown): EverOpenResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  if (o.cancelled) {
    if (o.manifest !== null) throw failure("INVALID_RESPONSE");
    return { cancelled: true, manifest: null };
  }
  const m = object(o.manifest);
  const magnets = values(m.magnets).map(everMagnet);
  const title = text(m.title);
  if (
    m.schemaVersion !== 1 ||
    m.contentVerified !== false ||
    !["original_files", "g4_package"].includes(text(m.representation)) ||
    !title.trim() ||
    title.length > 1024 ||
    magnets.length < 1 ||
    magnets.length > 16
  )
    throw failure("INVALID_RESPONSE");
  const totalBytes = decimal(m.totalBytes);
  if (BigInt(totalBytes) > 18446744073709551615n)
    throw failure("INVALID_RESPONSE");
  const fileName = filename(m.fileName);
  const fileCount = safeCount(m.fileCount, 1, 100000);
  if (
    !/\.g4e$/i.test(fileName) ||
    (m.representation === "g4_package" &&
      (fileCount !== 1 || BigInt(totalBytes) > 1100048498688n))
  )
    throw failure("INVALID_RESPONSE");
  return {
    cancelled: false,
    manifest: {
      fileName,
      schemaVersion: 1,
      representation: m.representation as "original_files" | "g4_package",
      title,
      fileCount,
      totalBytes,
      magnets,
      contentVerified: false,
    },
  };
}

function uuid(v: unknown): string {
  const s = text(v);
  if (
    !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(
      s,
    )
  )
    throw failure("INVALID_RESPONSE");
  return s;
}
export function parseTorrentCreateResult(v: unknown): TorrentCreateResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  if (o.cancelled) {
    if (o.fileName !== null || o.magnet !== null)
      throw failure("INVALID_RESPONSE");
    return { cancelled: true, fileName: null, magnet: null };
  }
  const fileName = filename(o.fileName);
  if (!/\.torrent$/i.test(fileName)) throw failure("INVALID_RESPONSE");
  return { cancelled: false, fileName, magnet: everMagnet(o.magnet) };
}
export function parseTorrentImportResult(v: unknown): TorrentImportResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  if (o.cancelled) {
    if (o.preview !== null) throw failure("INVALID_RESPONSE");
    return { cancelled: true, preview: null };
  }
  const p = object(o.preview);
  const files = values(p.files).map((v) => {
    const f = object(v);
    const path = text(f.path);
    if (
      !path ||
      path.startsWith("/") ||
      path.split("/").some((c) => !c || c === "." || c === "..") ||
      /[\\:\u0000-\u001f]/.test(path)
    )
      throw failure("INVALID_RESPONSE");
    return { path, sizeBytes: decimal(f.sizeBytes) };
  });
  const infoHash = text(p.infoHash);
  const totalBytes = decimal(p.totalBytes, true);
  if (
    !/^[0-9a-f]{40}$/.test(infoHash) ||
    files.length < 1 ||
    files.length > 1024 ||
    BigInt(totalBytes) > 1099511627776n ||
    files.reduce((s, f) => s + BigInt(f.sizeBytes), 0n) !==
      BigInt(totalBytes) ||
    new Set(files.map((f) => f.path.toLowerCase())).size !== files.length
  )
    throw failure("INVALID_RESPONSE");
  return {
    cancelled: false,
    preview: {
      trackers:
        p.trackers === undefined
          ? []
          : parseTrackerList({ trackers: p.trackers }).trackers,
      token: uuid(p.token),
      name: text(p.name),
      infoHash,
      totalBytes,
      files,
    },
  };
}
export function parseTransferMetricsInfo(v: unknown): TransferMetricsInfo {
  const o = object(v);
  if (
    typeof o.connectedPeerMetrics !== "boolean" ||
    typeof o.seedAfterDownload !== "boolean"
  )
    throw failure("INVALID_RESPONSE");
  return {
    connectedPeerMetrics: o.connectedPeerMetrics,
    seedAfterDownload: o.seedAfterDownload,
  };
}
function transferTelemetry(v: unknown): TransferTelemetry {
  const o = object(v);
  const connectedPeers = safeCount(o.connectedPeers, 0, 4294967295),
    seeders = safeCount(o.seeders, 0, 4294967295),
    leechers = safeCount(o.leechers, 0, 4294967295);
  const downloadBps = decimal(o.downloadBps),
    uploadBps = decimal(o.uploadBps);
  if (
    seeders + leechers !== connectedPeers ||
    BigInt(downloadBps) > 18446744073709551615n ||
    BigInt(uploadBps) > 18446744073709551615n
  )
    throw failure("INVALID_RESPONSE");
  return { connectedPeers, seeders, leechers, downloadBps, uploadBps };
}
export function parseTransferSnapshot(v: unknown): TransferSnapshot {
  const o = object(v);
  const totalBytes = decimal(o.totalBytes, true);
  const verifiedBytes = decimal(o.verifiedBytes);
  const uploadedBytes = decimal(o.uploadedBytes);
  if (
    !["seed", "download"].includes(text(o.mode)) ||
    !["active", "stopping", "stopped", "completed", "failed"].includes(
      text(o.status),
    ) ||
    BigInt(totalBytes) > 1099511627776n ||
    BigInt(verifiedBytes) > BigInt(totalBytes) ||
    !/^([0-9a-f]{40})$/.test(text(o.infoHash))
  )
    throw failure("INVALID_RESPONSE");
  let error = null;
  if (o.error !== null) {
    const e = object(o.error);
    if (
      !ERROR_CODES.some((c) => c === e.code) ||
      typeof e.retryable !== "boolean" ||
      !(e.field === null || ERROR_FIELDS.some((f) => f === e.field))
    )
      throw failure("INVALID_RESPONSE");
    error = normalizeIpcError(e);
  }
  const listenPort =
    o.listenPort === null ? null : safeCount(o.listenPort, 1, 65535);
  const outputName = o.outputName === null ? null : filename(o.outputName);
  if (
    (o.status === "failed") !== !!error ||
    (o.status === "completed" &&
      (o.mode !== "download" || verifiedBytes !== totalBytes)) ||
    (o.mode === "seed" && outputName !== null) ||
    (o.mode === "download" && outputName === null)
  )
    throw failure("INVALID_RESPONSE");
  const receivedBytes =
    o.receivedBytes === undefined ? "0" : decimal(o.receivedBytes);
  const seedAfterDownload =
    o.seedAfterDownload === undefined ? false : o.seedAfterDownload;
  const isSeeding = o.isSeeding === undefined ? false : o.isSeeding;
  const telemetry =
    o.telemetry === undefined || o.telemetry === null
      ? null
      : transferTelemetry(o.telemetry);
  if (
    typeof seedAfterDownload !== "boolean" ||
    typeof isSeeding !== "boolean" ||
    BigInt(receivedBytes) > 18446744073709551615n ||
    (isSeeding &&
      (o.status !== "active" ||
        verifiedBytes !== totalBytes ||
        (o.mode !== "seed" && !seedAfterDownload))) ||
    (telemetry !== null && o.status !== "active") ||
    (o.mode === "seed" && seedAfterDownload)
  )
    throw failure("INVALID_RESPONSE");
  return {
    id: uuid(o.id),
    name: text(o.name),
    infoHash: text(o.infoHash),
    mode: o.mode as TransferSnapshot["mode"],
    status: o.status as TransferSnapshot["status"],
    totalBytes,
    verifiedBytes,
    uploadedBytes,
    receivedBytes,
    seedAfterDownload,
    isSeeding,
    telemetry,
    listenPort,
    dht:
      o.dht === undefined
        ? false
        : typeof o.dht === "boolean"
          ? o.dht
          : (() => {
              throw failure("INVALID_RESPONSE");
            })(),
    trackerCount:
      o.trackerCount === undefined ? 0 : safeCount(o.trackerCount, 0, 16),
    outputName,
    error,
  };
}
export function parseTransferStartResult(v: unknown): TransferStartResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  if (o.cancelled) {
    if (o.transfer !== null) throw failure("INVALID_RESPONSE");
    return { cancelled: true, transfer: null };
  }
  return { cancelled: false, transfer: parseTransferSnapshot(o.transfer) };
}
export function parseTransferPage(v: unknown): TransferPage {
  const o = object(v);
  const items = values(o.items).map(parseTransferSnapshot);
  if (
    items.length > 64 ||
    new Set(items.map((t) => t.id)).size !== items.length
  )
    throw failure("INVALID_RESPONSE");
  return { items };
}

export function parseMagnetLink(v: unknown): MagnetLink {
  const o = object(v);
  const magnet = everMagnet(o.magnet);
  const infoHash = text(o.infoHash);
  if (
    !/^[0-9a-f]{40}$/.test(infoHash) ||
    new URL(magnet).searchParams.get("xt") !== `urn:btih:${infoHash}`
  )
    throw failure("INVALID_RESPONSE");
  const params = new URL(magnet).searchParams;
  if ([...params.keys()].some((k) => !["xt", "tr"].includes(k)))
    throw failure("INVALID_RESPONSE");
  parseTrackerList({ trackers: params.getAll("tr") });
  return { magnet, infoHash };
}
export function parseCancelMagnetResult(v: unknown): CancelMagnetResult {
  const o = object(v);
  if (typeof o.cancelled !== "boolean") throw failure("INVALID_RESPONSE");
  return { cancelled: o.cancelled };
}

export function parseTrackerList(v: unknown): TrackerList {
  const o = object(v);
  const trackers = values(o.trackers).map((v) => {
    const s = text(v);
    let u: URL;
    try {
      u = new URL(s);
    } catch {
      throw failure("INVALID_RESPONSE");
    }
    if (
      !s ||
      s.length > 2048 ||
      /\s/.test(s) ||
      !["http:", "https:", "udp:"].includes(u.protocol) ||
      !u.hostname ||
      u.username ||
      u.password ||
      u.hash ||
      u.port === "0" ||
      (u.protocol === "udp:" && (!u.port || !!u.search))
    )
      throw failure("INVALID_RESPONSE");
    return s;
  });
  if (
    trackers.length > 16 ||
    trackers.reduce((s, t) => s + t.length, 0) > 4096 ||
    new Set(trackers).size !== trackers.length
  )
    throw failure("INVALID_RESPONSE");
  return { trackers };
}
