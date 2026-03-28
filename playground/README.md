# Playground

Language Server の動作確認用のサンプルプロジェクト集です。

## 構造

```
playground/
├── react-i18next/                  # react-i18next のパターン
│   ├── locales/
│   │   ├── en/translation.json
│   │   └── ja/translation.json
│   ├── useTranslation.tsx          # useTranslation の様々なパターン
│   ├── keyPrefix.tsx               # keyPrefix オプションのパターン
│   ├── Trans.tsx                   # Trans コンポーネントのパターン
│   └── Translation.tsx             # Translation コンポーネントのパターン
│
├── i18next/                        # i18next (getFixedT) のパターン
│   ├── locales/
│   │   ├── en/translation.json
│   │   └── ja/translation.json
│   └── getFixedT.ts                # getFixedT の様々なパターン
│
├── next-intl/                      # next-intl のパターン
│   ├── messages/
│   │   ├── en.json
│   │   └── ja.json
│   ├── useTranslations.tsx         # useTranslations の様々なパターン
│   └── richText.tsx                # t.rich, t.markup, t.raw のパターン
│
├── svelte-i18n/                    # svelte-i18n のパターン (実動 Vite プロジェクト)
│   ├── .js-i18n.json               # LSP 設定
│   ├── package.json                # svelte, svelte-i18n, vite
│   ├── vite.config.js
│   ├── index.html
│   ├── locales/
│   │   ├── en.json
│   │   └── ja.json
│   └── src/
│       ├── i18n.js                 # svelte-i18n 初期化
│       ├── main.js                 # アプリ起動
│       ├── App.svelte              # ルートコンポーネント
│       ├── BasicUsage.svelte       # $_, $t, $format 基本パターン
│       ├── FormatVariants.svelte   # $format, $json, $date, $number, $time
│       ├── ObjectForm.svelte       # $_({ id, values, locale, default })
│       ├── Interpolation.svelte    # 補間、plural、select (ICU MessageFormat)
│       ├── NestedKeys.svelte       # ドット記法のネストキー
│       └── ReactiveUsage.svelte    # $derived, 条件分岐, ループ
│
└── react-i18next-custom-settings/  # カスタム設定のパターン
    ├── .js-i18n.json               # keySeparator: "_" の設定
    ├── locales/
    │   ├── en/translation.json
    │   └── ja/translation.json
    ├── custom-key-separator.tsx    # カスタム key_separator のサンプル
    └── namespace-separator.tsx     # namespace_separator のサンプル（将来用）
```

## 対応ライブラリとパターン

### react-i18next

| パターン | ファイル | 説明 |
|----------|----------|------|
| `useTranslation()` | useTranslation.tsx | 基本的な使い方 |
| `useTranslation("ns")` | useTranslation.tsx | namespace 指定 |
| `{ t: customName }` | useTranslation.tsx | カスタム変数名 |
| `t("key", { var })` | useTranslation.tsx | 変数埋め込み |
| `keyPrefix: "prefix"` | keyPrefix.tsx | キープレフィックス |
| `<Trans i18nKey="..." t={t} />` | Trans.tsx | Trans (t 属性あり) |
| `<Trans i18nKey="..." />` | Trans.tsx | Trans (t 属性なし = 最外スコープ) |
| `<Translation>{(t) => ...}</Translation>` | Translation.tsx | Translation コンポーネント |
| `<Translation keyPrefix="...">` | Translation.tsx | Translation + keyPrefix |

### i18next

| パターン | ファイル | 説明 |
|----------|----------|------|
| `getFixedT("en")` | getFixedT.ts | 言語指定 |
| `getFixedT(null)` | getFixedT.ts | カレント言語 |
| `getFixedT(null, "ns")` | getFixedT.ts | namespace 指定 |
| `getFixedT(null, null, "prefix")` | getFixedT.ts | keyPrefix 指定 |
| `i18n.getFixedT(...)` | getFixedT.ts | メンバー式でのアクセス |

### next-intl

| パターン | ファイル | 説明 |
|----------|----------|------|
| `useTranslations()` | useTranslations.tsx | namespace なし |
| `useTranslations("ns")` | useTranslations.tsx | namespace 指定 (= keyPrefix) |
| `t("key")` | useTranslations.tsx | 通常の翻訳 |
| `t.rich("key", { tag: ... })` | richText.tsx | リッチテキスト |
| `t.markup("key", { tag: ... })` | richText.tsx | マークアップ |
| `t.raw("key")` | richText.tsx | 生テキスト |

### svelte-i18n

**注**: このプロジェクトは実際に動作する Vite + Svelte アプリです。
`cd playground/svelte-i18n && npm install && npm run dev` で起動できます。

| パターン | ファイル | 説明 |
|----------|----------|------|
| `$_('key')` | BasicUsage.svelte | 基本的な翻訳 (テンプレート & script) |
| `$t('key')` | BasicUsage.svelte | `$_` のエイリアス |
| `$format('key')` | FormatVariants.svelte | 明示的フォーマッター名 |
| `$json('key')` | FormatVariants.svelte | JSON 生値の取得 |
| `$date(date, opts)` | FormatVariants.svelte | 日付フォーマット (キーなし) |
| `$number(num, opts)` | FormatVariants.svelte | 数値フォーマット (キーなし) |
| `$time(date, opts)` | FormatVariants.svelte | 時刻フォーマット (キーなし) |
| `$_({ id: 'key', ... })` | ObjectForm.svelte | オブジェクト形式 |
| `$_('key', { values })` | Interpolation.svelte | 変数埋め込み |
| `{count, plural, ...}` | Interpolation.svelte | ICU 複数形 (翻訳ファイル内) |
| `{gender, select, ...}` | Interpolation.svelte | ICU 選択 (翻訳ファイル内) |
| `$_('a.b.c')` | NestedKeys.svelte | ネストキー |
| `$derived($_('key'))` | ReactiveUsage.svelte | リアクティブ宣言 |
| `{#if}{$_('key')}{/if}` | ReactiveUsage.svelte | 条件分岐内での使用 |
| `{#each}{$_('key')}{/each}` | ReactiveUsage.svelte | ループ内での使用 |
| `unwrapFunctionStore(store)` | advanced.js | コンポーネント外での翻訳関数 |
| `defineMessages({...})` | advanced.js | 翻訳キーの静的定義 |

### カスタム設定 (react-i18next-custom-settings)

| パターン | ファイル | 説明 |
|----------|----------|------|
| `keySeparator: "_"` | custom-key-separator.tsx | デフォルトの `.` ではなく `_` を使用 |
| `namespaceSeparator: ":"` | namespace-separator.tsx | 名前空間区切り文字（将来対応予定） |

**注**: このプロジェクトには `.js-i18n.json` 設定ファイルがあり、`keySeparator: "_"` が設定されています。
翻訳キーは `common_greeting_hello` のような形式になります。

## 使い方

1. エディタでファイルを開く
2. Language Server の機能を確認:
   - 補完 (キー入力時)
   - ホバー (翻訳値の表示)
   - 診断 (存在しないキーの警告)
   - 定義へ移動 (JSON ファイルへジャンプ)
