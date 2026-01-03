/**
 * useTranslation の様々なパターン
 */
import { useTranslation } from "react-i18next";

// 基本的な使い方
function BasicUsage() {
  const { t } = useTranslation();

  return (
    <div>
      <h1>{t("common.hello")}</h1>
      <p>{t("common.goodbye")}</p>
    </div>
  );
}

// namespace を指定
function WithNamespace() {
  const { t } = useTranslation("translation");

  return <h1>{t("common.hello")}</h1>;
}

// カスタム変数名
function CustomVariableName() {
  const { t: translate } = useTranslation();

  return <h1>{translate("common.hello")}</h1>;
}

// 変数を含む翻訳
function WithInterpolation() {
  const { t } = useTranslation();

  return <h1>{t("common.welcome", { name: "John" })}</h1>;
}

// ネストしたキー
function NestedKeys() {
  const { t } = useTranslation();

  return (
    <div>
      <button>{t("common.buttons.save")}</button>
      <button>{t("common.buttons.cancel")}</button>
    </div>
  );
}

// 複数の useTranslation
function MultipleHooks() {
  const { t: tCommon } = useTranslation();
  const { t: tHome } = useTranslation();

  return (
    <div>
      <h1>{tCommon("common.hello")}</h1>
      <h2>{tHome("home.title")}</h2>
    </div>
  );
}

// 関数スコープ内での使用
function FunctionScope() {
  const { t } = useTranslation();

  const handleClick = () => {
    alert(t("common.hello"));
  };

  return <button onClick={handleClick}>{t("common.buttons.submit")}</button>;
}

// 条件分岐での使用
function ConditionalUsage() {
  const { t } = useTranslation();
  const isLoggedIn = true;

  return (
    <div>
      {isLoggedIn ? (
        <span>{t("common.welcome", { name: "User" })}</span>
      ) : (
        <span>{t("common.hello")}</span>
      )}
    </div>
  );
}

export {
  BasicUsage,
  WithNamespace,
  CustomVariableName,
  WithInterpolation,
  NestedKeys,
  MultipleHooks,
  FunctionScope,
  ConditionalUsage,
};
