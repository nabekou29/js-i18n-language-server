import { TFunction } from "i18next";
import { t } from "i18next";

// Type-safe translation keys
type TranslationKeys = 
  | "app.name"
  | "app.version"
  | "user.profile.name"
  | "user.profile.email";

export function getTranslation(key: TranslationKeys): string {
  return t(key);
}

// With interpolation
export const userGreeting = t("user.greeting", { name: "John" });

// Array of keys
const keys: TranslationKeys[] = ["app.name", "app.version"];
export const translations = keys.map(key => t(key));