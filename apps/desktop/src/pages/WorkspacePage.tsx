import { useTranslation } from "react-i18next";
import { TransfersPage } from "./TransfersPage";
import { LibraryPage } from "./LibraryPage";
import { LibraryBackup } from "../components/LibraryBackup";
import { LanguageSelect } from "../components/LanguageSelect";

export type Page = "library" | "transfers" | "tasks" | "settings";

export function WorkspacePage({ page }: { page: Page }) {
  const { t } = useTranslation();
  if (page === "transfers") return <TransfersPage />;
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
      <LibraryBackup />
    </>
  );
}
