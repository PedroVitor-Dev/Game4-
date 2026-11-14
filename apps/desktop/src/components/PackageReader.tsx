import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { PackageSummary } from "@game4/ipc-contracts";
import { isTauri, openPackage } from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
import { formatBytes } from "../services/file-identification";
import { useNotifications } from "./Notifications";
export function PackageReader({ disabled }: { disabled: boolean }) {
  const { t, i18n } = useTranslation();
  const { notify } = useNotifications();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<PackageSummary>();
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
    setError(undefined);
    setResult(undefined);
    try {
      const value = await openPackage();
      if (mounted.current && value.package) {
        setResult(value.package);
        notify({ key: "notifications.packageChecked", kind: "success" });
      }
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  return (
    <section
      className="content-card package-reader"
      aria-labelledby="package-reader-title"
    >
      <h2 id="package-reader-title">{t("packages.title")}</h2>
      <p>{t("packages.description")}</p>
      <button
        disabled={disabled || busy || !isTauri()}
        onClick={() => void read()}
      >
        {t("packages.open")}
      </button>
      {busy && <p role="status">{t("packages.reading")}</p>}
      {error !== undefined && <p role="alert">{t(getIpcErrorKey(error))}</p>}
      {result && (
        <div role="status">
          <h3>{result.title}</h3>
          <p>
            {result.fileName} · .g4 v{result.packageVersion}
          </p>
          <p>
            {t("library.fileCount", { count: result.fileCount })} ·{" "}
            {formatBytes(result.totalBytes, i18n.language)}
          </p>
          <p>{t("packages.verified")}</p>
          <p>{t("packages.readOnly")}</p>
        </div>
      )}
    </section>
  );
}
