import { invoke } from "@tauri-apps/api/core";
import {
  failure,
  normalizeIpcError,
  parseArchiveResponse,
  type ArchiveRequest,
  type ArchiveResponse,
  type ArchiveGameCard,
  type ArchiveFileState,
} from "@game4/ipc-contracts";
import { getAppInfo, isTauri } from "./desktop";
const queries = new Map<string, { time: number; response: ArchiveResponse }>();
export async function archiveRequest(
  request: ArchiveRequest,
  cache = false,
): Promise<ArchiveResponse> {
  if (!isTauri()) throw failure("IPC_UNAVAILABLE");
  const key = JSON.stringify(request),
    saved = queries.get(key);
  if (cache && saved && Date.now() - saved.time < 30000) return saved.response;
  try {
    if (!(await getAppInfo()).capabilities.includes("archive_request"))
      throw failure("CAPABILITY_UNAVAILABLE");
    const response = parseArchiveResponse(
      await invoke("archive_request", { request }),
    );
    if (cache) {
      if (queries.size >= 20) queries.delete(queries.keys().next().value!);
      queries.set(key, { time: Date.now(), response });
    }
    return response;
  } catch (error) {
    throw normalizeIpcError(error);
  }
}
export function invalidateArchive() {
  queries.clear();
}
export type GameDetail = Extract<ArchiveResponse, { kind: "game" }>;
export type ArchiveOverview = Extract<ArchiveResponse, { kind: "overview" }>;
export function preservationState(
  states: ArchiveFileState[],
  expectedFileIds?: string[],
): "verified" | "atRisk" | "unknown" {
  if (
    states.some((s) =>
      ["changed", "missing", "inaccessible"].includes(s.status),
    )
  )
    return "atRisk";
  if (expectedFileIds?.some((id) => !states.some((s) => s.fileId === id)))
    return "unknown";
  if (states.length > 0 && states.every((s) => s.status === "verified"))
    return "verified";
  return "unknown";
}
export function availability(card: ArchiveGameCard) {
  return card.localFiles > 0
    ? "local"
    : card.sourceCount > 0
      ? "reference"
      : "metadata";
}
export function preference(key: string, fallback: string, allowed?: string[]) {
  try {
    const v = localStorage.getItem(`game4.archive.${key}`);
    return v && (!allowed || allowed.includes(v)) ? v : fallback;
  } catch {
    return fallback;
  }
}
export function savePreference(key: string, value: string) {
  try {
    localStorage.setItem(`game4.archive.${key}`, value);
  } catch {
    /* Read-only profiles remain usable. */
  }
}
