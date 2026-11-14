import type { G4epSummary } from "@game4/ipc-contracts";
import { useTranslation } from "react-i18next";
export function G4epManifestDetails({ summary }: { summary: G4epSummary }) {
  const { t } = useTranslation();
  return (
    <section className="g4ep-manifest-details">
      <h3>{t("ever.g4epTitle")}</h3>
      <p>
        {t("ever.g4epGeometry", {
          groups: summary.groupCount,
          fragments: summary.fragmentCount,
        })}
      </p>
      <p>{t("ever.g4epReadOnly")}</p>
      <p>
        {t(
          summary.signerKeyIds.length ? "ever.g4epSigned" : "ever.g4epUnsigned",
        )}
      </p>
      {summary.signerKeyIds.length > 0 && <p>{t("ever.g4epTrust")}</p>}
      <details>
        <summary>{t("ever.g4epTechnical")}</summary>
        <dl>
          <dt>{t("ever.g4epContentId")}</dt>
          <dd>
            <code>{summary.contentId}</code>
          </dd>
          <dt>{t("ever.g4epCodec")}</dt>
          <dd>
            <code>{summary.codecId}</code>
          </dd>
          <dt>{t("ever.g4epMetadataHash")}</dt>
          <dd>
            <code>{summary.payloadSha256}</code>
          </dd>
          {summary.signerKeyIds.map((key) => (
            <div key={key}>
              <dt>{t("ever.g4epSigner")}</dt>
              <dd>
                <code>{key}</code>
              </dd>
            </div>
          ))}
        </dl>
      </details>
    </section>
  );
}
