import { invoke } from "@tauri-apps/api/core";
import {
  failure,
  normalizeIpcError,
  parseGuardianSnapshot,
  parseNetworkSnapshot,
  type GuardianNetworkRequest,
  type GuardianRequest,
} from "@game4/ipc-contracts";
import { getAppInfo, isTauri } from "./desktop";
export async function guardianRequest(request: GuardianRequest) {
  if (!isTauri()) throw failure("IPC_UNAVAILABLE");
  try {
    if (!(await getAppInfo()).capabilities.includes("guardian_request"))
      throw failure("CAPABILITY_UNAVAILABLE");
    return parseGuardianSnapshot(await invoke("guardian_request", { request }));
  } catch (e) {
    throw normalizeIpcError(e);
  }
}

export async function guardianNetworkRequest(request: GuardianNetworkRequest) {
  if (!isTauri()) throw failure("IPC_UNAVAILABLE");
  try {
    if (!(await getAppInfo()).capabilities.includes("guardian_network_request"))
      throw failure("CAPABILITY_UNAVAILABLE");
    return parseNetworkSnapshot(
      await invoke("guardian_network_request", { request }),
    );
  } catch (e) {
    throw normalizeIpcError(e);
  }
}
