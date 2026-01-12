import { useTranslation } from "react-i18next";

/**
 * Default Namespace パターン
 *
 * 設定: defaultNamespace: "translation"
 *
 * useTranslation() で namespace を指定しない場合、
 * defaultNamespace が使用される。
 */
export function DefaultNamespace() {
  // namespace を指定しない → defaultNamespace ("translation") が使用される
  const { t } = useTranslation();

  return (
    <div>
      {/* translation.json から検索される */}
      <h1>{t("welcome")}</h1>
      <p>{t("description")}</p>
    </div>
  );
}

/**
 * Default Namespace + 明示的指定の組み合わせ
 */
export function MixedNamespaces() {
  // namespace なし → defaultNamespace
  const { t } = useTranslation();

  return (
    <div>
      {/* デフォルト: translation namespace */}
      <h1>{t("welcome")}</h1>

      {/* 明示的: common namespace (separator 形式) */}
      <button>{t("common:button.save")}</button>

      {/* 明示的: errors namespace (separator 形式) */}
      <p>{t("errors:serverError")}</p>
    </div>
  );
}
