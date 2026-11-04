import { invoke, isTauri } from "@tauri-apps/api/core";
import { createIpcClient } from "./ipc-client.ts";

export { isTauri };
export type { AppInfo, DatabaseInfo, IpcError } from "@game4/ipc-contracts";

const client = createIpcClient({
  isAvailable: isTauri,
  invoke: (command) => invoke<unknown>(command),
});
export const getAppInfo = client.getAppInfo;
export const getDatabaseInfo = client.getDatabaseInfo;
