import type { TaskSnapshot, TransferSnapshot } from "@game4/ipc-contracts";
export type NoticeInput = {
  key: string;
  kind?: "success" | "info" | "error";
  params?: Record<string, string | number>;
  target?: "tasks" | "transfers" | "settings";
  dedupeKey?: string;
};
export type Notice = NoticeInput & { id: number; unread: boolean };
export function addNotice(
  items: Notice[],
  input: NoticeInput,
  id: number,
): Notice[] {
  if (
    input.dedupeKey &&
    items.some((item) => item.dedupeKey === input.dedupeKey)
  )
    return items;
  return [...items, { ...input, id, unread: true }].slice(-30);
}
export function observeTasks(
  previous: Map<string, Pick<TaskSnapshot, "status" | "kind">>,
  items: TaskSnapshot[],
): NoticeInput[] {
  const notices: NoticeInput[] = [];
  for (const item of items) {
    const old = previous.get(item.id);
    if (
      old &&
      old.status !== item.status &&
      ["queued", "running"].includes(old.status)
    ) {
      if (item.status === "succeeded")
        notices.push({
          key:
            item.kind === "verify"
              ? "notifications.filesChecked"
              : "notifications.importComplete",
          kind: "success",
          target: "tasks",
          dedupeKey: `task:${item.id}:succeeded`,
        });
      if (["failed", "interrupted", "cancelled"].includes(item.status))
        notices.push({
          key:
            item.status === "cancelled"
              ? "notifications.taskCancelled"
              : "notifications.taskFailed",
          kind: item.status === "cancelled" ? "info" : "error",
          target: "tasks",
          dedupeKey: `task:${item.id}:${item.status}`,
        });
    }
    previous.set(item.id, { status: item.status, kind: item.kind });
  }
  // Recent native pages are bounded; retain a modest number of explicitly watched jobs.
  if (previous.size > 500)
    for (const [id, item] of previous) {
      if (!["queued", "running"].includes(item.status)) previous.delete(id);
      if (previous.size <= 400) break;
    }
  return notices;
}
export function observeTransfers(
  previous: Map<string, Pick<TransferSnapshot, "status" | "isSeeding">>,
  items: TransferSnapshot[],
): NoticeInput[] {
  const notices: NoticeInput[] = [];
  for (const item of items) {
    const old = previous.get(item.id);
    if (
      old &&
      item.mode === "download" &&
      (item.status === "completed" || item.isSeeding) &&
      old.status !== "completed" &&
      !old.isSeeding
    )
      notices.push({
        key: "notifications.downloadComplete",
        kind: "success",
        params: { name: item.name },
        target: "transfers",
        dedupeKey: `transfer:${item.id}:completed`,
      });
    if (old && item.status === "failed" && old.status !== "failed")
      notices.push({
        key: "notifications.transferFailed",
        kind: "error",
        params: { name: item.name },
        target: "transfers",
        dedupeKey: `transfer:${item.id}:failed`,
      });
    previous.set(item.id, { status: item.status, isSeeding: item.isSeeding });
  }
  const ids = new Set(items.map((item) => item.id));
  for (const id of previous.keys()) if (!ids.has(id)) previous.delete(id);
  return notices;
}
