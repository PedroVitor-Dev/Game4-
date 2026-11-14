import type { ArchiveSource, TransferSnapshot } from "@game4/ipc-contracts";
export function matchingArchiveTransfers(
  sources: Pick<ArchiveSource, "magnet">[],
  transfers: TransferSnapshot[],
): TransferSnapshot[] {
  const hashes = new Set(
    sources.flatMap((source) => {
      try {
        const magnet = new URL(source.magnet);
        const values = magnet.searchParams.getAll("xt");
        const value = values.length === 1 ? values[0] : "";
        return magnet.protocol === "magnet:" &&
          /^urn:btih:[a-f0-9]{40}$/i.test(value)
          ? [value.slice(9).toLowerCase()]
          : [];
      } catch {
        return [];
      }
    }),
  );
  return transfers.filter((transfer) =>
    hashes.has(transfer.infoHash.toLowerCase()),
  );
}
