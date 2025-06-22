import { t } from "i18next";

// Simple t function calls
export const simpleKey = t("simple.key");
export const nestedKey = t("nested.path.to.key");

// With i18n object
import i18n from "i18next";

export const withI18n = i18n.t("with.i18n.object");

// Multiple calls
export function multipleTranslations() {
  const greeting = t("common.greeting");
  const farewell = t("common.farewell");
  const error = i18n.t("errors.notFound");
  
  return { greeting, farewell, error };
}