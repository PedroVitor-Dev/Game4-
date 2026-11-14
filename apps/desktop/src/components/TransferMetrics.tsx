import { useTranslation } from "react-i18next";
import type { TransferSnapshot } from "@game4/ipc-contracts";
import { formatBytes } from "../services/file-identification";
import {
  observedTelemetry,
  verifiedPercent,
} from "../services/transfer-metrics";

export function TransferMetrics({
  transfer: job,
  fresh,
}: {
  transfer: TransferSnapshot;
  fresh: boolean;
}) {
  const { t, i18n } = useTranslation();
  const bytes = (value: string) => formatBytes(value, i18n.language);
  const number = (value: number) => value.toLocaleString(i18n.language);
  const telemetry = observedTelemetry(job, fresh);
  const percent = verifiedPercent(job.verifiedBytes, job.totalBytes);
  return (
    <section
      className="transfer-metrics-panel"
      aria-label={t("transfers.measurements")}
    >
      <div className="verified-heading">
        <div>
          <span>{t("transfers.pieces")}</span>
          <p>
            {bytes(job.verifiedBytes)} <span>/ {bytes(job.totalBytes)}</span>
          </p>
        </div>
        <strong>
          {percent.toLocaleString(i18n.language, {
            minimumFractionDigits: 1,
            maximumFractionDigits: 1,
          })}
          %
        </strong>
      </div>
      <progress max={100} value={percent} aria-label={t("transfers.pieces")} />
      <dl className="live-metric-grid">
        <div>
          <dt>
            <span aria-hidden="true">↓</span> {t("transfers.downloadSpeed")}
          </dt>
          <dd>{telemetry ? `${bytes(telemetry.downloadBps)}/s` : "—"}</dd>
        </div>
        <div>
          <dt>
            <span aria-hidden="true">↑</span> {t("transfers.uploadSpeed")}
          </dt>
          <dd>{telemetry ? `${bytes(telemetry.uploadBps)}/s` : "—"}</dd>
        </div>
        <div>
          <dt>{t("transfers.peersLabel")}</dt>
          <dd>{telemetry ? number(telemetry.connectedPeers) : "—"}</dd>
        </div>
        <div>
          <dt>{t("transfers.seedersLabel")}</dt>
          <dd>{telemetry ? number(telemetry.seeders) : "—"}</dd>
        </div>
        <div>
          <dt>{t("transfers.leechersLabel")}</dt>
          <dd>{telemetry ? number(telemetry.leechers) : "—"}</dd>
        </div>
      </dl>
      <div className="transfer-byte-totals">
        <span>
          {t("transfers.received", { bytes: bytes(job.receivedBytes) })}
        </span>
        <span>
          {t("transfers.uploadedTotal", { bytes: bytes(job.uploadedBytes) })}
        </span>
      </div>
      <p className="metric-note">
        {!fresh
          ? t("transfers.staleMetrics")
          : !telemetry
            ? t("transfers.metricsUnavailable")
            : t("transfers.liveScope")}
      </p>
      {telemetry && (
        <details className="peer-explanation">
          <summary>{t("transfers.availabilityLabel")}</summary>
          <p>
            {t(
              telemetry.seeders > 0
                ? "transfers.availabilityFull"
                : "transfers.availabilityUnknown",
              { count: telemetry.seeders },
            )}
          </p>
          <p>{t("transfers.peerScope")}</p>
        </details>
      )}
    </section>
  );
}
