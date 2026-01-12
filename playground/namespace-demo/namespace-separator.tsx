import { useTranslation } from "react-i18next";

/**
 * Namespace Separator パターン
 *
 * 設定: namespaceSeparator: ":"
 *
 * t("namespace:key") の形式で明示的に namespace を指定できる。
 * useTranslation で指定した namespace よりも優先される。
 */
export function NamespaceSeparator() {
  // デフォルトは "common" namespace
  const { t } = useTranslation("common");

  return (
    <div>
      {/* 通常: common namespace から検索 */}
      <h1>{t("greeting.hello")}</h1>

      {/* 明示的 namespace: errors namespace から検索 */}
      <p>{t("errors:notFound")}</p>
      <p>{t("errors:validation.email")}</p>

      {/* 明示的 namespace: translation namespace から検索 */}
      <p>{t("translation:welcome")}</p>
    </div>
  );
}
