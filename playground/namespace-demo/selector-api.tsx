import { Trans, useTranslation } from "react-i18next";

/**
 * Selector API + Namespace パターン
 *
 * useTranslation に namespace を指定した上で Selector API を使用
 */
export function SelectorWithNamespace() {
  const { t } = useTranslation("common");

  return (
    <div>
      <h1>{t(($) => $.greeting.hello)}</h1>
      <p>{t(($) => $.greeting.goodbye)}</p>

      <button>{t(($) => $.button.save)}</button>
      <button>{t(($) => $.button.cancel)}</button>
    </div>
  );
}

/**
 * Selector API + 複数 Namespace
 */
export function SelectorMultipleNamespaces() {
  const { t: tCommon } = useTranslation("common");
  const { t: tErrors } = useTranslation("errors");

  return (
    <div>
      <button>{tCommon(($) => $.button.delete)}</button>

      <p>{tErrors(($) => $.notFound)}</p>
      <p>{tErrors(($) => $.validation.required)}</p>
    </div>
  );
}

/**
 * Selector API + 明示的な namespace オーバーライド
 */
export function SelectorExplicitNamespace() {
  const { t } = useTranslation("common");

  return (
    <div>
      {/* common namespace のキー */}
      <h1>{t(($) => $.greeting.hello)}</h1>

      {/* ns オプションで errors namespace に切り替え */}
      <p>{t(($) => $.notFound, { ns: "errors" })}</p>
      <p>{t(($) => $.validation.email, { ns: "errors" })}</p>
    </div>
  );
}

/**
 * Selector API + Trans コンポーネント
 */
export function SelectorTrans() {
  const { t } = useTranslation("common");

  return (
    <div>
      <Trans i18nKey={($) => $.greeting.hello} t={t} />
    </div>
  );
}
