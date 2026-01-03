/**
 * Translation コンポーネントの様々なパターン
 */
import { Translation } from "react-i18next";

// 基本的な使い方
function BasicTranslation() {
  return (
    <Translation>
      {(t) => (
        <div>
          <h1>{t("common.hello")}</h1>
          <p>{t("common.goodbye")}</p>
        </div>
      )}
    </Translation>
  );
}

// keyPrefix を指定
function TranslationWithKeyPrefix() {
  return (
    <Translation keyPrefix="common">
      {(t) => (
        <div>
          {/* 実際のキー: common.hello, common.goodbye */}
          <h1>{t("hello")}</h1>
          <p>{t("goodbye")}</p>
        </div>
      )}
    </Translation>
  );
}

// ネストした keyPrefix
function TranslationNestedKeyPrefix() {
  return (
    <Translation keyPrefix="common.buttons">
      {(t) => (
        <div>
          {/* 実際のキー: common.buttons.save, common.buttons.cancel */}
          <button>{t("save")}</button>
          <button>{t("cancel")}</button>
        </div>
      )}
    </Translation>
  );
}

// function 形式
function TranslationWithFunction() {
  return (
    <Translation keyPrefix="form.field">
      {function renderForm(t) {
        return (
          <form>
            {/* 実際のキー: form.field.name, form.field.email */}
            <label>{t("name")}</label>
            <input type="text" />

            <label>{t("email")}</label>
            <input type="email" />
          </form>
        );
      }}
    </Translation>
  );
}

// namespace を指定
function TranslationWithNamespace() {
  return (
    <Translation ns="translation" keyPrefix="home">
      {(t) => (
        <div>
          {/* 実際のキー: home.title, home.description */}
          <h1>{t("title")}</h1>
          <p>{t("description")}</p>
        </div>
      )}
    </Translation>
  );
}

// ネストした Translation
function NestedTranslation() {
  return (
    <Translation keyPrefix="common">
      {(tCommon) => (
        <div>
          <h1>{tCommon("hello")}</h1>

          <Translation keyPrefix="form.field">
            {(tForm) => (
              <form>
                {/* tForm: form.field.name */}
                <label>{tForm("name")}</label>

                {/* tCommon: common.buttons.submit */}
                <button>{tCommon("buttons.submit")}</button>
              </form>
            )}
          </Translation>
        </div>
      )}
    </Translation>
  );
}

export {
  BasicTranslation,
  TranslationWithKeyPrefix,
  TranslationNestedKeyPrefix,
  TranslationWithFunction,
  TranslationWithNamespace,
  NestedTranslation,
};
