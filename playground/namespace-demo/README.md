# Namespace Demo

i18next の namespace 機能の動作確認用 playground です。

## ディレクトリ構成

```
namespace-demo/
  .js-i18n.json           # LSP 設定
  locales/
    en/
      common.json         # namespace: "common"
      errors.json         # namespace: "errors"
      translation.json    # namespace: "translation"
    ja/
      common.json
      errors.json
      translation.json
  namespace-basic.tsx     # 基本的な namespace 使用
  namespace-separator.tsx # namespace separator (t("ns:key"))
  default-namespace.tsx   # defaultNamespace 設定
```

## 設定 (.js-i18n.json)

```json
{
  "namespaceSeparator": ":",
  "defaultNamespace": "translation"
}
```

## 確認ポイント

### 1. useTranslation("namespace") による補完

```tsx
const { t } = useTranslation("common");
t("greeting.hello")  // → common.json のキーのみ補完
```

### 2. namespace separator (t("ns:key"))

```tsx
const { t } = useTranslation("common");
t("errors:notFound")  // → errors.json から検索
```

### 3. defaultNamespace

```tsx
const { t } = useTranslation();  // namespace 指定なし
t("welcome")  // → translation.json (defaultNamespace) から検索
```

## 期待される動作

| パターン | 検索対象 |
|---------|---------|
| `useTranslation("common")` + `t("key")` | common.json |
| `useTranslation("common")` + `t("errors:key")` | errors.json |
| `useTranslation()` + `t("key")` | translation.json (default) |
| `useTranslation()` + `t("common:key")` | common.json |
