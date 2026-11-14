import { useTranslation } from "react-i18next";
import { TransfersPage } from "./TransfersPage";
import { LibraryPage } from "./LibraryPage";
import { LibraryBackup } from "../components/LibraryBackup";
import { LanguageSelect } from "../components/LanguageSelect";

import { DashboardPage } from "./DashboardPage";

export type Page = "dashboard" | "library" | "transfers" | "tasks" | "settings";

export function WorkspacePage({
  page,
  onNavigate,
}: {
  page: Page;
  onNavigate: (page: Page) => void;
}) {
  const { t } = useTranslation();
  if (page === "dashboard") return <DashboardPage onNavigate={onNavigate} />;
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
