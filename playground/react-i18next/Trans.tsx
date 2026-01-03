/**
 * Trans コンポーネントの様々なパターン
 */
import { Trans, useTranslation } from "react-i18next";

// 基本的な使い方 (self-closing)
function BasicTrans() {
  const { t } = useTranslation();

  return <Trans i18nKey="trans.simple" t={t} />;
}

// self-closing + t 属性なし (デフォルトの t を使用)
function TransWithoutT() {
  const { t } = useTranslation();

  // t 属性がないので、デフォルトのスコープの t を使用
  return <Trans i18nKey="trans.simple" />;
}

// コンポーネントを含む Trans
function TransWithComponents() {
  const { t } = useTranslation();

  return (
    <Trans
      i18nKey="trans.with_component"
      t={t}
      components={{
        link: <a href="/more" />,
      }}
    />
  );
}

// 変数を含む Trans
function TransWithValues() {
  const { t } = useTranslation();

  return (
    <Trans
      i18nKey="trans.nested"
      t={t}
      values={{ name: "John" }}
      components={{
        bold: <strong />,
      }}
    />
  );
}

// 子要素を持つ Trans
function TransWithChildren() {
  const { t } = useTranslation();

  return (
    <Trans i18nKey="trans.with_component" t={t}>
      Click <a href="/more">here</a> for more
    </Trans>
  );
}

// keyPrefix との組み合わせ
function TransWithKeyPrefix() {
  const { t } = useTranslation("translation", { keyPrefix: "trans" });

  // 実際のキー: trans.simple
  return <Trans i18nKey="simple" t={t} />;
}

// ネストしたスコープでの Trans
function NestedScopeWithTrans() {
  const { t } = useTranslation("translation", { keyPrefix: "common" });

  function Inner() {
    // 内側で同じ名前 t を再定義（シャドーイング）
    const { t } = useTranslation("translation", { keyPrefix: "trans" });

    return (
      <div>
        {/* t 属性あり: 内側の t を使用 → trans.simple */}
        <Trans i18nKey="simple" t={t} />

        {/* t 属性なし: デフォルトのスコープ（外側の t）を使用 → common.hello */}
        <Trans i18nKey="trans.simple" />
      </div>
    );
  }

  return (
    <div>
      <h1>{t("hello")}</h1>
      <Inner />
    </div>
  );
}

export {
  BasicTrans,
  TransWithoutT,
  TransWithComponents,
  TransWithValues,
  TransWithChildren,
  TransWithKeyPrefix,
  NestedScopeWithTrans,
};
