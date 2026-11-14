import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { EverSummary } from "@game4/ipc-contracts";
import { openEver, isTauri } from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
import { formatBytes } from "../services/file-identification";
import { useNotifications } from "./Notifications";
export function EverReader({
  disabled,
  onUseMagnet,
}: {
  disabled: boolean;
  onUseMagnet?: (magnet: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const { notify } = useNotifications();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<EverSummary>();
  const [error, setError] = useState<unknown>();
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  async function read() {
    setBusy(true);
    setResult(undefined);
    setError(undefined);
    try {
      const value = await openEver();
      if (mounted.current && value.manifest) {
        setResult(value.manifest);
        notify({ key: "notifications.referencesChecked", kind: "success" });
      }
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  return (
    <section className="content-card ever-reader">
      <h2>{t("ever.title")}</h2>
      <p>{t("ever.description")}</p>
      <button
        disabled={disabled || busy || !isTauri()}
        onClick={() => void read()}
      >
        {t("ever.open")}
      </button>
      {busy && <p role="status">{t("ever.reading")}</p>}
      {error !== undefined && <p role="alert">{t(getIpcErrorKey(error))}</p>}
      {result && (
        <div role="status">
          <h3>{result.title}</h3>
          <p>
            {result.fileName} · .g4e v{result.schemaVersion}
          </p>
          <p>
            {t(`ever.${result.representation}`)} ·{" "}
            {t("library.fileCount", { count: result.fileCount })} ·{" "}
            {formatBytes(result.totalBytes, i18n.language)}
          </p>
          <p>{t("ever.descriptorOnly")}</p>
          {result.magnets.map((magnet, i) => (
            <div key={i}>
              <textarea
                readOnly
                aria-label={`${t("ever.magnet")} ${i + 1}`}
                value={magnet}
              />
              {onUseMagnet && (
                <button
                  disabled={disabled || busy}
                  onClick={() => onUseMagnet(magnet)}
                >
                  {t("ever.useLink")}
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
