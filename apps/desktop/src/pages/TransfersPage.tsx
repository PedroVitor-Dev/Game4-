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
  listLibrary,
  listTransfers,
  createTorrent,
  importTorrent,
  seedTorrent,
  downloadTorrent,
  stopTransfer,
} from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
import { formatBytes } from "../services/file-identification";
export function TransfersPage() {
  const { t, i18n } = useTranslation();
  const [items, setItems] = useState<LibraryItemSummary[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [itemId, setItemId] = useState("");
  const [port, setPort] = useState("0");
  const [peer, setPeer] = useState("");
  const [preview, setPreview] = useState<TorrentPreview>();
  const [allFiles, setAllFiles] = useState(false);
  const [transfers, setTransfers] = useState<TransferSnapshot[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>();
  const [saved, setSaved] = useState<{ name: string; magnet: string }>();
  const alive = useRef(true);
  const seedPending = useRef<StartSeedRequest | null>(null);
  const downloadPending = useRef<StartDownloadRequest | null>(null);
  async function refresh() {
    if (!isTauri()) return;
    try {
      const result = await listTransfers();
      if (alive.current) setTransfers(result.items);
    } catch (e) {
      if (alive.current) setError(e);
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
  function seed() {
    const listenPort = Number(port);
    if (!Number.isInteger(listenPort) || listenPort < 0 || listenPort > 65535)
      return;
    void act(async () => {
      if (
        seedPending.current?.itemId !== itemId ||
        seedPending.current.listenPort !== listenPort
      )
        seedPending.current = {
          requestId: crypto.randomUUID(),
          itemId,
          listenPort,
        };
      await seedTorrent(seedPending.current);
      seedPending.current = null;
    });
  }
  function download() {
    if (!preview) return;
    void act(async () => {
      if (
        downloadPending.current?.token !== preview.token ||
        downloadPending.current.peer !== peer.trim()
      )
        downloadPending.current = {
          requestId: crypto.randomUUID(),
          token: preview.token,
          peer: peer.trim(),
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
      <section className="content-card transfer-panel">
        <h2>{t("transfers.connectionTitle")}</h2>
        <p>{t("transfers.connectionDescription")}</p>
        <p>{t("transfers.exposure")}</p>
        {!isTauri() && <p>{t("errors.ipcUnavailable")}</p>}
      </section>
      {error !== undefined && (
        <p role="alert" className="error-message">
          {t(getIpcErrorKey(error))}
        </p>
      )}
      {busy && <p role="status">{t("transfers.preparing")}</p>}
      <section className="content-card transfer-panel">
        <h2>{t("transfers.seedTitle")}</h2>
        <p>{t("transfers.sourceHint")}</p>
        <label>
          {t("transfers.source")}
          <select
            value={itemId}
            disabled={busy}
            onChange={(e) => setItemId(e.target.value)}
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
                const result = await createTorrent({ itemId });
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
            disabled={busy || !itemId || !isTauri()}
            onClick={seed}
          >
            {t("transfers.startSeed")}
          </button>
        </div>
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
      <section className="content-card transfer-panel">
        <h2>{t("transfers.downloadTitle")}</h2>
        <p>{t("transfers.importHint")}</p>
        <button
          disabled={busy || !isTauri()}
          onClick={() =>
            void act(async () => {
              const result = await importTorrent();
              if (alive.current && result.preview) {
                setPreview(result.preview);
                setAllFiles(false);
                downloadPending.current = null;
              }
            })
          }
        >
          {t("transfers.import")}
        </button>
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
            <label>
              {t("transfers.peer")}
              <input
                value={peer}
                onChange={(e) => setPeer(e.target.value)}
                placeholder="100.x.x.x:42420"
                disabled={busy}
              />
            </label>
            <p>{t("transfers.outputHint")}</p>
            <button
              className="primary"
              disabled={busy || !peer.trim()}
              onClick={download}
            >
              {t("transfers.startDownload")}
            </button>
          </div>
        )}
      </section>
      <section className="content-card transfer-panel">
        <h2>{t("transfers.sessions")}</h2>
        {!transfers.length && <p>{t("transfers.empty")}</p>}
        {transfers.map((job) => (
          <article className="transfer-row" key={job.id}>
            <h3>{job.name}</h3>
            <p>
              {t(`transfers.mode.${job.mode}`)} ·{" "}
              {t(`transfers.status.${job.status}`)}
            </p>
            <p>
              {t("transfers.progress", {
                verified: bytes(job.verifiedBytes),
                total: bytes(job.totalBytes),
                uploaded: bytes(job.uploadedBytes),
              })}
            </p>
            <progress
              max="100"
              value={Number(
                (BigInt(job.verifiedBytes) * 100n) / BigInt(job.totalBytes),
              )}
              aria-label={t("transfers.pieces")}
            />
            {job.listenPort !== null && (
              <p>{t("transfers.listening", { port: job.listenPort })}</p>
            )}
            {job.outputName && (
              <p className="transfer-output">
                {t("transfers.output", { name: job.outputName })}
              </p>
            )}
            {job.error && <p role="alert">{t(getIpcErrorKey(job.error))}</p>}
            {job.status === "active" && (
              <button
                disabled={busy}
                onClick={() =>
                  void act(async () => {
                    await stopTransfer({ transferId: job.id });
                  })
                }
              >
                {t("transfers.stop")}
              </button>
            )}
          </article>
        ))}
      </section>
    </div>
  );
}
