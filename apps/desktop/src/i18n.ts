import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import ptBR from "./locales/pt-BR.json";

export type Locale = "en" | "pt-BR";
const storageKey = "game4.locale";

function savedLocale(): Locale {
  try {
    const value = localStorage.getItem(storageKey);
    return value === "pt-BR" ? value : "en";
  } catch {
    return "en";
  }
}

i18n.use(initReactI18next).init({
  resources: { en: { translation: en }, "pt-BR": { translation: ptBR } },
  lng: savedLocale(),
  fallbackLng: "en",
  supportedLngs: ["en", "pt-BR"],
  interpolation: { escapeValue: false },
});

document.documentElement.lang = i18n.language;
i18n.on("languageChanged", (language) => {
  document.documentElement.lang = language;
  try {
    localStorage.setItem(storageKey, language);
  } catch {
    // Language switching still works when local storage is unavailable.
  }
});

export default i18n;
