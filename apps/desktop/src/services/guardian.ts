import { invoke } from "@tauri-apps/api/core";
import {
  failure,
  normalizeIpcError,
  parseGuardianSnapshot,
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
