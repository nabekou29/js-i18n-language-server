import { useTranslation } from "react-i18next";

/**
 * namespace_separator が設定されている場合のサンプル（将来対応予定）
 *
 * 設定: namespaceSeparator: ":"
 *
 * 現時点では namespace_separator は未実装のため、
 * このファイルは将来の動作確認用に準備しておく。
 */
export function NamespaceSeparator() {
  // namespace を明示的に指定
  const { t } = useTranslation("common");
  const { t: tErrors } = useTranslation("errors");

  return (
    <div>
      {/* common namespace */}
      <p>{t("greeting.hello")}</p>

      {/* errors namespace */}
      <p>{tErrors("notFound")}</p>
      <p>{tErrors("serverError")}</p>
    </div>
  );
}
