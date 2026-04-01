/**
 * next-intl getTranslations patterns (Server Components)
 *
 * getTranslations(namespace?)
 * getTranslations({ namespace?, locale? })
 */
import { getTranslations } from "next-intl/server";

// Basic usage (no namespace)
async function BasicPage() {
  const t = await getTranslations();

  return (
    <div>
      <h1>{t("common.hello")}</h1>
      <p>{t("common.goodbye")}</p>
    </div>
  );
}

// With string namespace
async function WithNamespace() {
  const t = await getTranslations("common");

  return (
    <div>
      <h1>{t("hello")}</h1>
      <p>{t("goodbye")}</p>
    </div>
  );
}

// With object argument (namespace only)
async function WithObjectNamespace() {
  const t = await getTranslations({ namespace: "common" });

  return <h1>{t("welcome", { name: "World" })}</h1>;
}

// With object argument (locale + namespace)
async function WithLocaleAndNamespace() {
  const t = await getTranslations({ locale: "en", namespace: "home" });

  return (
    <div>
      <h1>{t("title")}</h1>
      <p>{t("description")}</p>
    </div>
  );
}

// Metadata API pattern
async function generateMetadata({ params: { locale } }: { params: { locale: string } }) {
  const t = await getTranslations({ locale, namespace: "common" });

  return {
    title: t("hello"),
  };
}

// Method chains work the same as useTranslations
async function RichTextPage() {
  const t = await getTranslations("rich");

  return (
    <div>
      {t.rich("terms", {
        terms: (chunks) => <a href="/terms">{chunks}</a>,
      })}
      {t.markup("highlight", {
        highlight: (chunks) => `<mark>${chunks}</mark>`,
      })}
      {t.raw("html")}
    </div>
  );
}

export {
  BasicPage,
  WithNamespace,
  WithObjectNamespace,
  WithLocaleAndNamespace,
  generateMetadata,
  RichTextPage,
};
