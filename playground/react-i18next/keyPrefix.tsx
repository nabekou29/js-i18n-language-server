/**
 * keyPrefix オプションの様々なパターン
 */
import { useTranslation } from "react-i18next";

// keyPrefix を指定
function WithKeyPrefix() {
  const { t } = useTranslation("translation", { keyPrefix: "common" });

  // 実際のキー: common.hello, common.goodbye
  return (
    <div>
      <h1>{t("hello")}</h1>
      <p>{t("goodbye")}</p>
    </div>
  );
}

// keyPrefix + ネストしたキー
function NestedWithKeyPrefix() {
  const { t } = useTranslation("translation", { keyPrefix: "common.buttons" });

  // 実際のキー: common.buttons.save, common.buttons.cancel
  return (
    <div>
      <button>{t("save")}</button>
      <button>{t("cancel")}</button>
    </div>
  );
}

// フォーム用のコンポーネント
function FormFields() {
  const { t } = useTranslation("translation", { keyPrefix: "form.field" });

  // 実際のキー: form.field.name, form.field.email, form.field.password
  return (
    <form>
      <label>{t("name")}</label>
      <input type="text" />

      <label>{t("email")}</label>
      <input type="email" />

      <label>{t("password")}</label>
      <input type="password" />
    </form>
  );
}

// バリデーションメッセージ
function ValidationMessages() {
  const { t } = useTranslation("translation", { keyPrefix: "form.validation" });

  // 実際のキー: form.validation.required, form.validation.invalid_email
  return (
    <div>
      <span className="error">{t("required")}</span>
      <span className="error">{t("invalid_email")}</span>
    </div>
  );
}

// keyPrefix なしと混在
function MixedUsage() {
  const { t: tWithPrefix } = useTranslation("translation", {
    keyPrefix: "common",
  });
  const { t: tWithoutPrefix } = useTranslation();

  return (
    <div>
      {/* keyPrefix あり: common.hello */}
      <h1>{tWithPrefix("hello")}</h1>

      {/* keyPrefix なし: home.title */}
      <h2>{tWithoutPrefix("home.title")}</h2>
    </div>
  );
}

// ネストしたコンポーネントでの keyPrefix
function ParentComponent() {
  const { t } = useTranslation("translation", { keyPrefix: "common" });

  function ChildComponent() {
    const { t: tChild } = useTranslation("translation", {
      keyPrefix: "form.field",
    });

    // 子: form.field.name
    return <label>{tChild("name")}</label>;
  }

  return (
    <div>
      {/* 親: common.hello */}
      <h1>{t("hello")}</h1>
      <ChildComponent />
    </div>
  );
}

export {
  WithKeyPrefix,
  NestedWithKeyPrefix,
  FormFields,
  ValidationMessages,
  MixedUsage,
  ParentComponent,
};
