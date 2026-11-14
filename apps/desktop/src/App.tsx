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

const navIcons: Record<string, string> = {
  dashboard: "M3 11 12 3l9 8v10h-6v-7H9v7H3Z",
  archive: "M3 4h18v4H3Zm2 4v13h14V8M9 12h6",
  platforms: "M4 5h16v11H4Zm4 15h8m-4-4v4",
  library: "M4 4h4v17H4Zm6 0h4v17h-4Zm6 1 4-1 3 16-4 1Z",
  download: "M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5",
  share: "M12 16V4m-5 5 5-5 5 5M4 16v5h16v-5",
  transfers: "m3 7 4-4m-4 4 4 4M3 7h18m0 10-4-4m4 4-4 4M3 17h18",
  community:
    "M8 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm8-1a3 3 0 1 0 0-6M1 21v-3a7 7 0 0 1 14 0v3m2-8a6 6 0 0 1 6 6v2",
  collections: "M3 6h7l2 2h9v13H3Zm2-3h7l2 2h5",
  preservation: "M12 3 3 7v6c0 5 9 9 9 9s9-4 9-9V7Zm-5 9 3 3 7-7",
  tasks: "M9 5h12M9 12h12M9 19h12M3 5h1M3 12h1M3 19h1",
  settings:
    "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM9 3h6l1 4 4 1v8l-4 1-1 4H9l-1-4-4-1V8l4-1Z",
};
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
              "archive",
              "platforms",
              "download",
              "share",
              "library",
              "transfers",
              "community",
              "collections",
              "preservation",
              "tasks",
              "settings",
            ] as const
          ).map((item) => (
            <button
              key={item}
              className={`nav-item ${page === item ? "active" : ""}`}
              aria-current={page === item ? "page" : undefined}
              onClick={() => navigate(item)}
            >
              <span className="nav-number" aria-hidden="true">
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.6"
                >
                  <path d={navIcons[item]} />
                </svg>
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
