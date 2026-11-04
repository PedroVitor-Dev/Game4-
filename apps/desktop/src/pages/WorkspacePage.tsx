import { useTranslation } from "react-i18next";
import { LibraryPage } from "./LibraryPage";
import { LanguageSelect } from "../components/LanguageSelect";

export type Page = "library" | "tasks" | "settings";

export function WorkspacePage({ page }: { page: Page }) {
  const { t } = useTranslation();
  if (page !== "settings")
    return <LibraryPage key={page} tasksOnly={page === "tasks"} />;
  return (
    <>
      <section className="content-card settings-card">
        <div>
          <h2>{t("language.title")}</h2>
          <p>{t("language.description")}</p>
        </div>
        <LanguageSelect />
      </section>
    </>
  );
}
