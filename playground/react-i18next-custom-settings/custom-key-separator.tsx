import { useTranslation } from "react-i18next";

/**
 * key_separator が "_" の場合のサンプル
 *
 * 設定: keySeparator: "_"
 *
 * 確認項目:
 * - 補完: t("") 入力時にキー候補が表示される
 * - ホバー: キー上でホバーすると翻訳値が表示される
 * - 診断: 存在しないキーに警告が表示される
 * - 定義へ移動: キーから翻訳ファイルへジャンプできる
 */
export function CustomKeySeparator() {
  const { t } = useTranslation();

  return (
    <div>
      {/* 正常なキー */}
      <p>{t("common_greeting_hello")}</p>
      <p>{t("common_greeting_goodbye")}</p>
      <p>{t("common_buttons_save")}</p>
      <p>{t("common_buttons_cancel")}</p>

      {/* 存在しないキー（診断が表示されるべき） */}
      <p>{t("common_nonexistent")}</p>
    </div>
  );
}
