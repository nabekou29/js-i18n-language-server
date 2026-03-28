import { register, init, getLocaleFromNavigator } from "svelte-i18n";

register("en", () => import("../locales/en.json"));
register("ja", () => import("../locales/ja.json"));

init({
  fallbackLocale: "en",
  initialLocale: getLocaleFromNavigator(),
});
