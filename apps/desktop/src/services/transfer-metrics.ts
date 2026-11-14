import type { TransferSnapshot } from "@game4/ipc-contracts";

/** Floor using exact byte integers so incomplete payloads never display 100%. */
export function verifiedPercent(
  verifiedBytes: string,
  totalBytes: string,
): number {
  const total = BigInt(totalBytes);
  if (total <= 0n) return 0;
  const verified = BigInt(verifiedBytes);
  return (
    Number(
      ((verified < 0n ? 0n : verified > total ? total : verified) * 1000n) /
        total,
    ) / 10
  );
}

export function observedTelemetry(job: TransferSnapshot, fresh: boolean) {
  return fresh && job.status === "active" ? job.telemetry : null;
}

/** KiB/s input converted to exact integer bytes/s for the native u32 limiter. */
export function parseBandwidthInput(value: string): number | null {
  if (!/^\d+(?:\.\d{1,10})?$/.test(value)) return null;
  const [whole, fraction = ""] = value.split(".");
  const divisor = 10n ** BigInt(fraction.length);
  const scaled = (BigInt(whole) * divisor + BigInt(fraction || "0")) * 1024n;
  if (scaled % divisor !== 0n) return null;
  const bps = scaled / divisor;
  return bps <= 4294967295n && (bps === 0n || bps >= 16384n)
    ? Number(bps)
    : null;
}
