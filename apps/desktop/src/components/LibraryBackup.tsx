import { useRef, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LibraryRestorePreview } from "@game4/ipc-contracts";
import {
  saveLibraryBackup,
  prepareLibraryRestore,
  restoreLibraryBackup,
  isTauri,
} from "../services/desktop";
import { getIpcErrorKey } from "../services/errors";

import { useNotifications } from "./Notifications";
export function LibraryBackup() {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<LibraryRestorePreview>();
  const [error, setError] = useState<unknown>();
  const [message, setMessage] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  async function save() {
    setBusy(true);
    setError(undefined);
    setMessage("");
    try {
      const result = await saveLibraryBackup();
      if (mounted.current && !result.cancelled) {
        setMessage(t("backup.saved", { fileName: result.fileName }));
        notify({
          key: "notifications.backupSaved",
          kind: "success",
          target: "settings",
        });
      }
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  async function prepare() {
    setBusy(true);
    setError(undefined);
    setMessage("");
    setPreview(undefined);
    setConfirmed(false);
    try {
      const result = await prepareLibraryRestore();
      if (mounted.current && result.preview) setPreview(result.preview);
    } catch (e) {
      if (mounted.current) setError(e);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  async function restore() {
    if (!preview || !confirmed) return;
    setBusy(true);
    setError(undefined);
    try {
      const result = await restoreLibraryBackup({
        restoreToken: preview.restoreToken,
      });
      if (mounted.current) {
        setPreview(undefined);
        setConfirmed(false);
        notify({
          key: "notifications.libraryRestored",
          kind: "success",
          target: "settings",
        });
        setMessage(
          t("backup.restored", {
            count: result.itemCount,
            recovery: result.recoveryFileName,
          }),
        );
      }
    } catch (e) {
      if (mounted.current) {
        setError(e);
        setPreview(undefined);
        setConfirmed(false);
      }
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  return (
    <section
      className="content-card backup-card library-workspace"
      aria-labelledby="backup-title"
    >
      <div>
        <h2 id="backup-title">{t("backup.title")}</h2>
        <p>{t("backup.description")}</p>
        <p>{t("backup.privateData")}</p>
      </div>
      <div className="actions">
        <button
          disabled={busy || !isTauri() || !!preview}
          onClick={() => void save()}
        >
          {t("backup.save")}
        </button>
        <button disabled={busy || !isTauri()} onClick={() => void prepare()}>
          {t("backup.chooseRestore")}
        </button>
      </div>
      {!isTauri() && <p>{t("errors.ipcUnavailable")}</p>}
      {busy && <p role="status">{t("backup.working")}</p>}
      {message && <p role="status">{message}</p>}
      {error !== undefined && (
        <p className="error-message" role="alert">
          {t(getIpcErrorKey(error))}
        </p>
      )}
      {preview && (
        <div className="restore-preview">
          <h3>{t("backup.previewTitle")}</h3>
          <p>{preview.fileName}</p>
          <dl>
            <div>
              <dt>{t("backup.items")}</dt>
              <dd>{preview.itemCount}</dd>
            </div>
            <div>
              <dt>{t("backup.files")}</dt>
              <dd>{preview.fileCount}</dd>
            </div>
            <div>
              <dt>{t("backup.locations")}</dt>
              <dd>{preview.locationCount}</dd>
            </div>
            <div>
              <dt>{t("backup.tasks")}</dt>
              <dd>{preview.taskCount}</dd>
            </div>
          </dl>
          <p>{t("backup.replaceWarning")}</p>
          <label className="restore-confirm">
            <input
              type="checkbox"
              checked={confirmed}
              disabled={busy}
              onChange={(e) => setConfirmed(e.target.checked)}
            />
            {t("backup.confirmReplacement")}
          </label>
          <div className="actions">
            <button
              className="restore-button"
              disabled={busy || !confirmed}
              onClick={() => void restore()}
            >
              {t("backup.restore")}
            </button>
            <button
              disabled={busy}
              onClick={() => {
                setPreview(undefined);
                setConfirmed(false);
              }}
            >
              {t("backup.cancel")}
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
