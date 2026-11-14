import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ArchiveSource, TransferSnapshot } from "@game4/ipc-contracts";
import { listTransfers } from "../../services/desktop";
import { matchingArchiveTransfers } from "../../services/archive-metrics";
import { TransferMetrics } from "../TransferMetrics";
import { ErrorNotice } from "../Feedback";
export function ArchiveLiveTransfers({
  sources,
}: {
  sources: ArchiveSource[];
}) {
  const { t } = useTranslation();
  const [items, setItems] = useState<TransferSnapshot[]>([]);
  const [fresh, setFresh] = useState(false);
  const [error, setError] = useState<unknown>();
  const [retry, setRetry] = useState(0);
  const identity = sources
    .map((source) => source.magnet)
    .sort()
    .join("\n");
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const current = identity
      .split("\n")
      .filter(Boolean)
      .map((magnet) => ({ magnet }));
    setItems([]);
    setFresh(false);
    async function read() {
      try {
        const result = await listTransfers();
        if (active) {
          setItems(matchingArchiveTransfers(current, result.items));
          setFresh(true);
          setError(undefined);
        }
      } catch (e) {
        if (active) {
          setFresh(false);
          setError(e);
        }
      } finally {
        if (active) timer = setTimeout(() => void read(), 3000);
      }
    }
    if (current.length) void read();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [identity, retry]);
  if (!identity) return null;
  return (
    <section className="content-card archive-live-transfers">
      <h3>{t("archive.liveForThisGame")}</h3>
      <p>{t("archive.liveReferenceScope")}</p>
      {error !== undefined && (
        <ErrorNotice error={error} onRetry={() => setRetry((n) => n + 1)} />
      )}
      {items.length === 0 && <p>{t("archive.noMatchingTransfer")}</p>}
      {items.map((item) => (
        <article key={item.id}>
          <div className="archive-section-heading">
            <strong>{item.name}</strong>
            <span>{t(`transfers.status.${item.status}`)}</span>
          </div>
          <TransferMetrics transfer={item} fresh={fresh} />
        </article>
      ))}
    </section>
  );
}
