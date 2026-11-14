import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  FileFormat,
  LibraryFileInfo,
  LibraryItemSummary,
  LibraryIntegrity,
} from "@game4/ipc-contracts";
import {
  formatBytes,
  formatLabels,
  platformLabels,
} from "../services/file-identification";
import { getItemFiles, getIntegrity, isTauri } from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
export function LibraryItemCard({
  item,
  busy,
  onExport,
  onVerify,
  onPackage,
  onEver,
}: {
  item: LibraryItemSummary;
  busy: boolean;
  onExport: (id: string) => void;
  onVerify: (id: string) => void;
  onPackage: (id: string) => void;
  onEver: (id: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const [health, setHealth] = useState<LibraryIntegrity>();
  const [healthError, setHealthError] = useState<unknown>();
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [files, setFiles] = useState<LibraryFileInfo[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [error, setError] = useState<unknown>();
  const mounted = useRef(true);
  const generation = useRef(0);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      generation.current++;
    };
  }, []);
  useEffect(() => {
    generation.current++;
    setOpen(false);
    setFiles([]);
    setCursor(null);
    setLoading(false);
    setError(undefined);
  }, [item]);
  useEffect(() => {
    let stopped = false;
    setHealth(undefined);
    setHealthError(undefined);
    if (isTauri())
      void getIntegrity({ itemId: item.id }).then(
        (result) => {
          if (!stopped) setHealth(result);
        },
        (e: unknown) => {
          if (!stopped) setHealthError(e);
        },
      );
    return () => {
      stopped = true;
    };
  }, [item]);
  async function load(append = false) {
    const requestGeneration = ++generation.current;
    setLoading(true);
    setError(undefined);
    try {
      const result = await getItemFiles({
        itemId: item.id,
        page: { cursor: append ? cursor : null, limit: 50 },
      });
      if (!mounted.current || requestGeneration !== generation.current) return;
      setFiles((old) => (append ? [...old, ...result.files] : result.files));
      setCursor(result.nextCursor);
    } catch (e) {
      if (mounted.current && requestGeneration === generation.current)
        setError(e);
    } finally {
      if (mounted.current && requestGeneration === generation.current)
        setLoading(false);
    }
  }
  const formatName = (format: FileFormat) =>
    format === "unknown"
      ? t("identification.unknown")
      : format === "raw_binary"
        ? t("identification.rawBinary")
        : format === "raw_rom"
          ? t("identification.rawRom")
          : formatLabels[format];
  const exact = (bytes: string) =>
    t("library.exactSize", {
      bytes: BigInt(bytes).toLocaleString(i18n.language),
    });
  return (
    <article className={`content-card library-item ${open ? "expanded" : ""}`}>
      <div className="library-cover" aria-hidden="true">
        <span>{item.title.slice(0, 1).toLocaleUpperCase(i18n.language)}</span>
        <small>GAME4∞ / ARCHIVE</small>
      </div>
      <h2>{item.title}</h2>
      <p title={exact(item.totalBytes)}>
        {t("library.fileCount", { count: item.fileCount })} ·{" "}
        {formatBytes(item.totalBytes, i18n.language)}
        <span className="exact-size">{exact(item.totalBytes)}</span>
      </p>
      {item.platform && (
        <p>
          {t("library.declaredPlatform")}: {item.platform}
        </p>
      )}
      <details className="item-observations">
        <summary>{t("library.metadataDetails")}</summary>
        <div className="identification">
          <h3>{t("library.platforms")}</h3>
          {item.detectedPlatforms.length ? (
            item.detectedPlatforms.map((p) => (
              <span
                className="identification-chip"
                key={p.platform + p.evidence}
              >
                {platformLabels[p.platform]} ·{" "}
                {t(`identification.${p.evidence}`)} · {p.fileCount}
              </span>
            ))
          ) : (
            <span className="identification-unknown">
              {t("library.platformUnknown")}
            </span>
          )}
          <h3>{t("library.formats")}</h3>
          {item.formats.slice(0, 6).map((f) => (
            <span className="identification-chip" key={f.format + f.evidence}>
              {formatName(f.format)}
              {f.evidence !== "unknown" && (
                <> · {t(`identification.${f.evidence}`)}</>
              )}{" "}
              · {f.fileCount}
            </span>
          ))}
          {!item.formats.length && (
            <span className="identification-unknown">
              {t("identification.unknown")}
            </span>
          )}
          {item.formats.length > 6 && (
            <span>
              {t("library.otherFormats", { count: item.formats.length - 6 })}
            </span>
          )}
        </div>
        <section
          className="integrity-summary"
          aria-label={t("integrity.title")}
        >
          <h3>{t("integrity.title")}</h3>
          <p>{t("integrity.scope")}</p>
          {health && (
            <>
              {health.changed +
                health.missing +
                health.inaccessible +
                health.unlocated >
                0 && <p role="status">{t("integrity.issues")}</p>}
              <dl>
                {(
                  [
                    "verified",
                    "changed",
                    "missing",
                    "inaccessible",
                    "unverified",
                    "unlocated",
                  ] as const
                ).map((status) => (
                  <div key={status}>
                    <dt>{t(`integrity.${status}`)}</dt>
                    <dd>{health[status].toLocaleString(i18n.language)}</dd>
                  </div>
                ))}
              </dl>
              <p>
                {health.lastCheckedAt
                  ? t("integrity.lastChecked", {
                      date: new Date(health.lastCheckedAt).toLocaleString(
                        i18n.language,
                      ),
                    })
                  : t("integrity.neverChecked")}
              </p>
            </>
          )}
          {healthError !== undefined && (
            <p role="alert">{t(getIpcErrorKey(healthError))}</p>
          )}
        </section>
      </details>
      <span className="local-badge">{t("library.localItem")}</span>
      <div className="manifest-action actions">
        <button
          disabled={busy || !isTauri()}
          onClick={() => onEver(item.id)}
          aria-label={t("ever.exportLabel", { title: item.title })}
        >
          {t("ever.export")}
        </button>
        <button
          disabled={busy || !isTauri()}
          onClick={() => onPackage(item.id)}
          aria-label={t("packages.exportLabel", { title: item.title })}
        >
          {t("packages.export")}
        </button>
        <button
          disabled={busy || !isTauri()}
          onClick={() => onVerify(item.id)}
          aria-label={t("integrity.verifyLabel", { title: item.title })}
        >
          {t("integrity.verify")}
        </button>
        <button
          disabled={busy || !isTauri()}
          aria-label={t("library.exportManifestLabel", { title: item.title })}
          onClick={() => onExport(item.id)}
        >
          {t("library.exportManifest")}
        </button>
        <button
          disabled={!isTauri()}
          aria-expanded={open}
          onClick={() => {
            if (open) {
              generation.current++;
              setOpen(false);
              setLoading(false);
            } else {
              setOpen(true);
              void load();
            }
          }}
        >
          {t(open ? "library.hideFiles" : "library.viewFiles")}
        </button>
      </div>
      {open && (
        <section
          className="item-files"
          aria-label={t("library.filesFor", { title: item.title })}
        >
          {loading && <p role="status">{t("library.loadingFiles")}</p>}
          {error !== undefined && (
            <p role="alert">
              {t(getIpcErrorKey(error))}{" "}
              <button onClick={() => void load()}>
                {t("library.refresh")}
              </button>
            </p>
          )}
          <ul>
            {files.map((file) => (
              <li key={file.path}>
                <div className="file-heading">
                  <strong>{file.path}</strong>
                  <span title={exact(file.sizeBytes)}>
                    {formatBytes(file.sizeBytes, i18n.language)}
                  </span>
                </div>
                <div className="file-metadata">
                  <span>
                    {formatName(file.format)}
                    {file.formatEvidence !== "unknown" && (
                      <> · {t(`identification.${file.formatEvidence}`)}</>
                    )}
                  </span>
                  <span>
                    {file.platform
                      ? `${platformLabels[file.platform]} · ${t(`identification.${file.platformEvidence}`)}`
                      : t("library.platformUnknown")}
                  </span>
                  <span>{exact(file.sizeBytes)}</span>
                </div>
                <code>SHA-256 {file.sha256}</code>
              </li>
            ))}
          </ul>
          {cursor && (
            <button disabled={loading} onClick={() => void load(true)}>
              {t("library.moreFiles")}
            </button>
          )}
        </section>
      )}
    </article>
  );
}
