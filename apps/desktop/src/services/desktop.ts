import { invoke, isTauri } from "@tauri-apps/api/core";
import { createIpcClient } from "./ipc-client.ts";

export { isTauri };
export type { AppInfo, DatabaseInfo, IpcError } from "@game4/ipc-contracts";

const client = createIpcClient({
  isAvailable: isTauri,
  invoke: (command, args) => invoke<unknown>(command, args),
});
export const getAppInfo = client.getAppInfo;
export const getDatabaseInfo = client.getDatabaseInfo;

export const { selectSources, importSources, listLibrary, listTasks, getTask } =
  client;

export const exportManifest = client.exportManifest;

export const {
  saveLibraryBackup,
  prepareLibraryRestore,
  restoreLibraryBackup,
} = client;

export const getItemFiles = client.getItemFiles;

export const { verifyItem, getIntegrity } = client;

export const { exportPackage, openPackage } = client;

export const { exportEver, openEver } = client;

export const {
  getTransferMetricsInfo,
  validateTrackers,
  generateMagnet,
  resolveMagnet,
  cancelMagnet,
  createTorrent,
  importTorrent,
  seedTorrent,
  downloadTorrent,
  listTransfers,
  setTransferLimits,
  stopTransfer,
  pauseTransfer,
  resumeTransfer,
  forgetTransfer,
} = client;
