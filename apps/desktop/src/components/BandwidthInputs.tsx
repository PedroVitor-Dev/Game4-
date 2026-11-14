import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import type { TransferSnapshot } from "@game4/ipc-contracts";
import { setTransferLimits } from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
import { parseBandwidthInput } from "../services/transfer-metrics";

export function BandwidthInputs({
  download,
  upload,
  onDownload,
  onUpload,
  disabled = false,
}: {
  download: string;
  upload: string;
  onDownload: (value: string) => void;
  onUpload: (value: string) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const hintId = useId();
  return (
    <div className="bandwidth-inputs">
      <div className="bandwidth-fields">
        {(
          [
            ["download", download, onDownload],
            ["upload", upload, onUpload],
          ] as const
        ).map(([direction, value, onChange]) => (
          <label key={direction}>
            {t(`transfers.${direction}Limit`)}
            <input
              type="number"
              min="0"
              max="4194303.9990234375"
              step="any"
              value={value}
              disabled={disabled}
              onChange={(e) => onChange(e.target.value)}
              aria-describedby={hintId}
              aria-invalid={parseBandwidthInput(value) === null}
            />
          </label>
        ))}
      </div>
      <p id={hintId}>{t("transfers.limitsHint")}</p>
    </div>
  );
}

export function TransferLimitsControl({
  transfer,
  onSaved,
}: {
  transfer: TransferSnapshot;
  onSaved: () => Promise<void>;
}) {
  const { t } = useTranslation();
  const [download, setDownload] = useState(
    String(transfer.limits.downloadBps / 1024),
  );
  const [upload, setUpload] = useState(
    String(transfer.limits.uploadBps / 1024),
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>();
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    setDownload(String(transfer.limits.downloadBps / 1024));
    setUpload(String(transfer.limits.uploadBps / 1024));
  }, [transfer.limits.downloadBps, transfer.limits.uploadBps]);
  const disabled =
    busy || transfer.status === "resuming" || transfer.status === "stopping";
  async function save(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const downloadBps = parseBandwidthInput(download),
      uploadBps = parseBandwidthInput(upload);
    if (downloadBps === null || uploadBps === null) return;
    setBusy(true);
    setError(undefined);
    setSaved(false);
    try {
      await setTransferLimits({
        transferId: transfer.id,
        limits: { downloadBps, uploadBps },
      });
      setSaved(true);
      await onSaved();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <details className="session-bandwidth">
      <summary>{t("transfers.sessionLimits")}</summary>
      <form onSubmit={(e) => void save(e)}>
        <BandwidthInputs
          download={download}
          upload={upload}
          onDownload={(v) => {
            setDownload(v);
            setSaved(false);
          }}
          onUpload={(v) => {
            setUpload(v);
            setSaved(false);
          }}
          disabled={disabled}
        />
        <p>{t("transfers.savedLimitsHint")}</p>
        <button
          type="submit"
          disabled={
            disabled ||
            parseBandwidthInput(download) === null ||
            parseBandwidthInput(upload) === null
          }
        >
          {t(busy ? "transfers.savingLimits" : "transfers.applyLimits")}
        </button>
        {saved && <p role="status">{t("transfers.limitsSaved")}</p>}
        {error !== undefined && <p role="alert">{t(getIpcErrorKey(error))}</p>}
      </form>
    </details>
  );
}
