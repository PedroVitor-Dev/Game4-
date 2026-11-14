import { G4epManifestDetails } from "./G4epManifestDetails";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  AssociatedFile,
  AssociatedFileResult,
} from "@game4/ipc-contracts";
import { failure } from "@game4/ipc-contracts";
import {
  isTauri,
  getAppInfo,
  nextAssociatedFile,
  readAssociatedFile,
} from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
import { formatBytes } from "../services/file-identification";
import { DownloadLink } from "./DownloadLink";
export function AssociatedFiles({
  onDownloadLink,
}: {
  onDownloadLink: (magnet: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const [file, setFile] = useState<AssociatedFile>();
  const [result, setResult] = useState<AssociatedFileResult>();
  const [error, setError] = useState<unknown>();
  const [busy, setBusy] = useState(false);
  const displayed = useRef(false);
  useEffect(() => {
    if (!isTauri()) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        if (!displayed.current) {
          const pending = await nextAssociatedFile();
          if (!alive) return;
          if (pending) {
            displayed.current = true;
            window.scrollTo({ top: 0 });
            setFile(pending);
            setResult(undefined);
            setError(undefined);
            setBusy(true);
            try {
              const inspected = await readAssociatedFile({
                token: pending.token,
              });
              if (inspected.kind !== pending.kind)
                throw failure("INVALID_RESPONSE");
              if (alive) setResult(inspected);
            } catch (e) {
              if (alive) setError(e);
            } finally {
              if (alive) setBusy(false);
            }
          }
        }
      } catch (e) {
        if (alive) setError(e);
      }
      if (alive) timer = setTimeout(() => void poll(), 1000);
    }
    void getAppInfo()
      .then((info) => {
        if (
          alive &&
          info.capabilities.includes("associated_file_next") &&
          info.capabilities.includes("associated_file_read")
        )
          void poll();
      })
      .catch(() => {});
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, []);
  if (!file) return null;
  const summary = result?.package ?? result?.manifest;
  function dismiss() {
    setFile(undefined);
    setResult(undefined);
    setError(undefined);
    displayed.current = false;
  }
  return (
    <section
      className="content-card associated-file"
      aria-labelledby="associated-title"
    >
      <div className="session-heading">
        <h2 id="associated-title">{t("associations.opened")}</h2>
        <button disabled={busy} onClick={dismiss}>
          {t("associations.close")}
        </button>
      </div>
      <p className="associated-filename">{file.fileName}</p>
      {busy && (
        <p role="status">
          {t(file.kind === "g4" ? "packages.reading" : "ever.reading")}
        </p>
      )}
      {error !== undefined && (
        <p className="error-message" role="alert">
          {t(getIpcErrorKey(error))}
        </p>
      )}
      {summary && (
        <>
          <h3>{summary.title}</h3>
          <p>
            {t("library.fileCount", { count: summary.fileCount })} ·{" "}
            {formatBytes(summary.totalBytes, i18n.language)}
          </p>
          <p>
            {t(
              result?.kind === "g4"
                ? "packages.verified"
                : "ever.descriptorOnly",
            )}
          </p>
          <p>{t("associations.readOnly")}</p>
          {result?.manifest?.g4ep && (
            <G4epManifestDetails summary={result.manifest.g4ep} />
          )}
        </>
      )}
      {result?.manifest?.magnets.map((magnet, index) => (
        <div className="associated-download" key={index}>
          <DownloadLink value={magnet} />
          <button
            className="dashboard-primary"
            onClick={() => {
              onDownloadLink(magnet);
              dismiss();
            }}
          >
            {t("ever.useLink")}
          </button>
        </div>
      ))}
    </section>
  );
}
