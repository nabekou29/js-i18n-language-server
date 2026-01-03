/**
 * getFixedT の様々なパターン
 *
 * getFixedT(lng, ns?, keyPrefix?)
 * - lng: 言語コード (null でカレント言語)
 * - ns: 名前空間 (オプション)
 * - keyPrefix: キープレフィックス (オプション)
 */
import i18n from "i18next";

// 基本的な使い方 (言語のみ指定)
function basicUsage() {
  const t = i18n.getFixedT("en");

  console.log(t("common.hello")); // Hello
  console.log(t("common.goodbye")); // Goodbye
}

// 言語を null (カレント言語を使用)
function withCurrentLanguage() {
  const t = i18n.getFixedT(null);

  console.log(t("common.hello"));
  console.log(t("buttons.save"));
}

// 名前空間を指定
function withNamespace() {
  const t = i18n.getFixedT("en", "translation");

  console.log(t("common.hello"));
  console.log(t("messages.success"));
}

// 名前空間を null (デフォルト名前空間を使用)
function withNullNamespace() {
  const t = i18n.getFixedT(null, null);

  console.log(t("common.hello"));
}

// keyPrefix を指定
function withKeyPrefix() {
  const t = i18n.getFixedT(null, "translation", "common");

  // 実際のキー: common.hello, common.goodbye
  console.log(t("hello"));
  console.log(t("goodbye"));
}

// keyPrefix でネストしたキーを指定
function withNestedKeyPrefix() {
  const t = i18n.getFixedT(null, "translation", "user.profile");

  // 実際のキー: user.profile.name, user.profile.email
  console.log(t("name"));
  console.log(t("email"));
}

// ボタン用の翻訳関数
function buttonTranslations() {
  const t = i18n.getFixedT("en", null, "buttons");

  // 実際のキー: buttons.save, buttons.cancel, buttons.submit
  console.log(t("save"));
  console.log(t("cancel"));
  console.log(t("submit"));
}

// メッセージ用の翻訳関数
function messageTranslations() {
  const t = i18n.getFixedT("ja", "translation", "messages");

  // 実際のキー: messages.success, messages.error
  console.log(t("success")); // 操作が成功しました
  console.log(t("error")); // エラーが発生しました
}

// メンバー式でのアクセス
function withMemberExpression() {
  const t = i18n.getFixedT(null, null, "user.settings");

  // 実際のキー: user.settings.language, user.settings.theme
  console.log(t("language"));
  console.log(t("theme"));
}

// 複数の getFixedT を使用
function multipleGetFixedT() {
  const tCommon = i18n.getFixedT(null, null, "common");
  const tButtons = i18n.getFixedT(null, null, "buttons");
  const tMessages = i18n.getFixedT(null, null, "messages");

  console.log(tCommon("hello")); // common.hello
  console.log(tButtons("save")); // buttons.save
  console.log(tMessages("loading")); // messages.loading
}

// クラス内での使用
class TranslationService {
  private t: typeof i18n.t;

  constructor() {
    this.t = i18n.getFixedT(null, "translation", "messages");
  }

  getSuccessMessage(): string {
    return this.t("success");
  }

  getErrorMessage(): string {
    return this.t("error");
  }
}

export {
  basicUsage,
  withCurrentLanguage,
  withNamespace,
  withNullNamespace,
  withKeyPrefix,
  withNestedKeyPrefix,
  buttonTranslations,
  messageTranslations,
  withMemberExpression,
  multipleGetFixedT,
  TranslationService,
};
