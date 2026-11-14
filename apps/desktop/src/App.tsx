import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LanguageSelect } from "./components/LanguageSelect";
import { useAppInfo } from "./hooks/useAppInfo";
import { getIpcErrorKey } from "./services/errors";
import { WorkspacePage, type Page } from "./pages/WorkspacePage";
import "./App.css";

export default function App() {
  const { t } = useTranslation();
  const [page, setPage] = useState<Page>("dashboard");
  const { runtime, version, failure } = useAppInfo();

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a className="brand" href="#main" aria-label="Game4∞">
          <span className="brand-mark">∞</span>Game4
          <span className="brand-infinity">∞</span>
        </a>
        <p className="sidebar-caption">{t("app.workspace")}</p>
        <nav aria-label={t("nav.label")}>
          {(
            ["dashboard", "library", "transfers", "tasks", "settings"] as const
          ).map((item, index) => (
            <button
              key={item}
              className={`nav-item ${page === item ? "active" : ""}`}
              aria-current={page === item ? "page" : undefined}
              onClick={() => setPage(item)}
            >
              <span className="nav-number" aria-hidden="true">
                0{index + 1}
              </span>
              {t(`nav.${item}`)}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <span className="status-dot" />
          {t("app.localFirst")}
          <p>{t("app.tagline")}</p>
        </div>
      </aside>
      <main id="main" className="main-content">
        <header className="topbar">
          <span className="stage-badge">{t("app.stage")}</span>
          <LanguageSelect />
        </header>
        <section className="page-heading">
          <p className="eyebrow">{t("app.eyebrow")}</p>
          <h1>{t(`${page}.title`)}</h1>
          <p>{t(`${page}.description`)}</p>
        </section>
        <WorkspacePage page={page} onNavigate={setPage} />
        <footer className="runtime-status" role="status">
          <span>
            {failure ? t(getIpcErrorKey(failure)) : t(`runtime.${runtime}`)}
          </span>
          {version && <span>v{version}</span>}
        </footer>
      </main>
    </div>
  );
}
