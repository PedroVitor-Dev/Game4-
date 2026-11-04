import {
  failure,
  IpcFailure,
  normalizeIpcError,
  parseAppInfo,
  parseDatabaseInfo,
  parseSourceSelection,
  parseTaskAccepted,
  parseLibraryPage,
  parseTaskPage,
  parseTaskSnapshot,
  parseExportManifestResult,
} from "@game4/ipc-contracts";
import type {
  AppInfo,
  ImplementedCommand,
  SelectSourcesRequest,
  StartImportRequest,
  LibraryListRequest,
  PageRequest,
  TaskIdRequest,
  ExportManifestRequest,
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
  library_list: parseLibraryPage,
  tasks_list: parseTaskPage,
  task_get: parseTaskSnapshot,
  library_export_manifest: parseExportManifestResult,
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
  return {
    getAppInfo,
    getDatabaseInfo,
    selectSources: (request: SelectSourcesRequest) =>
      capable("library_select_sources", request),
    importSources: (request: StartImportRequest) =>
      capable("library_import", request),
    listLibrary: (request: LibraryListRequest) =>
      capable("library_list", request),
    listTasks: (request: PageRequest) => capable("tasks_list", request),
    getTask: (request: TaskIdRequest) => capable("task_get", request),
    exportManifest: (request: ExportManifestRequest) =>
      capable("library_export_manifest", request),
  };
}
