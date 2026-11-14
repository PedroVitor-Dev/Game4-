import {
  failure,
  IpcFailure,
  normalizeIpcError,
  parseAppInfo,
  parseDatabaseInfo,
  parseSourceSelection,
  parseTaskAccepted,
  parseLibraryPage,
  parseLibraryIntegrity,
  parsePackageOpenResult,
  parseEverOpenResult,
  parseEverExportResult,
  parseTransferForgetResult,
  parseTransferMetricsInfo,
  parseTrackerList,
  parseMagnetLink,
  parseCancelMagnetResult,
  parseTorrentCreateResult,
  parseTorrentImportResult,
  parseTransferSnapshot,
  parseTransferStartResult,
  parseTransferPage,
  parseTaskPage,
  parseTaskSnapshot,
  parseExportManifestResult,
  parseLibraryBackupResult,
  parseLibraryPrepareRestoreResult,
  parseLibraryRestoreResult,
  parseLibraryItemFilesPage,
} from "@game4/ipc-contracts";
import type {
  AppInfo,
  ImplementedCommand,
  SelectSourcesRequest,
  StartImportRequest,
  StartVerifyRequest,
  LibraryListRequest,
  PageRequest,
  TaskIdRequest,
  ExportManifestRequest,
  LibraryRestoreRequest,
  LibraryItemFilesRequest,
  TrackerList,
  TorrentReferenceRequest,
  ResolveMagnetRequest,
  CancelMagnetRequest,
  StartSeedRequest,
  StartDownloadRequest,
  TransferIdRequest,
  TransferLimitsRequest,
  BandwidthLimits,
} from "@game4/ipc-contracts";

export type IpcTransport = {
  isAvailable: () => boolean;
  invoke: (
    command: ImplementedCommand,
    args?: { request: unknown },
  ) => Promise<unknown>;
};
const decoders = {
  app_info: parseAppInfo,
  database_info: parseDatabaseInfo,
  library_select_sources: parseSourceSelection,
  library_import: parseTaskAccepted,
  library_verify: parseTaskAccepted,
  library_integrity: parseLibraryIntegrity,
  library_list: parseLibraryPage,
  library_item_files: parseLibraryItemFilesPage,
  tasks_list: parseTaskPage,
  task_get: parseTaskSnapshot,
  library_export_manifest: parseExportManifestResult,
  library_export_package: parseExportManifestResult,
  package_open: parsePackageOpenResult,
  library_export_ever: parseEverExportResult,
  ever_open: parseEverOpenResult,
  transfer_metrics_info: parseTransferMetricsInfo,
  trackers_validate: parseTrackerList,
  magnet_generate: parseMagnetLink,
  magnet_resolve: parseTorrentImportResult,
  magnet_cancel: parseCancelMagnetResult,
  torrent_create: parseTorrentCreateResult,
  torrent_import: parseTorrentImportResult,
  torrent_seed: parseTransferSnapshot,
  torrent_download: parseTransferStartResult,
  transfers_list: parseTransferPage,
  transfer_stop: parseTransferSnapshot,
  transfer_pause: parseTransferSnapshot,
  transfer_resume: parseTransferSnapshot,
  transfer_forget: parseTransferForgetResult,
  transfer_set_limits: parseTransferSnapshot,
  library_save_backup: parseLibraryBackupResult,
  library_prepare_restore: parseLibraryPrepareRestoreResult,
  library_restore_backup: parseLibraryRestoreResult,
} satisfies Record<ImplementedCommand, (value: unknown) => unknown>;
type Responses = {
  [K in ImplementedCommand]: ReturnType<(typeof decoders)[K]>;
};

export function createIpcClient(transport: IpcTransport) {
  let handshake: Promise<AppInfo> | undefined;
  async function call<K extends ImplementedCommand>(
    command: K,
    request?: unknown,
  ): Promise<Responses[K]> {
    if (!transport.isAvailable()) throw failure("IPC_UNAVAILABLE");
    try {
      return decoders[command](
        await transport.invoke(
          command,
          request === undefined ? undefined : { request },
        ),
      ) as Responses[K];
    } catch (error) {
      throw error instanceof IpcFailure
        ? error
        : new IpcFailure(normalizeIpcError(error));
    }
  }
  function getAppInfo(): Promise<AppInfo> {
    handshake ??= call("app_info").catch((error) => {
      handshake = undefined;
      throw error;
    });
    return handshake;
  }
  async function getDatabaseInfo() {
    const info = await getAppInfo();
    if (!info.capabilities.includes("database_info"))
      throw failure("CAPABILITY_UNAVAILABLE");
    return call("database_info");
  }
  async function capable<K extends ImplementedCommand>(
    command: K,
    request: unknown,
  ): Promise<Responses[K]> {
    const info = await getAppInfo();
    if (!info.capabilities.includes(command))
      throw failure("CAPABILITY_UNAVAILABLE");
    return call(command, request);
  }
  async function requireDht(enabled: boolean) {
    // The magnet-resolution capability was introduced with optional DHT.
    // Older v1 backends must never silently ignore a requested DHT action.
    if (
      enabled &&
      !(await getAppInfo()).capabilities.includes("magnet_resolve")
    )
      throw failure("CAPABILITY_UNAVAILABLE");
  }
  async function requireTrackers(trackers: string[] | undefined) {
    if (trackers?.length) await capable("trackers_validate", { trackers });
  }
  async function requireLimits(limits: BandwidthLimits | undefined) {
    if (!limits) return;
    if (
      ![limits.downloadBps, limits.uploadBps].every(
        (n) =>
          Number.isInteger(n) &&
          n >= 0 &&
          n <= 4294967295 &&
          (n === 0 || n >= 16384),
      )
    )
      throw failure("INVALID_ARGUMENT");
    if (
      (limits.downloadBps || limits.uploadBps) &&
      !(await getAppInfo()).capabilities.includes("transfer_set_limits")
    )
      throw failure("CAPABILITY_UNAVAILABLE");
  }
  return {
    verifyItem: (request: StartVerifyRequest) =>
      capable("library_verify", request),
    getIntegrity: (request: ExportManifestRequest) =>
      capable("library_integrity", request),
    exportPackage: (request: ExportManifestRequest) =>
      capable("library_export_package", request),
    openPackage: () => capable("package_open", undefined),
    exportEver: (request: ExportManifestRequest) =>
      capable("library_export_ever", request),
    openEver: () => capable("ever_open", undefined),
    getTransferMetricsInfo: () => capable("transfer_metrics_info", undefined),
    validateTrackers: (request: TrackerList) =>
      capable("trackers_validate", request),
    generateMagnet: async (request: TorrentReferenceRequest) => {
      await requireTrackers(request.trackers);
      return capable("magnet_generate", request);
    },
    resolveMagnet: async (request: ResolveMagnetRequest) => {
      await requireTrackers(request.trackers);
      return capable("magnet_resolve", request);
    },
    cancelMagnet: (request: CancelMagnetRequest) =>
      capable("magnet_cancel", request),
    createTorrent: async (request: TorrentReferenceRequest) => {
      await requireTrackers(request.trackers);
      return capable("torrent_create", request);
    },
    importTorrent: () => capable("torrent_import", undefined),
    seedTorrent: async (request: StartSeedRequest) => {
      await requireLimits(request.limits);
      await requireDht(request.dht);
      await requireTrackers(request.trackers);
      return capable("torrent_seed", request);
    },
    downloadTorrent: async (request: StartDownloadRequest) => {
      await requireLimits(request.limits);
      if (
        request.seedAfterDownload &&
        !(await capable("transfer_metrics_info", undefined)).seedAfterDownload
      )
        throw failure("CAPABILITY_UNAVAILABLE");
      await requireDht(request.dht);
      await requireTrackers(request.trackers);
      return capable("torrent_download", request);
    },
    setTransferLimits: async (request: TransferLimitsRequest) => {
      await requireLimits(request.limits);
      return capable("transfer_set_limits", request);
    },
    listTransfers: () => capable("transfers_list", undefined),
    pauseTransfer: (request: TransferIdRequest) =>
      capable("transfer_pause", request),
    resumeTransfer: (request: TransferIdRequest) =>
      capable("transfer_resume", request),
    forgetTransfer: (request: TransferIdRequest) =>
      capable("transfer_forget", request),
    stopTransfer: (request: TransferIdRequest) =>
      capable("transfer_stop", request),
    getAppInfo,
    getDatabaseInfo,
    saveLibraryBackup: () => capable("library_save_backup", undefined),
    prepareLibraryRestore: () => capable("library_prepare_restore", undefined),
    restoreLibraryBackup: (request: LibraryRestoreRequest) =>
      capable("library_restore_backup", request),
    selectSources: (request: SelectSourcesRequest) =>
      capable("library_select_sources", request),
    importSources: (request: StartImportRequest) =>
      capable("library_import", request),
    getItemFiles: (request: LibraryItemFilesRequest) =>
      capable("library_item_files", request),
    listLibrary: (request: LibraryListRequest) =>
      capable("library_list", request),
    listTasks: (request: PageRequest) => capable("tasks_list", request),
    getTask: (request: TaskIdRequest) => capable("task_get", request),
    exportManifest: (request: ExportManifestRequest) =>
      capable("library_export_manifest", request),
  };
}
