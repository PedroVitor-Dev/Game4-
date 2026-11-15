import { GuardianNetworkSettings } from "./GuardianNetworkSettings";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { GuardianSnapshot, GuardianRequest } from "@game4/ipc-contracts";
import { guardianRequest } from "../services/guardian";
import { isTauri } from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";
export function GuardianSettings() {
  const { t } = useTranslation();
  const [data, setData] = useState<GuardianSnapshot>();
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<unknown>(),
    [notice, setNotice] = useState(false);
  const [quota, setQuota] = useState("10240"),
    [up, setUp] = useState("0"),
    [down, setDown] = useState("0");
  const [group, setGroup] = useState("0"),
    [index, setIndex] = useState("0"),
    [sharing, setSharing] = useState(false);
  function receive(d: GuardianSnapshot) {
    setData(d);
    setQuota(String(Number(d.config.quotaBytes) / 1048576));
    setUp(String(d.config.uploadBps / 1024));
    setDown(String(d.config.downloadBps / 1024));
  }
  useEffect(() => {
    let alive = true;
    if (isTauri())
      guardianRequest({ action: "status", offset: 0 })
        .then((d) => {
          if (alive) receive(d);
        })
        .catch((e) => {
          if (alive) setError(e);
        });
    return () => {
      alive = false;
    };
  }, []);
  async function run(request: GuardianRequest) {
    setBusy(true);
    setError(undefined);
    setNotice(false);
    try {
      receive(await guardianRequest(request));
      setNotice(true);
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  const limits = [
    Number(quota) * 1048576,
    Number(up) * 1024,
    Number(down) * 1024,
  ];
  const valid =
    limits.every((n) => Number.isSafeInteger(n) && n >= 0) &&
    limits[0] <= 2 ** 40 &&
    limits[1] <= 1e9 &&
    limits[2] <= 1e9;
  const disabled = busy || !data || !isTauri();
  return (
    <section
      className="content-card storage-settings guardian-settings"
      aria-labelledby="guardian-title"
    >
      <h2 id="guardian-title">Game4∞ Guardian Node</h2>
      <p>{t("guardian.description")}</p>
      <p>{t("guardian.localScope")}</p>
      {!isTauri() && <p>{t("errors.ipcUnavailable")}</p>}
      {error !== undefined && (
        <p role="alert" className="error-message">
          {t(getIpcErrorKey(error))}
        </p>
      )}
      {notice && <p role="status">{t("guardian.updated")}</p>}
      <button
        disabled={busy || !isTauri()}
        onClick={() =>
          void run({ action: "status", offset: data?.offset ?? 0 })
        }
      >
        {t("guardian.refresh")}
      </button>
      {data && (
        <>
          <p>
            <strong>{t("guardian.directory")}</strong>{" "}
            <span className="guardian-path">{data.directory}</span>
          </p>
          <button
            disabled={disabled || data.config.active}
            onClick={() => void run({ action: "choose_directory" })}
          >
            {t("guardian.chooseDirectory")}
          </button>
          <p>{t("guardian.directoryHelp")}</p>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (valid)
                void run({
                  action: "configure",
                  config: {
                    ...data.config,
                    quotaBytes: String(limits[0]),
                    uploadBps: limits[1],
                    downloadBps: limits[2],
                  },
                });
            }}
          >
            <div className="guardian-controls">
              <label>
                {t("guardian.quota")}
                <input
                  type="number"
                  min="0"
                  max="1048576"
                  step="1"
                  value={quota}
                  onChange={(e) => setQuota(e.target.value)}
                  required
                />
              </label>
              <label>
                {t("guardian.upload")}
                <input
                  type="number"
                  min="0"
                  max="976562.5"
                  step="0.5"
                  value={up}
                  onChange={(e) => setUp(e.target.value)}
                  required
                />
              </label>
              <label>
                {t("guardian.download")}
                <input
                  type="number"
                  min="0"
                  max="976562.5"
                  step="0.5"
                  value={down}
                  onChange={(e) => setDown(e.target.value)}
                  required
                />
              </label>
            </div>
            <p>{t("guardian.limitsHelp")}</p>
            <button disabled={disabled || !valid} type="submit">
              {t("guardian.save")}
            </button>
          </form>
          <p role="status">
            {t(data.config.active ? "guardian.active" : "guardian.inactive")} ·{" "}
            {t("guardian.used", {
              used: (Number(data.usedBytes) / 1048576).toFixed(2),
              count: data.fragmentCount,
            })}{" "}
            · {t(`guardian.health.${data.health}`)}
          </p>
          <button
            disabled={!data || !isTauri() || (busy && !data.config.active)}
            onClick={() =>
              void run({
                action: "configure",
                config: { ...data.config, active: !data.config.active },
              })
            }
          >
            {t(data.config.active ? "guardian.disable" : "guardian.enable")}
          </button>
          <button
            disabled={disabled}
            onClick={() => void run({ action: "verify" })}
          >
            {t("guardian.verify")}
          </button>
          <button
            disabled={disabled}
            onClick={() => {
              if (window.confirm(t("guardian.pruneConfirm")))
                void run({ action: "prune" });
            }}
          >
            {t("guardian.prune")}
          </button>
          <GuardianNetworkSettings
            guardian={data}
            onChanged={() =>
              void run({ action: "status", offset: data.offset })
            }
          />
          <h3>{t("guardian.approveTitle")}</h3>
          <p>{t("guardian.approveHelp")}</p>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run({
                action: "approve",
                group,
                index: Number(index),
                sharing,
              });
            }}
          >
            <div className="guardian-controls">
              <label>
                {t("guardian.group")}
                <input
                  type="number"
                  min="0"
                  max="83332"
                  step="1"
                  required
                  value={group}
                  onChange={(e) => setGroup(e.target.value)}
                />
              </label>
              <label>
                {t("guardian.fragment")}
                <select
                  value={index}
                  onChange={(e) => setIndex(e.target.value)}
                >
                  {Array.from({ length: 12 }, (_, n) => (
                    <option key={n}>{n}</option>
                  ))}
                </select>
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={sharing}
                  onChange={(e) => setSharing(e.target.checked)}
                />
                {t("guardian.allowSharing")}
              </label>
            </div>
            <button disabled={disabled} type="submit">
              {t("guardian.approve")}
            </button>
          </form>
          <h3>{t("guardian.fragments")}</h3>
          {data.entries.length === 0 ? (
            <p>{t("guardian.empty")}</p>
          ) : (
            <ul className="guardian-fragments">
              {data.entries.map((f) => (
                <li key={f.id}>
                  <strong>{f.name}</strong>
                  <p>
                    {t("guardian.coordinate", {
                      group: f.group,
                      index: f.index,
                    })}{" "}
                    · {(f.sizeBytes / 1048576).toFixed(2)} MiB ·{" "}
                    {t(`guardian.health.${f.health}`)}
                  </p>
                  <p>
                    {t(
                      f.sharing
                        ? "guardian.shareAllowed"
                        : "guardian.storageOnly",
                    )}
                  </p>
                  <details>
                    <summary>SHA-256</summary>
                    <code>{f.sha256}</code>
                  </details>
                  <button
                    disabled={disabled || !data.config.active}
                    onClick={() => void run({ action: "import", id: f.id })}
                  >
                    {t("guardian.import")}
                  </button>
                  <button
                    disabled={
                      disabled ||
                      !data.config.active ||
                      !f.sharing ||
                      f.health === "missing" ||
                      f.health === "corrupt"
                    }
                    onClick={() => void run({ action: "export", id: f.id })}
                  >
                    {t("guardian.export")}
                  </button>
                  <button
                    disabled={disabled}
                    onClick={() =>
                      void run({ action: "revoke", id: f.id, delete: false })
                    }
                  >
                    {t("guardian.revoke")}
                  </button>
                  <button
                    disabled={disabled}
                    onClick={() => {
                      if (window.confirm(t("guardian.deleteConfirm")))
                        void run({ action: "revoke", id: f.id, delete: true });
                    }}
                  >
                    {t("guardian.delete")}
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div>
            <button
              disabled={disabled || data.offset === 0}
              onClick={() =>
                void run({
                  action: "status",
                  offset: Math.max(0, data.offset - 100),
                })
              }
            >
              {t("guardian.previous")}
            </button>
            <button
              disabled={disabled || data.offset + 100 >= data.total}
              onClick={() =>
                void run({ action: "status", offset: data.offset + 100 })
              }
            >
              {t("guardian.next")}
            </button>
          </div>
        </>
      )}
    </section>
  );
}
