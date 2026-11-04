import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { Locale } from "./i18n";
import "./App.css";

type Page = "library" | "tasks" | "settings";
type Runtime = "checking" | "connected" | "preview" | "unavailable";

function LanguageSelect() {
  const { t, i18n } = useTranslation();
  return (
    <label className="language-control">
      <span>{t("language.label")}</span>
      <select
        value={i18n.language}
        onChange={(event) =>
          void i18n.changeLanguage(event.target.value as Locale)
        }
      >
        <option value="en">English</option>
        <option value="pt-BR">Português (Brasil)</option>
      </select>
    </label>
  );
}

export default function App() {
  const { t } = useTranslation();
  const [page, setPage] = useState<Page>("library");
  const [runtime, setRuntime] = useState<Runtime>("checking");
  const [version, setVersion] = useState<string>();

  useEffect(() => {
    let active = true;
    if (!isTauri()) {
      setRuntime("preview");
      return;
    }
    invoke<{ version: string }>("app_info")
      .then((info) => {
        if (active) {
          setVersion(info.version);
          setRuntime("connected");
        }
      })
      .catch(() => {
        if (active) setRuntime("unavailable");
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a className="brand" href="#main" aria-label="Game4∞">
          <span className="brand-mark">∞</span>Game4
          <span className="brand-infinity">∞</span>
        </a>
        <p className="sidebar-caption">{t("app.workspace")}</p>
        <nav aria-label={t("nav.label")}>
          {(["library", "tasks", "settings"] as const).map((item, index) => (
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
        {page === "settings" ? (
          <section className="content-card settings-card">
            <div>
              <h2>{t("language.title")}</h2>
              <p>{t("language.description")}</p>
            </div>
            <LanguageSelect />
          </section>
        ) : (
          <section className="content-card empty-state">
            <div className="empty-icon" aria-hidden="true">
              {page === "library" ? "▤" : "≋"}
            </div>
            <h2>{t(`${page}.emptyTitle`)}</h2>
            <p>{t(`${page}.emptyDescription`)}</p>
            <span className="planned-label">{t("app.planned")}</span>
          </section>
        )}
        <footer className="runtime-status" role="status">
          <span>{t(`runtime.${runtime}`)}</span>
          {version && <span>v{version}</span>}
        </footer>
      </main>
    </div>
  );
}
