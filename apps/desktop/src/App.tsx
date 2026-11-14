import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NotificationCenter } from "./components/Notifications";
import { AssociatedFiles } from "./components/AssociatedFiles";
import { LanguageSelect } from "./components/LanguageSelect";
import { useAppInfo } from "./hooks/useAppInfo";
import { getIpcErrorKey } from "./services/errors";
import { WorkspacePage, type Page } from "./pages/WorkspacePage";
import "./App.css";
import brandLogo from "../../../assets/White_SF.png";
import type { LibraryItemSummary } from "@game4/ipc-contracts";
import type { Navigate } from "./services/navigation";

export default function App() {
  const { t } = useTranslation();
  const [page, setPage] = useState<Page>("dashboard");
  const [source, setSource] = useState<LibraryItemSummary>();
  const [initialMagnet, setInitialMagnet] = useState<string>();
  const navigate: Navigate = (next, selected, magnet) => {
    setInitialMagnet(magnet);
    setSource(selected);
    setPage(next);
    window.scrollTo({ top: 0 });
  };
  const { runtime, version, failure } = useAppInfo();

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <button
          className="brand brand-home"
          onClick={() => navigate("dashboard")}
          aria-label={t("nav.homeLabel")}
        >
          <img src={brandLogo} alt="Game4∞ — Game For Ever" />
        </button>
        <p className="sidebar-caption">{t("app.workspace")}</p>
        <nav aria-label={t("nav.label")}>
          {(
            [
              "dashboard",
              "download",
              "share",
              "library",
              "transfers",
              "community",
              "tasks",
              "settings",
            ] as const
          ).map((item, index) => (
            <button
              key={item}
              className={`nav-item ${page === item ? "active" : ""}`}
              aria-current={page === item ? "page" : undefined}
              onClick={() => navigate(item)}
            >
              <span className="nav-number" aria-hidden="true">
                0{index + 1}
              </span>
              {t(`nav.${item}`)}
              {item === "community" && (
                <span className="nav-soon">{t("community.soon")}</span>
              )}
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
          <div className="topbar-controls">
            <NotificationCenter onNavigate={navigate} />
            <LanguageSelect />
          </div>
        </header>
        <section className="page-heading">
          <p className="eyebrow">{t("app.eyebrow")}</p>
          <h1>{t(`${page}.title`)}</h1>
          <p>{t(`${page}.description`)}</p>
        </section>
        <AssociatedFiles
          onDownloadLink={(magnet) => navigate("download", undefined, magnet)}
        />
        <WorkspacePage
          page={page}
          onNavigate={navigate}
          source={source}
          initialMagnet={initialMagnet}
        />
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
