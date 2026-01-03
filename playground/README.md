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
