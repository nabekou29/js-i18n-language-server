import { useTranslation } from "react-i18next";

/**
 * Namespace 基本パターン
 *
 * useTranslation に namespace を指定すると、
 * その namespace のファイルからのみ補完・検索が行われる。
 */
export function NamespaceBasic() {
  // "common" namespace を使用
  // → locales/en/common.json, locales/ja/common.json から検索
  const { t } = useTranslation("common");

  return (
    <div>
      {/* common.json のキーが補完される */}
      <h1>{t("greeting.hello")}</h1>
      <p>{t("greeting.goodbye")}</p>

      <button>{t("button.save")}</button>
      <button>{t("button.cancel")}</button>
    </div>
  );
}

/**
 * 複数 namespace の使用
 */
export function MultipleNamespaces() {
  // それぞれ異なる namespace を使用
  const { t: tCommon } = useTranslation("common");
  const { t: tErrors } = useTranslation("errors");

  return (
    <div>
      {/* common namespace */}
      <button>{tCommon("button.delete")}</button>

      {/* errors namespace */}
      <p>{tErrors("notFound")}</p>
      <p>{tErrors("validation.required")}</p>
    </div>
  );
}
