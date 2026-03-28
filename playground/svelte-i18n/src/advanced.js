/**
 * svelte-i18n の高度なパターン
 *
 * unwrapFunctionStore: Svelte コンポーネント外で翻訳関数を使う
 * defineMessages: 翻訳キーを静的に定義する
 */
import {
  _,
  format,
  unwrapFunctionStore,
  defineMessages,
} from "svelte-i18n";

// --- unwrapFunctionStore ---
// コンポーネント外では $_ が使えないため、unwrapFunctionStore で関数化する

// 基本パターン: $format という名前で使う
const $format = unwrapFunctionStore(format);
$format("common.hello");
$format("common.goodbye");

// カスタム変数名
const translate = unwrapFunctionStore(_);
translate("home.title");
translate("home.description");

// --- defineMessages ---
// 翻訳キーを一箇所にまとめて定義する

const messages = defineMessages({
  greeting: { id: "common.hello" },
  farewell: { id: "common.goodbye" },
  welcome: { id: "common.welcome" },
});

// 別のメッセージグループ
const navMessages = defineMessages({
  homeTitle: { id: "home.title" },
  homeDesc: { id: "home.description" },
});

export { $format, translate, messages, navMessages };
