import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  GuardianSnapshot,
  GuardianNetworkRequest,
  NetworkSnapshot,
} from "@game4/ipc-contracts";
import { guardianNetworkRequest } from "../services/guardian";
import { isTauri } from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
export function GuardianNetworkSettings({
  guardian,
  onChanged,
}: {
  guardian: GuardianSnapshot;
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const [data, setData] = useState<NetworkSnapshot>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<unknown>();
  const [bind, setBind] = useState("127.0.0.1:42430"),
    [remote, setRemote] = useState(""),
    [connections, setConnections] = useState("4"),
    [timeout, setTimeoutValue] = useState("120"),
    [retries, setRetries] = useState("2"),
    [entry, setEntry] = useState("");
  function receive(d: NetworkSnapshot) {
    setData(d);
    setBind(d.config.bind);
    setConnections(String(d.config.maxConnections));
    setTimeoutValue(String(d.config.timeoutSeconds));
    setRetries(String(d.config.retries));
  }
  useEffect(() => {
    let alive = true;
    if (isTauri())
      guardianNetworkRequest({ action: "status" })
        .then((d) => {
          if (alive) receive(d);
        })
        .catch((e) => {
          if (alive) setError(e);
        });
    return () => {
      alive = false;
    };
  }, [guardian.nodeId, guardian.config.active]);
  useEffect(() => {
    if (!data?.running || busy) return;
    let alive = true;
    const timer = window.setInterval(() => {
      guardianNetworkRequest({ action: "status" })
        .then((next) => {
          if (alive)
            setData((previous) => ({
              ...next,
              outcome: previous?.outcome ?? null,
            }));
        })
        .catch((e) => {
          if (alive) setError(e);
        });
    }, 2000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [data?.running, busy]);
  async function run(request: GuardianNetworkRequest) {
    setBusy(true);
    setError(undefined);
    try {
      receive(await guardianNetworkRequest(request));
      if (request.action === "fetch") onChanged();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  const inactive = busy || !data || !isTauri();
  const id = Number(entry);
  const selected = guardian.entries.find((f) => f.id === id);
  return (
    <section
      className="guardian-network"
      aria-labelledby="guardian-network-title"
    >
      <h3 id="guardian-network-title">{t("guardianNetwork.title")}</h3>
      <p>{t("guardianNetwork.description")}</p>
      <p>{t("guardianNetwork.enrollmentHelp")}</p>
      {error !== undefined && (
        <p role="alert" className="error-message">
          {t(getIpcErrorKey(error))}
        </p>
      )}
      <button
        disabled={busy || !isTauri()}
        onClick={() => void run({ action: "status" })}
      >
        {t("guardian.refresh")}
      </button>
      {data && (
        <>
          <p role="status">
            {t(
              data.running
                ? "guardianNetwork.running"
                : "guardianNetwork.stopped",
            )}{" "}
            ·{" "}
            {t("guardianNetwork.connections", {
              count: data.connections,
              max: data.config.maxConnections,
            })}
          </p>
          <details>
            <summary>{t("guardianNetwork.identity")}</summary>
            <p>
              <code>{data.nodeId}</code>
            </p>
            <p>
              SHA-256 <code>{data.certificateSha256}</code>
            </p>
          </details>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run({
                action: "configure",
                bind,
                max_connections: Number(connections),
                timeout_seconds: Number(timeout),
                retries: Number(retries),
              });
            }}
          >
            <div className="guardian-controls">
              <label>
                {t("guardianNetwork.bind")}
                <input
                  value={bind}
                  onChange={(e) => setBind(e.target.value)}
                  maxLength={100}
                  required
                />
              </label>
              <label>
                {t("guardianNetwork.maxConnections")}
                <input
                  type="number"
                  min="1"
                  max="16"
                  value={connections}
                  onChange={(e) => setConnections(e.target.value)}
                  required
                />
              </label>
              <label>
                {t("guardianNetwork.timeout")}
                <input
                  type="number"
                  min="2"
                  max="600"
                  value={timeout}
                  onChange={(e) => setTimeoutValue(e.target.value)}
                  required
                />
              </label>
              <label>
                {t("guardianNetwork.retries")}
                <input
                  type="number"
                  min="0"
                  max="3"
                  value={retries}
                  onChange={(e) => setRetries(e.target.value)}
                  required
                />
              </label>
            </div>
            <button type="submit" disabled={inactive || data.running}>
              {t("guardianNetwork.save")}
            </button>
          </form>
          <p>{t("guardianNetwork.bindHelp")}</p>
          <button
            disabled={inactive || data.running}
            onClick={() =>
              void run({ action: "export_profile", address: bind })
            }
          >
            {t("guardianNetwork.exportProfile")}
          </button>
          <button
            disabled={
              inactive ||
              data.running ||
              !guardian.config.active ||
              data.config.peers.length === 0
            }
            onClick={() => void run({ action: "start" })}
          >
            {t("guardianNetwork.start")}
          </button>
          <button
            disabled={!data.running}
            onClick={() => void run({ action: "stop" })}
          >
            {t("guardianNetwork.stop")}
          </button>
          <div className="guardian-controls">
            <label>
              {t("guardianNetwork.remoteAddress")}
              <input
                value={remote}
                onChange={(e) => setRemote(e.target.value)}
                maxLength={100}
              />
            </label>
          </div>
          <button
            disabled={inactive || data.running}
            onClick={() => void run({ action: "add_peer", address: remote })}
          >
            {t("guardianNetwork.importPeer")}
          </button>
          <h4>{t("guardianNetwork.peers")}</h4>
          {data.config.peers.length === 0 && (
            <p>{t("guardianNetwork.noPeers")}</p>
          )}
          <label>
            {t("guardianNetwork.selectedFragment")}
            <select value={entry} onChange={(e) => setEntry(e.target.value)}>
              <option value="">{t("guardianNetwork.chooseFragment")}</option>
              {guardian.entries.map((f) => (
                <option value={f.id} key={f.id}>
                  {f.name} · {f.group}/{f.index}
                </option>
              ))}
            </select>
          </label>
          <p>{t("guardianNetwork.fragmentHelp")}</p>
          <ul className="guardian-fragments">
            {data.config.peers.map((peer) => (
              <li key={peer.nodeId}>
                <strong>{peer.address}</strong>
                <details>
                  <summary>{t("guardianNetwork.peerIdentity")}</summary>
                  <p>
                    <code>{peer.nodeId}</code>
                  </p>
                  <p>
                    SHA-256 <code>{peer.certificateSha256}</code>
                  </p>
                </details>
                <button
                  disabled={inactive || !data.running}
                  onClick={() =>
                    void run({ action: "probe", node_id: peer.nodeId })
                  }
                >
                  {t("guardianNetwork.probe")}
                </button>
                <button
                  disabled={inactive || !data.running || !selected}
                  onClick={() =>
                    void run({
                      action: "availability",
                      node_id: peer.nodeId,
                      id,
                    })
                  }
                >
                  {t("guardianNetwork.availability")}
                </button>
                <button
                  disabled={inactive || !data.running || !selected}
                  onClick={() =>
                    void run({ action: "fetch", node_id: peer.nodeId, id })
                  }
                >
                  {t("guardianNetwork.fetch")}
                </button>
                <button
                  disabled={
                    inactive ||
                    !data.running ||
                    !selected?.sharing ||
                    selected.health === "missing" ||
                    selected.health === "corrupt"
                  }
                  onClick={() =>
                    void run({ action: "push", node_id: peer.nodeId, id })
                  }
                >
                  {t("guardianNetwork.push")}
                </button>
                <button
                  disabled={inactive || data.running}
                  onClick={() =>
                    void run({ action: "remove_peer", node_id: peer.nodeId })
                  }
                >
                  {t("guardianNetwork.removePeer")}
                </button>
              </li>
            ))}
          </ul>
          {data.outcome && (
            <p role="status">
              {t(
                data.outcome.operation === "availability"
                  ? data.outcome.available
                    ? "guardianNetwork.available"
                    : "guardianNetwork.unavailable"
                  : `guardianNetwork.result.${data.outcome.operation}`,
                { bytes: data.outcome.bytes, attempts: data.outcome.attempts },
              )}
            </p>
          )}
        </>
      )}
    </section>
  );
}
