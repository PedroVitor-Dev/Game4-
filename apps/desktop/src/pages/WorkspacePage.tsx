import { useTranslation } from "react-i18next";
import { TransfersPage } from "./TransfersPage";
import { LibraryPage } from "./LibraryPage";
import { LibraryBackup } from "../components/LibraryBackup";
import { StorageSettings } from "../components/StorageSettings";
import { LanguageSelect } from "../components/LanguageSelect";

import { DashboardPage } from "./DashboardPage";

import { CommunityPage } from "./CommunityPage";
import type { LibraryItemSummary } from "@game4/ipc-contracts";
import { transferView, type Navigate, type Page } from "../services/navigation";
export type { Page } from "../services/navigation";

export function WorkspacePage({
  page,
  onNavigate,
  source,
  initialMagnet,
}: {
  page: Page;
  onNavigate: Navigate;
  source?: LibraryItemSummary;
  initialMagnet?: string;
}) {
  const { t } = useTranslation();
  if (page === "dashboard") return <DashboardPage onNavigate={onNavigate} />;
  if (page === "community") return <CommunityPage onNavigate={onNavigate} />;
  const view = transferView(page);
  if (view)
    return (
      <TransfersPage
        key={`${page}:${source?.id ?? ""}:${initialMagnet ?? ""}`}
        view={view}
        source={source}
        initialMagnet={initialMagnet}
        onNavigate={onNavigate}
      />
    );
  if (page !== "settings")
    return (
      <LibraryPage
        key={page}
        tasksOnly={page === "tasks"}
        onShare={(item) => onNavigate("share", item)}
      />
    );
  return (
    <>
      <section className="content-card settings-card">
        <div>
          <h2>{t("language.title")}</h2>
          <p>{t("language.description")}</p>
        </div>
        <LanguageSelect />
      </section>
      <StorageSettings />
      <LibraryBackup />
    </>
  );
}
