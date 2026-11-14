import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  StorageDirectoryKind,
  StorageSettingsInfo,
} from "@game4/ipc-contracts";
import {
  isTauri,
  getStorageSettings,
  chooseStorageDirectory,
  resetStorageDirectory,
} from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";

export function StorageSettings() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<StorageSettingsInfo>();
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(isTauri());
  const [error, setError] = useState<unknown>();
  const [notice, setNotice] = useState("");
  const alive = useRef(true);
  async function refresh() {
    if (!isTauri()) return;
    setLoading(true);
    setError(undefined);
    try {
      const value = await getStorageSettings();
      if (alive.current) setSettings(value);
    } catch (e) {
      if (alive.current) setError(e);
    } finally {
      if (alive.current) setLoading(false);
    }
  }
  useEffect(() => {
    alive.current = true;
    void refresh();
    return () => {
      alive.current = false;
    };
  }, []);
  async function change(kind: StorageDirectoryKind, reset = false) {
    setBusy(true);
    setError(undefined);
    setNotice("");
    try {
      if (reset) {
        const value = await resetStorageDirectory({ kind });
        if (alive.current) {
          setSettings(value);
          setNotice("storage.resetDone");
        }
      } else {
        const result = await chooseStorageDirectory({ kind });
        if (alive.current) {
          setSettings(result.settings);
          if (!result.cancelled) setNotice("storage.saved");
        }
      }
    } catch (e) {
      if (alive.current) setError(e);
    } finally {
      if (alive.current) setBusy(false);
    }
  }
  return (
    <section
      className="content-card storage-settings"
      aria-labelledby="storage-title"
    >
      <h2 id="storage-title">{t("storage.title")}</h2>
      <p>{t("storage.description")}</p>
      {!isTauri() && <p>{t("errors.ipcUnavailable")}</p>}
      {loading && <p role="status">{t("storage.loading")}</p>}
      {error !== undefined && (
        <div className="error-message" role="alert">
          {t(getIpcErrorKey(error))}
          <button
            disabled={busy || loading || !isTauri()}
            onClick={() => void refresh()}
          >
            {t("library.refresh")}
          </button>
        </div>
      )}
      {settings && (
        <div className="storage-directories">
          {(["downloads", "exports", "backups"] as const).map((kind) => (
            <article key={kind}>
              <div>
                <h3>{t(`storage.${kind}`)}</h3>
                <p>{t(`storage.${kind}Hint`)}</p>
                <code>{settings[kind]?.path ?? t("storage.notSet")}</code>
                {settings[kind] && !settings[kind]?.available && (
                  <p className="storage-missing">{t("storage.missing")}</p>
                )}
              </div>
              <div className="actions">
                <button
                  disabled={busy || loading || !isTauri()}
                  onClick={() => void change(kind)}
                >
                  {t(settings[kind] ? "storage.change" : "storage.choose")}
                </button>
                {settings[kind] && (
                  <button
                    disabled={busy || loading || !isTauri()}
                    onClick={() => void change(kind, true)}
                  >
                    {t("storage.reset")}
                  </button>
                )}
              </div>
            </article>
          ))}
        </div>
      )}
      {notice && <p role="status">{t(notice)}</p>}
      {settings && (
        <details className="storage-internals">
          <summary>{t("storage.internal")}</summary>
          <p>{t("storage.internalHint")}</p>
          <code>{settings.appDataDirectory}</code>
        </details>
      )}
      <p className="storage-retention">{t("storage.existingHint")}</p>
    </section>
  );
}
