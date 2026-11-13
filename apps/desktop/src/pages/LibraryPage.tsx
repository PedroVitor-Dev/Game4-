import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  LibraryItemSummary,
  SourceKind,
  SourceSelection,
  StartImportRequest,
  StartVerifyRequest,
  TaskSnapshot,
  EverExportResult,
} from "@game4/ipc-contracts";
import {
  selectSources,
  importSources,
  listLibrary,
  listTasks,
  isTauri,
  exportManifest,
  verifyItem,
  exportPackage,
  exportEver,
} from "../services/desktop";
import { EverReader } from "../components/EverReader";
import { PackageReader } from "../components/PackageReader";
import { LibraryItemCard } from "../components/LibraryItemCard";
import { getIpcErrorKey } from "../services/errors";

export function LibraryPage({ tasksOnly = false }: { tasksOnly?: boolean }) {
  const { t, i18n } = useTranslation();
  const [items, setItems] = useState<LibraryItemSummary[]>([]);
  const [tasks, setTasks] = useState<TaskSnapshot[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [everWorking, setEverWorking] = useState(false);
  const [everSaved, setEverSaved] = useState<EverExportResult>();
  const [packaging, setPackaging] = useState(false);
  const [packageSaved, setPackageSaved] = useState<string>();
  const [exported, setExported] = useState<string>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<unknown>();
  const [selection, setSelection] = useState<SourceSelection>();
  const libraryLoaded = useRef(false);
  const completed = useRef("");
  const pending = useRef<StartImportRequest | null>(null);
  const pendingVerify = useRef<StartVerifyRequest | null>(null);
  const alive = useRef(true);
  const mounted = useRef(false);
  const refresh = useCallback(async () => {
    if (!isTauri()) {
      setLoading(false);
      return;
    }
    try {
      const jobs = await listTasks({ cursor: null, limit: 50 });
      const signature = jobs.items
        .filter((j) => j.status === "succeeded")
        .map((j) => j.id + ":" + j.revision)
        .join(",");
      const library =
        !tasksOnly &&
        (!libraryLoaded.current || completed.current !== signature)
          ? await listLibrary({
              page: { cursor: null, limit: 50 },
              query: null,
            })
          : null;
      if (!alive.current) return;
      if (library) {
        setItems(library.items);
        setCursor(library.nextCursor);
        libraryLoaded.current = true;
        completed.current = signature;
      }
      setTasks(jobs.items);
    } catch (e) {
      if (alive.current) setError(e);
    } finally {
      if (alive.current) setLoading(false);
    }
  }, [tasksOnly]);
  useEffect(() => {
    alive.current = true;
    mounted.current = true;
    let timer: ReturnType<typeof setTimeout>;
    let stopped = false;
    const poll = async () => {
      await refresh();
      if (!stopped) timer = setTimeout(poll, 1500);
    };
    void poll();
    return () => {
      stopped = true;
      alive.current = false;
      mounted.current = false;
      clearTimeout(timer);
    };
  }, [refresh]);
  async function choose(kind: SourceKind) {
    setBusy(true);
    setError(undefined);
    try {
      const selected = await selectSources({ kind });
      if (!mounted.current) return;
      pending.current = null;
      setSelection(selected.cancelled ? undefined : selected);
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  async function start() {
    if (!selection?.selectionToken) return;
    setBusy(true);
    setError(undefined);
    pending.current ??= {
      requestId: crypto.randomUUID(),
      selectionToken: selection.selectionToken,
    };
    try {
      await importSources(pending.current);
      if (!mounted.current) return;
      pending.current = null;
      setSelection(undefined);
      await refresh();
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  async function more() {
    if (!cursor) return;
    setBusy(true);
    try {
      const page = await listLibrary({
        page: { cursor, limit: 50 },
        query: null,
      });
      if (!mounted.current) return;
      setItems((v) => [...v, ...page.items]);
      setCursor(page.nextCursor);
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  async function verify(itemId: string) {
    setBusy(true);
    setError(undefined);
    if (pendingVerify.current?.itemId !== itemId)
      pendingVerify.current = { requestId: crypto.randomUUID(), itemId };
    try {
      await verifyItem(pendingVerify.current);
      pendingVerify.current = null;
      if (mounted.current) await refresh();
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  async function saveEver(itemId: string) {
    setBusy(true);
    setEverWorking(true);
    setEverSaved(undefined);
    setError(undefined);
    try {
      const result = await exportEver({ itemId });
      if (mounted.current && !result.cancelled) setEverSaved(result);
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) {
        setBusy(false);
        setEverWorking(false);
      }
    }
  }
  async function savePackage(itemId: string) {
    setBusy(true);
    setPackaging(true);
    setError(undefined);
    setPackageSaved(undefined);
    try {
      const result = await exportPackage({ itemId });
      if (mounted.current && !result.cancelled && result.fileName)
        setPackageSaved(result.fileName);
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) {
        setBusy(false);
        setPackaging(false);
      }
    }
  }
  async function saveManifest(itemId: string) {
    setBusy(true);
    setError(undefined);
    setExported(undefined);
    try {
      const result = await exportManifest({ itemId });
      if (mounted.current && !result.cancelled && result.fileName)
        setExported(result.fileName);
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  const active = tasks.some(
    (j) => j.status === "queued" || j.status === "running",
  );
  const number = (v: string | number) =>
    BigInt(v).toLocaleString(i18n.language);
  return (
    <div className="library-workspace">
      {!tasksOnly && (
        <section className="content-card import-panel">
          <div>
            <h2>{t("library.importTitle")}</h2>
            <p>{t("library.importDescription")}</p>
            <p>{t("library.manifestHint")}</p>
          </div>
          <div className="actions">
            <button
              disabled={busy || active || !isTauri()}
              onClick={() => void choose("files")}
            >
              {t("library.addFiles")}
            </button>
            <button
              disabled={busy || active || !isTauri()}
              onClick={() => void choose("directories")}
            >
              {t("library.addFolders")}
            </button>
          </div>
          {!isTauri() && <p>{t("errors.ipcUnavailable")}</p>}
          {selection && (
            <div className="selection">
              <h3>{t("library.selection")}</h3>
              <ul>
                {selection.displayNames.map((name, i) => (
                  <li key={i}>{name}</li>
                ))}
              </ul>
              <div className="actions">
                <button
                  className="primary"
                  disabled={busy || active}
                  onClick={() => void start()}
                >
                  {t("library.startImport")}
                </button>
                <button
                  disabled={busy}
                  onClick={() => {
                    setSelection(undefined);
                    pending.current = null;
                  }}
                >
                  {t("library.dismiss")}
                </button>
              </div>
            </div>
          )}
        </section>
      )}
      {!tasksOnly && <PackageReader disabled={busy || active} />}
      {!tasksOnly && <EverReader disabled={busy || active} />}
      {everWorking && <p role="status">{t("ever.creating")}</p>}
      {everSaved && (
        <section className="content-card ever-export-result" role="status">
          <p>
            {t("ever.saved", {
              fileName: everSaved.fileName,
              torrentFileName: everSaved.torrentFileName,
            })}
          </p>
          <p>{t("ever.shareHint")}</p>
          <textarea
            readOnly
            aria-label={t("ever.magnet")}
            value={everSaved.magnet ?? ""}
          />
        </section>
      )}
      {packaging && <p role="status">{t("packages.creating")}</p>}
      {packageSaved && (
        <p role="status">{t("packages.saved", { fileName: packageSaved })}</p>
      )}
      {error !== undefined && (
        <div className="error-message" role="alert">
          {t(getIpcErrorKey(error))}{" "}
          <button
            onClick={() => {
              setError(undefined);
              libraryLoaded.current = false;
              void refresh();
            }}
          >
            {t("library.refresh")}
          </button>
        </div>
      )}
      {exported && (
        <p role="status">
          {t("library.manifestSaved", { fileName: exported })}
        </p>
      )}
      {loading && <p role="status">{t("library.loading")}</p>}
      {!tasksOnly &&
        !loading &&
        (items.length === 0 ? (
          <section className="content-card empty-state">
            <div className="empty-icon" aria-hidden="true">
              ▤
            </div>
            <h2>{t("library.emptyTitle")}</h2>
            <p>{t("library.emptyDescription")}</p>
          </section>
        ) : (
          <section aria-label={t("nav.library")} className="library-grid">
            {items.map((item) => (
              <LibraryItemCard
                key={item.id}
                item={item}
                busy={busy || active}
                onVerify={(id) => void verify(id)}
                onPackage={(id) => void savePackage(id)}
                onEver={(id) => void saveEver(id)}
                onExport={(id) => void saveManifest(id)}
              />
            ))}
          </section>
        ))}
      {!tasksOnly && cursor && (
        <button disabled={busy} onClick={() => void more()}>
          {t("library.loadMore")}
        </button>
      )}
      {(tasksOnly || tasks.length > 0) && (
        <section className="content-card task-panel">
          <h2>{t("tasks.recent")}</h2>
          {tasks.length === 0 && !loading && (
            <p>{t("tasks.emptyDescription")}</p>
          )}
          {tasks.map((job) => (
            <article key={job.id} className="task-row">
              <div>
                <strong>
                  {t(`tasks.kind.${job.kind}`)} #{job.id}
                </strong>
                <span className={`task-status ${job.status}`}>
                  {t(`tasks.status.${job.status}`)}
                </span>
              </div>
              <p>
                {t(
                  job.kind === "verify"
                    ? "integrity.progress"
                    : "tasks.progress",
                  {
                    processed: number(job.processedCount),
                    total:
                      job.totalCount === null ? "…" : number(job.totalCount),
                  },
                )}
              </p>
              {job.kind === "verify" && job.status === "succeeded" && (
                <p>{t("integrity.completed")}</p>
              )}
              {job.error && <p role="alert">{t(getIpcErrorKey(job.error))}</p>}
              {job.status === "running" && (
                <progress
                  aria-label={t(`tasks.kind.${job.kind}`)}
                  {...(job.totalCount !== null && BigInt(job.totalCount) > 0n
                    ? {
                        max: 100,
                        value: Number(
                          (BigInt(job.processedCount) * 100n) /
                            BigInt(job.totalCount),
                        ),
                      }
                    : {})}
                />
              )}
            </article>
          ))}
        </section>
      )}
    </div>
  );
}
