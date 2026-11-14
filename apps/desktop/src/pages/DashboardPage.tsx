import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LibraryPage, TaskPage, TransferPage } from "@game4/ipc-contracts";
import {
  isTauri,
  listLibrary,
  listTasks,
  listTransfers,
} from "../services/desktop";
import { formatBytes } from "../services/file-identification";
import { getIpcErrorKey } from "../services/errors";
import type { Navigate } from "../services/navigation";
import brandLogo from "../../../../assets/White_SF.png";

export function DashboardPage({ onNavigate }: { onNavigate: Navigate }) {
  const { t, i18n } = useTranslation();
  const [library, setLibrary] = useState<LibraryPage>();
  const [tasks, setTasks] = useState<TaskPage>();
  const [transfers, setTransfers] = useState<TransferPage>();
  const [errors, setErrors] = useState<unknown[]>([]);
  const [loading, setLoading] = useState(isTauri());
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    if (!isTauri()) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      const results = await Promise.allSettled([
        listLibrary({ page: { cursor: null, limit: 50 }, query: null }),
        listTasks({ cursor: null, limit: 50 }),
        listTransfers(),
      ]);
      if (!alive) return;
      const [collection, jobs, sessions] = results;
      setLibrary(
        collection.status === "fulfilled" ? collection.value : undefined,
      );
      setTasks(jobs.status === "fulfilled" ? jobs.value : undefined);
      setTransfers(
        sessions.status === "fulfilled" ? sessions.value : undefined,
      );
      setErrors(
        results.flatMap((result) =>
          result.status === "rejected" ? [result.reason] : [],
        ),
      );
      setLoading(false);
      timer = setTimeout(refresh, 5000);
    };
    setLoading(true);
    void refresh();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [revision]);
  const bytes = library?.items.reduce(
    (sum, item) => sum + BigInt(item.totalBytes),
    0n,
  );
  const value = (count: number | undefined) =>
    count === undefined
      ? "—"
      : new Intl.NumberFormat(i18n.language).format(count);
  const stats = [
    ["items", value(library?.items.length), "◫"],
    [
      "storage",
      bytes === undefined ? "—" : formatBytes(bytes.toString(), i18n.language),
      "▤",
    ],
    [
      "active",
      value(transfers?.items.filter((item) => item.status === "active").length),
      "↗",
    ],
    [
      "paused",
      value(transfers?.items.filter((item) => item.status === "paused").length),
      "Ⅱ",
    ],
  ];
  return (
    <div className="dashboard">
      <section className="dashboard-hero">
        <div className="hero-copy">
          <p className="eyebrow">{t("dashboard.heroLabel")}</p>
          <h2>{t("dashboard.heroTitle")}</h2>
          <p>{t("dashboard.heroText")}</p>
          <div className="actions">
            <button
              className="dashboard-primary"
              onClick={() => onNavigate("download")}
            >
              {t("journeys.downloadAction")} <span aria-hidden="true">↗</span>
            </button>
            <button
              className="dashboard-secondary"
              onClick={() => onNavigate("share")}
            >
              {t("journeys.shareAction")}
            </button>
          </div>
        </div>
        <div className="mission-art" aria-hidden="true">
          <img src={brandLogo} alt="" />
          <p>{t("app.tagline")}</p>
        </div>
      </section>
      <section className="journey-grid" aria-label={t("journeys.title")}>
        {(["download", "share", "community"] as const).map((action, index) => (
          <article key={action}>
            <span className="journey-index" aria-hidden="true">
              0{index + 1}
            </span>
            {action === "community" && (
              <span className="planned-label">{t("community.soon")}</span>
            )}
            <h2>{t(`journeys.${action}Title`)}</h2>
            <p>{t(`journeys.${action}Text`)}</p>
            <button
              className="dashboard-link"
              onClick={() => onNavigate(action)}
            >
              {t(`journeys.${action}Action`)} →
            </button>
          </article>
        ))}
      </section>
      <div className="dashboard-section-heading">
        <h2>{t("dashboard.collection")}</h2>
        <button
          className="dashboard-link"
          disabled={loading || !isTauri()}
          onClick={() => setRevision((v) => v + 1)}
        >
          {t("dashboard.refresh")} <span aria-hidden="true">↻</span>
        </button>
      </div>
      <div role="status" className="dashboard-notice">
        {loading
          ? t("dashboard.loading")
          : !isTauri()
            ? t("dashboard.preview")
            : null}
      </div>
      {errors.map((error, index) => (
        <p className="error-message" role="alert" key={index}>
          {t(getIpcErrorKey(error))}
        </p>
      ))}
      <div className="dashboard-stats">
        {stats.map(([key, count, icon]) => (
          <article className="metric-card" key={key}>
            <div>
              <span>{t(`dashboard.${key}`)}</span>
              <span className="metric-icon" aria-hidden="true">
                {icon}
              </span>
            </div>
            <strong>{count}</strong>
          </article>
        ))}
      </div>
      <details className="dashboard-footnote metric-scope">
        <summary>{t("dashboard.metricScope")}</summary>
        {t("dashboard.scope")} {library?.nextCursor && t("dashboard.partial")}{" "}
        {t("dashboard.transferScope")}
      </details>
      <div className="dashboard-columns">
        <section className="dashboard-panel">
          <div className="dashboard-section-heading">
            <h2>{t("dashboard.recent")}</h2>
            <button
              className="dashboard-link"
              onClick={() => onNavigate("library")}
            >
              {t("dashboard.viewAll")} →
            </button>
          </div>
          {library && library.items.length > 0 ? (
            <div className="dashboard-collection">
              {library.items.slice(0, 4).map((item, index) => (
                <button
                  className="collection-entry"
                  key={item.id}
                  onClick={() => onNavigate("library")}
                >
                  <span
                    className={`collection-art art-${index}`}
                    aria-hidden="true"
                  >
                    {item.title.slice(0, 1).toLocaleUpperCase(i18n.language)}
                    <small>Game For Ever</small>
                  </span>
                  <strong>{item.title}</strong>
                  <span>
                    {t("library.fileCount", { count: item.fileCount })} ·{" "}
                    {formatBytes(item.totalBytes, i18n.language)}
                  </span>
                </button>
              ))}
            </div>
          ) : (
            <div className="dashboard-empty">
              <span aria-hidden="true">◫</span>
              <h3>
                {library
                  ? t("dashboard.empty")
                  : loading
                    ? t("dashboard.loading")
                    : isTauri()
                      ? t("dashboard.unavailable")
                      : t("dashboard.empty")}
              </h3>
              <p>{t("dashboard.emptyText")}</p>
              <button
                className="dashboard-secondary"
                onClick={() => onNavigate("library")}
              >
                {t("library.addFiles")} →
              </button>
            </div>
          )}
        </section>
        <section className="dashboard-panel">
          <div className="dashboard-section-heading">
            <h2>{t("dashboard.activity")}</h2>
            <span className="activity-dot" aria-hidden="true" />
          </div>
          {tasks?.items.length ? (
            <ul className="dashboard-activity">
              {tasks.items.slice(0, 5).map((task) => (
                <li key={task.id}>
                  <span
                    className={`activity-icon ${task.status}`}
                    aria-hidden="true"
                  >
                    {task.kind === "verify" ? "✓" : "↓"}
                  </span>
                  <div>
                    <strong>{t(`tasks.kind.${task.kind}`)}</strong>
                    <span>{t(`tasks.status.${task.status}`)}</span>
                  </div>
                  <time dateTime={task.updatedAt}>
                    {new Intl.DateTimeFormat(i18n.language, {
                      month: "short",
                      day: "numeric",
                    }).format(new Date(task.updatedAt))}
                  </time>
                </li>
              ))}
            </ul>
          ) : (
            <p className="dashboard-activity-empty">
              {tasks
                ? t("dashboard.noActivity")
                : loading
                  ? t("dashboard.loading")
                  : isTauri()
                    ? t("dashboard.unavailable")
                    : t("dashboard.noActivity")}
            </p>
          )}
          <p className="dashboard-footnote">{t("dashboard.activityScope")}</p>
          <button
            className="dashboard-link"
            onClick={() => onNavigate("tasks")}
          >
            {t("nav.tasks")} →
          </button>
        </section>
      </div>
      <section
        aria-label={t("dashboard.principles")}
        className="dashboard-principles"
      >
        {["originals", "integrity", "network"].map((key, index) => (
          <article key={key}>
            <span aria-hidden="true">0{index + 1} /</span>
            <div>
              <h3>{t(`dashboard.${key}`)}</h3>
              <p>{t(`dashboard.${key}Text`)}</p>
            </div>
          </article>
        ))}
      </section>
    </div>
  );
}
