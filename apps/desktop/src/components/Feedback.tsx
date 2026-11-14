import { useTranslation } from "react-i18next";
import { getIpcErrorKey } from "../services/errors";
export function ErrorNotice({
  error,
  title,
  onRetry,
  onDismiss,
  busy = false,
}: {
  error: unknown;
  title?: string;
  onRetry?: () => void;
  onDismiss?: () => void;
  busy?: boolean;
}) {
  const { t } = useTranslation();
  return (
    <section className="error-message error-notice" role="alert">
      <div>
        <h3>{t(title ?? "feedback.errorTitle")}</h3>
        <p>{t(getIpcErrorKey(error))}</p>
      </div>
      {(onRetry || onDismiss) && (
        <div className="actions">
          {onRetry && (
            <button disabled={busy} onClick={onRetry}>
              {t("feedback.retry")}
            </button>
          )}
          {onDismiss && (
            <button disabled={busy} onClick={onDismiss}>
              {t("feedback.dismiss")}
            </button>
          )}
        </div>
      )}
    </section>
  );
}
export function EmptyState({
  title,
  description,
  action,
  onAction,
  disabled = false,
  icon = "◫",
}: {
  title: string;
  description: string;
  action?: string;
  onAction?: () => void;
  disabled?: boolean;
  icon?: string;
}) {
  const { t } = useTranslation();
  return (
    <section className="content-card empty-state">
      <span className="empty-icon" aria-hidden="true">
        {icon}
      </span>
      <h2>{t(title)}</h2>
      <p>{t(description)}</p>
      {action && onAction && (
        <button className="primary" disabled={disabled} onClick={onAction}>
          {t(action)}
        </button>
      )}
    </section>
  );
}
