import { invoke, isTauri } from "@tauri-apps/api/core";

export { isTauri };

export type AppInfo = { version: string };
export type DatabaseInfo = {
  schemaVersion: number;
  fileCount: number;
  locationCount: number;
  itemCount: number;
  taskCount: number;
};

export const getAppInfo = () => invoke<AppInfo>("app_info");
export const getDatabaseInfo = () => invoke<DatabaseInfo>("database_info");
