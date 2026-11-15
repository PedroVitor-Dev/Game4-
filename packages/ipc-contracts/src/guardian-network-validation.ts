import type { NetworkSnapshot } from "./generated.ts";
import { failure } from "./validation.ts";
export function parseNetworkSnapshot(value: unknown): NetworkSnapshot {
  const bad = (): never => {
    throw failure("INVALID_RESPONSE");
  };
  const obj = (v: unknown): Record<string, unknown> =>
    v !== null && typeof v === "object" && !Array.isArray(v)
      ? (v as Record<string, unknown>)
      : bad();
  const keys = (o: Record<string, unknown>, allowed: string[]) => {
    if (Object.keys(o).sort().join() !== [...allowed].sort().join()) bad();
  };
  const hash = (v: unknown) =>
    typeof v === "string" && /^[0-9a-f]{64}$/.test(v);
  const text = (v: unknown, max: number) =>
    typeof v === "string" &&
    v.length > 0 &&
    v.length <= max &&
    !/[\u0000-\u0020]/.test(v);
  const int = (v: unknown, min: number, max: number) =>
    typeof v === "number" && Number.isInteger(v) && v >= min && v <= max;
  const d = obj(value),
    c = obj(d.config);
  keys(d, [
    "running",
    "nodeId",
    "certificateSha256",
    "config",
    "connections",
    "outcome",
  ]);
  keys(c, ["bind", "maxConnections", "timeoutSeconds", "retries", "peers"]);
  if (
    typeof d.running !== "boolean" ||
    !hash(d.nodeId) ||
    !hash(d.certificateSha256) ||
    !text(c.bind, 100) ||
    !int(c.maxConnections, 1, 16) ||
    !int(c.timeoutSeconds, 2, 600) ||
    !int(c.retries, 0, 3) ||
    !int(d.connections, 0, c.maxConnections as number) ||
    !Array.isArray(c.peers) ||
    c.peers.length > 32 ||
    (!d.running && d.connections !== 0)
  )
    bad();
  const ids = new Set(),
    certs = new Set(),
    addresses = new Set();
  for (const raw of c.peers as unknown[]) {
    const p = obj(raw);
    keys(p, [
      "version",
      "nodeId",
      "address",
      "certificateSha256",
      "certificateDer",
    ]);
    if (
      p.version !== 1 ||
      !hash(p.nodeId) ||
      p.nodeId === d.nodeId ||
      !text(p.address, 100) ||
      !hash(p.certificateSha256) ||
      typeof p.certificateDer !== "string" ||
      p.certificateDer.length > 16384 ||
      !/^([0-9a-f]{2})+$/.test(p.certificateDer) ||
      ids.has(p.nodeId) ||
      certs.has(p.certificateSha256) ||
      addresses.has(p.address)
    )
      bad();
    ids.add(p.nodeId);
    certs.add(p.certificateSha256);
    addresses.add(p.address);
  }
  if (d.outcome !== null) {
    const o = obj(d.outcome);
    keys(o, ["operation", "peerId", "bytes", "attempts", "available"]);
    if (
      !["probe", "availability", "download", "upload"].includes(
        o.operation as string,
      ) ||
      !ids.has(o.peerId) ||
      typeof o.bytes !== "string" ||
      !/^(0|[1-9][0-9]*)$/.test(o.bytes) ||
      o.bytes.length > 8 ||
      BigInt(o.bytes) > 4194304n ||
      !int(o.attempts, 1, 4) ||
      (o.operation === "availability"
        ? typeof o.available !== "boolean"
        : o.available !== null)
    )
      bad();
  }
  return value as NetworkSnapshot;
}
