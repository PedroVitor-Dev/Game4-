import { useTranslation } from "react-i18next";
import type { Navigate } from "../services/navigation";
export function CommunityPage({ onNavigate }: { onNavigate: Navigate }) {
  const { t } = useTranslation();
  return (
    <div className="community-page">
      <section className="workspace-banner">
        <div>
          <span className="planned-label">{t("community.soon")}</span>
          <h2>{t("community.hero")}</h2>
          <p>{t("community.notAvailable")}</p>
        </div>
        <span className="banner-symbol" aria-hidden="true">
          ∞
        </span>
      </section>
      <section className="journey-grid">
        {["catalogs", "stories", "guardians"].map((key) => (
          <article key={key}>
            <h2>{t(`community.${key}`)}</h2>
            <p>{t(`community.${key}Text`)}</p>
          </article>
        ))}
      </section>
      <section className="content-card community-current">
        <h2>{t("community.now")}</h2>
        <p>{t("community.nowText")}</p>
        <div className="actions">
          <button
            className="dashboard-primary"
            onClick={() => onNavigate("download")}
          >
            {t("journeys.downloadAction")}
          </button>
          <button
            className="dashboard-secondary"
            onClick={() => onNavigate("library")}
          >
            {t("dashboard.openLibrary")}
          </button>
        </div>
      </section>
    </div>
  );
}
