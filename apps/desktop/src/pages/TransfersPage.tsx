import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  LibraryItemSummary,
  TorrentPreview,
  TransferSnapshot,
  StartSeedRequest,
  StartDownloadRequest,
} from "@game4/ipc-contracts";
import {
  isTauri,
  generateMagnet,
  resolveMagnet,
  cancelMagnet,
  listLibrary,
  listTransfers,
  createTorrent,
  importTorrent,
  seedTorrent,
  downloadTorrent,
  stopTransfer,
  pauseTransfer,
  resumeTransfer,
  forgetTransfer,
} from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
import { formatBytes } from "../services/file-identification";
import { TransferMetrics } from "../components/TransferMetrics";
import {
  BandwidthInputs,
  TransferLimitsControl,
} from "../components/BandwidthInputs";
import { parseBandwidthInput } from "../services/transfer-metrics";
export function TransfersPage() {
  const { t, i18n } = useTranslation();
  const [downloadLimit, setDownloadLimit] = useState("0");
  const [uploadLimit, setUploadLimit] = useState("0");
  const downloadBps = parseBandwidthInput(downloadLimit);
  const uploadBps = parseBandwidthInput(uploadLimit);
  const validLimits = downloadBps !== null && uploadBps !== null;
  const limits = { downloadBps: downloadBps ?? 0, uploadBps: uploadBps ?? 0 };
  const [sessionFilter, setSessionFilter] = useState("all");
  const [composer, setComposer] = useState<"seed" | "download">("download");
  const [fresh, setFresh] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [items, setItems] = useState<LibraryItemSummary[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [itemId, setItemId] = useState("");
  const [port, setPort] = useState("0");
  const [peer, setPeer] = useState("");
  const [dht, setDht] = useState(false);
  const [seedAfterDownload, setSeedAfterDownload] = useState(false);
  const [useTrackers, setUseTrackers] = useState(false);
  const [trackerText, setTrackerText] = useState("");
  const trackers = useTrackers
    ? trackerText
        .split(/\r?\n/)
        .map((v) => v.trim())
        .filter(Boolean)
    : [];
  const discovery = dht || trackers.length > 0;
  const [magnet, setMagnet] = useState("");
  const [generatedMagnet, setGeneratedMagnet] = useState("");
  const [resolving, setResolving] = useState<string | null>(null);
  const resolvingRef = useRef<string | null>(null);
  const [preview, setPreview] = useState<TorrentPreview>();
  const [allFiles, setAllFiles] = useState(false);
  const [transfers, setTransfers] = useState<TransferSnapshot[]>([]);
  const [busy, setBusy] = useState(false);
  const [resuming, setResuming] = useState<Set<string>>(new Set());
  const [stopping, setStopping] = useState<Set<string>>(new Set());
  const [error, setError] = useState<unknown>();
  const [saved, setSaved] = useState<{ name: string; magnet: string }>();
  const alive = useRef(true);
  const seedPending = useRef<StartSeedRequest | null>(null);
  const downloadPending = useRef<StartDownloadRequest | null>(null);
  async function refresh() {
    if (!isTauri()) return;
    try {
      const result = await listTransfers();
      if (alive.current) {
        setTransfers(result.items);
        setFresh(true);
        setLoaded(true);
      }
    } catch (e) {
      if (alive.current) {
        setError(e);
        setFresh(false);
      }
    }
  }
  useEffect(() => {
    alive.current = true;
    let timer: ReturnType<typeof setTimeout>;
    let ended = false;
    const poll = async () => {
      await refresh();
      if (!ended) timer = setTimeout(poll, 1000);
    };
    void poll();
    if (isTauri())
      void listLibrary({
        page: { cursor: null, limit: 100 },
        query: null,
      }).then(
        (p) => {
          if (alive.current) {
            setItems(p.items);
            setCursor(p.nextCursor);
            setItemId(p.items[0]?.id ?? "");
          }
        },
        (e) => {
          if (alive.current) setError(e);
        },
      );
    return () => {
      alive.current = false;
      ended = true;
      if (resolvingRef.current)
        void cancelMagnet({ requestId: resolvingRef.current }).catch(() => {});
      clearTimeout(timer);
    };
  }, []);
  async function act(operation: () => Promise<void>) {
    setBusy(true);
    setError(undefined);
    try {
      await operation();
    } catch (e) {
      if (alive.current) setError(e);
    } finally {
      if (alive.current) {
        setBusy(false);
        await refresh();
      }
    }
  }
  async function stop(
    id: string,
    action: "stop" | "pause" | "forget" = "stop",
  ) {
    setStopping((v) => new Set(v).add(id));
    try {
      if (action === "pause") await pauseTransfer({ transferId: id });
      else if (action === "forget") await forgetTransfer({ transferId: id });
      else await stopTransfer({ transferId: id });
    } catch (e) {
      if (alive.current) setError(e);
    } finally {
      if (alive.current) {
        setStopping((v) => {
          const next = new Set(v);
          next.delete(id);
          return next;
        });
        await refresh();
      }
    }
  }
  async function resume(id: string) {
    setResuming((v) => new Set(v).add(id));
    try {
      await resumeTransfer({ transferId: id });
    } catch (e) {
      if (alive.current) setError(e);
    } finally {
      if (alive.current) {
        setResuming((v) => {
          const next = new Set(v);
          next.delete(id);
          return next;
        });
        await refresh();
      }
    }
  }
  function seed() {
    if (!validLimits) return;
    const listenPort = Number(port);
    if (!Number.isInteger(listenPort) || listenPort < 0 || listenPort > 65535)
      return;
    void act(async () => {
      if (
        seedPending.current?.itemId !== itemId ||
        seedPending.current.listenPort !== listenPort ||
        seedPending.current.dht !== dht ||
        JSON.stringify(seedPending.current.limits) !== JSON.stringify(limits) ||
        JSON.stringify(seedPending.current.trackers) !==
          JSON.stringify(trackers)
      )
        seedPending.current = {
          requestId: crypto.randomUUID(),
          itemId,
          limits,
          listenPort,
          dht,
          trackers,
        };
      await seedTorrent(seedPending.current);
      seedPending.current = null;
    });
  }
  function download() {
    if (!preview || !validLimits) return;
    void act(async () => {
      if (
        downloadPending.current?.token !== preview.token ||
        downloadPending.current.peer !== peer.trim() ||
        downloadPending.current.dht !== dht ||
        downloadPending.current.seedAfterDownload !== seedAfterDownload ||
        JSON.stringify(downloadPending.current.limits) !==
          JSON.stringify(limits) ||
        JSON.stringify(downloadPending.current.trackers) !==
          JSON.stringify(trackers)
      )
        downloadPending.current = {
          requestId: crypto.randomUUID(),
          token: preview.token,
          limits,
          seedAfterDownload,
          peer: peer.trim(),
          dht,
          trackers,
        };
      const result = await downloadTorrent(downloadPending.current);
      if (!result.cancelled) {
        downloadPending.current = null;
        if (alive.current) setPreview(undefined);
      }
    });
  }
  const bytes = (value: string) => formatBytes(value, i18n.language);
  return (
    <div className="library-workspace transfers-workspace">
      <section className="workspace-banner transfer-banner">
        <div>
          <p className="eyebrow">BITTORRENT / P2P</p>
          <h2>{t("transfers.centerTitle")}</h2>
          <p>{t("transfers.persistenceHint")}</p>
        </div>
        <div className="banner-symbol" aria-hidden="true">
          ⇄
        </div>
      </section>
      <div className="workspace-metrics transfer-metrics">
        {["active", "paused", "completed"].map((status) => (
          <article key={status}>
            <span>{t(`transfers.filter.${status}`)}</span>
            <strong>
              {loaded
                ? transfers
                    .filter((job) => job.status === status)
                    .length.toLocaleString(i18n.language)
                : "—"}
            </strong>
          </article>
        ))}
      </div>
      {error !== undefined && (
        <p role="alert" className="error-message">
          {t(getIpcErrorKey(error))}
        </p>
      )}
      {busy && (
        <p role="status">
          {t(resolving ? "transfers.resolving" : "transfers.preparing")}
        </p>
      )}
      <section className="content-card transfer-panel saved-sessions">
        <div className="session-heading">
          <h2>{t("transfers.sessions")}</h2>
          <label>
            <span className="sr-only">{t("transfers.filterLabel")}</span>
            <select
              value={sessionFilter}
              onChange={(e) => setSessionFilter(e.target.value)}
            >
              {["all", "active", "paused", "completed", "failed"].map(
                (status) => (
                  <option key={status} value={status}>
                    {t(`transfers.filter.${status}`)}
                  </option>
                ),
              )}
            </select>
          </label>
        </div>
        {!isTauri() ? (
          <p role="status">{t("errors.ipcUnavailable")}</p>
        ) : !loaded ? (
          <p role="status">{t("transfers.loading")}</p>
        ) : !transfers.length ? (
          <div className="session-empty">
            <span aria-hidden="true">⇄</span>
            <h3>{t("transfers.emptyTitle")}</h3>
            <p>{t("transfers.empty")}</p>
          </div>
        ) : null}
        {loaded &&
          transfers.length > 0 &&
          !transfers.some(
            (job) => sessionFilter === "all" || job.status === sessionFilter,
          ) && <p>{t("transfers.noMatches")}</p>}
        {transfers
          .filter(
            (job) => sessionFilter === "all" || job.status === sessionFilter,
          )
          .map((job) => (
            <article className="transfer-row" key={job.id}>
              <div className="session-heading">
                <h3>{job.name}</h3>
                <span className={`session-status status-${job.status}`}>
                  {t(`transfers.status.${job.status}`)}
                </span>
              </div>
              <p>
                {t(
                  job.isSeeding
                    ? "transfers.seedingNow"
                    : `transfers.mode.${job.mode}`,
                )}{" "}
                · {t(`transfers.status.${job.status}`)}
              </p>
              <details className="session-details">
                <summary>{t("transfers.sessionDetails")}</summary>
                <p>{t(job.dht ? "transfers.dhtOn" : "transfers.dhtOff")}</p>
                <p>
                  {t("transfers.trackerCount", { count: job.trackerCount })}
                </p>
                <label>
                  {t("ever.magnet")}
                  <textarea
                    readOnly
                    value={`magnet:?xt=urn:btih:${job.infoHash}`}
                  />
                </label>
              </details>
              <TransferMetrics transfer={job} fresh={fresh} />
              <TransferLimitsControl transfer={job} onSaved={refresh} />
              {job.listenPort !== null && (
                <p>{t("transfers.listening", { port: job.listenPort })}</p>
              )}
              {job.outputName && (
                <p className="transfer-output">
                  {t("transfers.output", { name: job.outputName })}
                </p>
              )}
              {job.error && <p role="alert">{t(getIpcErrorKey(job.error))}</p>}
              <div className="actions">
                {["paused", "stopped", "failed"].includes(job.status) && (
                  <button
                    disabled={resuming.has(job.id) || stopping.has(job.id)}
                    onClick={() => void resume(job.id)}
                  >
                    {t("transfers.resume")}
                  </button>
                )}
                {job.status === "active" && (
                  <button
                    disabled={stopping.has(job.id)}
                    onClick={() => void stop(job.id, "pause")}
                  >
                    {t("transfers.pause")}
                  </button>
                )}
                {["paused", "stopped", "failed", "completed"].includes(
                  job.status,
                ) && (
                  <button
                    disabled={resuming.has(job.id) || stopping.has(job.id)}
                    onClick={() => void stop(job.id, "forget")}
                  >
                    {t("transfers.forget")}
                  </button>
                )}
                {["active", "resuming"].includes(job.status) && (
                  <button
                    disabled={stopping.has(job.id)}
                    onClick={() => void stop(job.id)}
                  >
                    {t("transfers.stop")}
                  </button>
                )}
              </div>
            </article>
          ))}
      </section>
      <section className="content-card transfer-panel new-bandwidth">
        <h2>{t("transfers.nextLimits")}</h2>
        <BandwidthInputs
          download={downloadLimit}
          upload={uploadLimit}
          onDownload={setDownloadLimit}
          onUpload={setUploadLimit}
          disabled={busy}
        />
      </section>
      <div
        className="composer-switch"
        role="group"
        aria-label={t("transfers.newTransfer")}
      >
        <span>{t("transfers.newTransfer")}</span>
        <button
          disabled={busy}
          aria-pressed={composer === "download"}
          onClick={() => setComposer("download")}
        >
          {t("transfers.downloadTab")}
        </button>
        <button
          disabled={busy}
          aria-pressed={composer === "seed"}
          onClick={() => setComposer("seed")}
        >
          {t("transfers.seedTab")}
        </button>
      </div>
      <section
        hidden={composer !== "seed"}
        className="content-card transfer-panel transfer-composer"
      >
        <h2>{t("transfers.seedTitle")}</h2>
        <p>{t("transfers.sourceHint")}</p>
        <label>
          {t("transfers.source")}
          <select
            value={itemId}
            disabled={busy}
            onChange={(e) => {
              setItemId(e.target.value);
              setGeneratedMagnet("");
              setSaved(undefined);
            }}
          >
            {items.map((item) => (
              <option key={item.id} value={item.id}>
                {item.title}
              </option>
            ))}
          </select>
        </label>
        {!items.length && <p>{t("transfers.noSource")}</p>}
        {cursor && (
          <button
            disabled={busy}
            onClick={() =>
              void act(async () => {
                const p = await listLibrary({
                  page: { cursor, limit: 100 },
                  query: null,
                });
                if (alive.current) {
                  setItems((v) => [...v, ...p.items]);
                  setCursor(p.nextCursor);
                }
              })
            }
          >
            {t("library.loadMore")}
          </button>
        )}
        <label>
          {t("transfers.port")}
          <input
            type="number"
            min="0"
            max="65535"
            value={port}
            disabled={busy}
            onChange={(e) => setPort(e.target.value)}
          />
        </label>
        <div className="actions">
          <button
            disabled={busy || !itemId || !isTauri()}
            onClick={() =>
              void act(async () => {
                const result = await generateMagnet({ itemId, trackers });
                if (alive.current) setGeneratedMagnet(result.magnet);
              })
            }
          >
            {t("transfers.generateMagnet")}
          </button>
          <button
            disabled={busy || !itemId || !isTauri()}
            onClick={() =>
              void act(async () => {
                const result = await createTorrent({ itemId, trackers });
                if (
                  alive.current &&
                  !result.cancelled &&
                  result.fileName &&
                  result.magnet
                )
                  setSaved({ name: result.fileName, magnet: result.magnet });
              })
            }
          >
            {t("transfers.create")}
          </button>
          <button
            className="primary"
            disabled={busy || !itemId || !isTauri() || !validLimits}
            onClick={seed}
          >
            {t("transfers.startSeed")}
          </button>
        </div>
        {generatedMagnet && (
          <label>
            {t("ever.magnet")}
            <textarea readOnly value={generatedMagnet} />
          </label>
        )}
        {saved && (
          <div role="status">
            <p>{t("transfers.created", { fileName: saved.name })}</p>
            <textarea
              readOnly
              aria-label={t("ever.magnet")}
              value={saved.magnet}
            />
          </div>
        )}
      </section>
      <section
        hidden={composer !== "download"}
        className="content-card transfer-panel transfer-composer"
      >
        <h2>{t("transfers.downloadTitle")}</h2>
        <p>{t("transfers.importHint")}</p>
        <button
          disabled={busy || !isTauri()}
          onClick={() =>
            void act(async () => {
              const result = await importTorrent();
              if (alive.current && result.preview) {
                setPreview(result.preview);
                setTrackerText(result.preview.trackers.join("\n"));
                setUseTrackers(false);
                setAllFiles(false);
                downloadPending.current = null;
              }
            })
          }
        >
          {t("transfers.import")}
        </button>
        <label>
          {t("ever.magnet")}
          <textarea
            value={magnet}
            maxLength={16384}
            disabled={busy}
            onChange={(e) => {
              setMagnet(e.target.value);
              try {
                const tr = new URL(e.target.value.trim()).searchParams.getAll(
                  "tr",
                );
                if (tr.length) {
                  setTrackerText(tr.join("\n"));
                  setUseTrackers(false);
                }
              } catch {
                /* Native parser validates on lookup. */
              }
            }}
            placeholder="magnet:?xt=urn:btih:…"
          />
        </label>
        <label>
          {t("transfers.peer")}
          <input
            value={peer}
            onChange={(e) => setPeer(e.target.value)}
            placeholder="100.x.x.x:42420"
            disabled={busy}
          />
        </label>
        <p>{t("transfers.magnetHint")}</p>
        <button
          disabled={
            busy || !isTauri() || !magnet.trim() || (!discovery && !peer.trim())
          }
          onClick={() =>
            void act(async () => {
              const requestId = crypto.randomUUID();
              resolvingRef.current = requestId;
              setResolving(requestId);
              setPreview(undefined);
              try {
                const result = await resolveMagnet({
                  requestId,
                  magnet: magnet.trim(),
                  peer: peer.trim(),
                  dht,
                  trackers,
                });
                if (alive.current && result.preview) {
                  setPreview(result.preview);
                  setAllFiles(false);
                  downloadPending.current = null;
                }
              } finally {
                resolvingRef.current = null;
                if (alive.current) setResolving(null);
              }
            })
          }
        >
          {t("transfers.resolveMagnet")}
        </button>
        {resolving && (
          <button
            onClick={() =>
              void cancelMagnet({ requestId: resolving }).catch((e) =>
                setError(e),
              )
            }
          >
            {t("transfers.cancelResolve")}
          </button>
        )}
        {preview && (
          <div>
            <h3>{preview.name}</h3>
            <p>
              {t("library.fileCount", { count: preview.files.length })} ·{" "}
              {bytes(preview.totalBytes)}
            </p>
            <code>
              {t("transfers.infoHash")}: {preview.infoHash}
            </code>
            <ul>
              {preview.files.slice(0, allFiles ? undefined : 20).map((file) => (
                <li key={file.path}>
                  {file.path} · {bytes(file.sizeBytes)}
                </li>
              ))}
            </ul>
            {preview.files.length > 20 && !allFiles && (
              <button onClick={() => setAllFiles(true)}>
                {t("transfers.showFiles")}
              </button>
            )}
            <p>{t("transfers.outputHint")}</p>
            <label className="dht-choice">
              <input
                type="checkbox"
                checked={seedAfterDownload}
                disabled={busy}
                onChange={(e) => setSeedAfterDownload(e.target.checked)}
              />
              {t("transfers.keepSeeding")}
            </label>
            <p>{t("transfers.keepSeedingHint")}</p>
            <button
              className="primary"
              disabled={busy || !validLimits || (!discovery && !peer.trim())}
              onClick={download}
            >
              {t("transfers.startDownload")}
            </button>
          </div>
        )}
      </section>
      <details className="content-card transfer-panel connection-settings">
        <summary>{t("transfers.connectionTitle")}</summary>
        <p>{t("transfers.connectionDescription")}</p>
        <p>{t("transfers.exposure")}</p>
        <p>{t("transfers.persistenceHint")}</p>
        <label className="dht-choice">
          <input
            type="checkbox"
            checked={dht}
            disabled={busy}
            onChange={(e) => setDht(e.target.checked)}
          />
          {t("transfers.dht")}
        </label>
        <p>{t("transfers.dhtHint")}</p>
        <label className="dht-choice">
          <input
            type="checkbox"
            checked={useTrackers}
            disabled={busy}
            onChange={(e) => setUseTrackers(e.target.checked)}
          />
          {t("transfers.useTrackers")}
        </label>
        <label>
          {t("transfers.trackerList")}
          <textarea
            maxLength={8192}
            value={trackerText}
            disabled={busy}
            onChange={(e) => setTrackerText(e.target.value)}
            placeholder="https://tracker.example/announce"
          />
        </label>
        <p>{t("transfers.trackerHint")}</p>
        {!isTauri() && <p>{t("errors.ipcUnavailable")}</p>}
      </details>
    </div>
  );
}
