import {
  failure,
  IpcFailure,
  normalizeIpcError,
  parseAppInfo,
  parseDatabaseInfo,
} from "@game4/ipc-contracts";
import type { AppInfo, ImplementedCommand } from "@game4/ipc-contracts";

export type IpcTransport = {
  isAvailable: () => boolean;
  invoke: (command: ImplementedCommand) => Promise<unknown>;
};
const decoders = {
  app_info: parseAppInfo,
  database_info: parseDatabaseInfo,
} satisfies Record<ImplementedCommand, (value: unknown) => unknown>;
type Responses = {
  [K in ImplementedCommand]: ReturnType<(typeof decoders)[K]>;
};

export function createIpcClient(transport: IpcTransport) {
  let handshake: Promise<AppInfo> | undefined;
  async function call<K extends ImplementedCommand>(
    command: K,
  ): Promise<Responses[K]> {
    if (!transport.isAvailable()) throw failure("IPC_UNAVAILABLE");
    try {
      return decoders[command](await transport.invoke(command)) as Responses[K];
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
  return { getAppInfo, getDatabaseInfo };
}
