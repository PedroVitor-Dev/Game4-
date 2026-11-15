import type { GuardianSnapshot } from "./generated.ts";
import { failure } from "./validation.ts";
export function parseGuardianSnapshot(value: unknown): GuardianSnapshot {
  const fail = (): never => {
    throw failure("INVALID_RESPONSE");
  };
  const obj = (v: unknown): Record<string, unknown> =>
    v !== null && typeof v === "object" && !Array.isArray(v)
      ? (v as Record<string, unknown>)
      : fail();
  const keys = (o: Record<string, unknown>, expected: string[]) => {
    if (Object.keys(o).sort().join() !== [...expected].sort().join()) fail();
  };
  const text = (v: unknown, max = 4096): v is string =>
    typeof v === "string" && v.length <= max && !/[\u0000-\u001f]/.test(v);
  const integer = (v: unknown, max: number): v is number =>
    typeof v === "number" && Number.isSafeInteger(v) && v >= 0 && v <= max;
  const decimal = (v: unknown, max: bigint) =>
    typeof v === "string" &&
    /^(0|[1-9][0-9]*)$/.test(v) &&
    v.length <= 20 &&
    BigInt(v) <= max;
  const hash = (v: unknown) =>
    typeof v === "string" && /^[0-9a-f]{64}$/.test(v);
  const d = obj(value),
    c = obj(d.config);
  keys(d, [
    "nodeId",
    "directory",
    "config",
    "usedBytes",
    "fragmentCount",
    "health",
    "entries",
    "offset",
    "total",
    "networkStarted",
  ]);
  keys(c, ["active", "quotaBytes", "uploadBps", "downloadBps"]);
  if (
    !hash(d.nodeId) ||
    !text(d.directory, 32768) ||
    !d.directory ||
    typeof c.active !== "boolean" ||
    !decimal(c.quotaBytes, 1n << 40n) ||
    !integer(c.uploadBps, 1e9) ||
    !integer(c.downloadBps, 1e9) ||
    !decimal(d.usedBytes, (1n << 63n) - 1n) ||
    !integer(d.fragmentCount, 100000) ||
    !["ready", "attention", "unchecked"].includes(d.health as string) ||
    !integer(d.offset, 4294967295) ||
    !integer(d.total, 100000) ||
    d.networkStarted !== false ||
    !Array.isArray(d.entries) ||
    d.entries.length > 100
  )
    fail();
  let last = 0;
  for (const raw of d.entries as unknown[]) {
    const e = obj(raw);
    keys(e, [
      "id",
      "contentId",
      "name",
      "group",
      "index",
      "sha256",
      "sizeBytes",
      "sharing",
      "health",
    ]);
    if (
      !integer(e.id, Number.MAX_SAFE_INTEGER) ||
      e.id <= last ||
      !text(e.contentId, 75) ||
      !/^urn:sha256:[0-9a-f]{64}$/.test(e.contentId) ||
      !text(e.name, 255) ||
      !e.name ||
      /[\\/]/.test(e.name) ||
      !decimal(e.group, 83332n) ||
      !integer(e.index, 11) ||
      !hash(e.sha256) ||
      !integer(e.sizeBytes, 4194304) ||
      e.sizeBytes < 65536 ||
      e.sizeBytes % 64 !== 0 ||
      typeof e.sharing !== "boolean" ||
      !["unknown", "verified", "missing", "corrupt"].includes(
        e.health as string,
      )
    )
      fail();
    last = e.id as number;
  }
  if (
    (d.entries as unknown[]).length >
    Math.max(0, (d.total as number) - (d.offset as number))
  )
    fail();
  return value as GuardianSnapshot;
}
