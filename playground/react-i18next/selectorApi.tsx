/**
 * Selector API の様々なパターン (react-i18next + i18next v25.4.0+)
 *
 * t($ => $.key) — アロー関数でキーを指定する型安全な API
 */
import { Trans, useTranslation } from "react-i18next";

// 基本的な使い方
function BasicSelector() {
  const { t } = useTranslation();

  return (
    <div>
      <h1>{t(($) => $.common.hello)}</h1>
      <p>{t(($) => $.common.goodbye)}</p>
    </div>
  );
}

// namespace を指定
function WithNamespace() {
  const { t } = useTranslation("translation");

  return <h1>{t(($) => $.common.hello)}</h1>;
}

// keyPrefix との組み合わせ
function WithKeyPrefix() {
  const { t } = useTranslation("translation", { keyPrefix: "common" });

  // 実際のキー: common.hello, common.goodbye
  return (
    <div>
      <h1>{t(($) => $.hello)}</h1>
      <p>{t(($) => $.goodbye)}</p>
    </div>
  );
}

// ネストしたキー
function NestedKeys() {
  const { t } = useTranslation();

  return (
    <div>
      <p>{t(($) => $.user.profile.name)}</p>
      <p>{t(($) => $.user.profile.email)}</p>
      <p>{t(($) => $.user.settings.language)}</p>
    </div>
  );
}

// 補間パラメータとの併用
function WithInterpolation() {
  const { t } = useTranslation();

  return <h1>{t(($) => $.common.welcome, { name: "World" })}</h1>;
}

// 文字列キーとの混在
function MixedWithStringKeys() {
  const { t } = useTranslation();

  return (
    <div>
      {/* 文字列キー */}
      <h1>{t("common.hello")}</h1>
      {/* Selector API */}
      <p>{t(($) => $.common.goodbye)}</p>
      {/* 文字列キー */}
      <button>{t("buttons.save")}</button>
      {/* Selector API */}
      <button>{t(($) => $.buttons.cancel)}</button>
    </div>
  );
}

// Trans コンポーネント + Selector API
function TransWithSelector() {
  const { t } = useTranslation();

  return <Trans i18nKey={($) => $.trans.simple} t={t} />;
}

// Trans コンポーネント (self-closing) + Selector API
function TransSelfClosingWithSelector() {
  const { t } = useTranslation();

  return <Trans i18nKey={($) => $.trans.with_component} t={t} components={{ link: <a href="/more" /> }} />;
}

// Trans コンポーネント (子要素あり) + Selector API
function TransWithChildrenAndSelector() {
  const { t } = useTranslation();

  return (
    <Trans i18nKey={($) => $.trans.with_component} t={t}>
      Click <a href="/more">here</a> for more
    </Trans>
  );
}

// カスタム変数名
function CustomVariableName() {
  const { t: translate } = useTranslation();

  return <h1>{translate(($) => $.common.hello)}</h1>;
}

// 関数スコープ内での使用
function FunctionScope() {
  const { t } = useTranslation();

  const handleClick = () => {
    alert(t(($) => $.common.hello));
  };

  return <button onClick={handleClick}>{t(($) => $.buttons.submit)}</button>;
}

export {
  BasicSelector,
  WithNamespace,
  WithKeyPrefix,
  NestedKeys,
  WithInterpolation,
  MixedWithStringKeys,
  TransWithSelector,
  TransSelfClosingWithSelector,
  TransWithChildrenAndSelector,
  CustomVariableName,
  FunctionScope,
};
