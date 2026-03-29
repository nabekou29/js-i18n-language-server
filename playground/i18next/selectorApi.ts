/**
 * Selector API の様々なパターン (i18next v25.4.0+)
 *
 * t($ => $.key) — アロー関数でキーを指定する型安全な API
 */
import i18n from "i18next";

// 基本的な使い方
function basicSelector() {
  const t = i18n.t;

  console.log(t(($) => $.common.hello));
  console.log(t(($) => $.common.goodbye));
}

// 括弧なしのパラメータ
function withoutParens() {
  const t = i18n.t;

  console.log(t($ => $.common.hello));
  console.log(t($ => $.buttons.save));
}

// ネストしたキー
function nestedKeys() {
  const t = i18n.t;

  console.log(t(($) => $.user.profile.name));
  console.log(t(($) => $.user.profile.email));
  console.log(t(($) => $.user.settings.language));
  console.log(t(($) => $.user.settings.theme));
}

// 補間パラメータとの併用
function withInterpolation() {
  const t = i18n.t;

  console.log(t(($) => $.common.welcome, { name: "World" }));
}

// getFixedT + Selector API
function withGetFixedT() {
  const t = i18n.getFixedT(null, "translation");

  console.log(t(($) => $.common.hello));
  console.log(t(($) => $.buttons.save));
}

// getFixedT + keyPrefix + Selector API
function withGetFixedTAndKeyPrefix() {
  const t = i18n.getFixedT(null, "translation", "common");

  // 実際のキー: common.hello, common.goodbye
  console.log(t(($) => $.hello));
  console.log(t(($) => $.goodbye));
}

// getFixedT + keyPrefix (ネストしたキー)
function withNestedKeyPrefix() {
  const t = i18n.getFixedT(null, "translation", "user.profile");

  // 実際のキー: user.profile.name, user.profile.email
  console.log(t(($) => $.name));
  console.log(t(($) => $.email));
}

// 文字列キーとの混在
function mixedWithStringKeys() {
  const t = i18n.t;

  // 文字列キー
  console.log(t("common.hello"));
  // Selector API
  console.log(t(($) => $.common.goodbye));
  // 文字列キー
  console.log(t("buttons.save"));
  // Selector API
  console.log(t(($) => $.buttons.cancel));
}

export {
  basicSelector,
  withoutParens,
  nestedKeys,
  withInterpolation,
  withGetFixedT,
  withGetFixedTAndKeyPrefix,
  withNestedKeyPrefix,
  mixedWithStringKeys,
};
