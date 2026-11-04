import { useTranslation } from "react-i18next";
import { LanguageSelect } from "../components/LanguageSelect";

export type Page = "library" | "tasks" | "settings";

export function WorkspacePage({ page }: { page: Page }) {
  const { t } = useTranslation();
  return (
    <>
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
    </>
  );
}
