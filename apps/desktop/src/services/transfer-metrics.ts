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
