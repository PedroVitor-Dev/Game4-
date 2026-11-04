import { useTranslation } from "react-i18next";
import type { Locale } from "../i18n";

export function LanguageSelect() {
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
