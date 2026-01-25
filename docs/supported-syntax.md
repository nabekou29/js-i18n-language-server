# Supported Syntax

## Supported Libraries

- [i18next](https://www.i18next.com/)
- [react-i18next](https://react.i18next.com/)
- [next-intl](https://next-intl-docs.vercel.app/)

## Translation Function Calls

The `t()` function is the primary detection target.

```tsx
// Basic
t("key")
t("nested.key.path")

// With options (any options are allowed)
t("key", { count: 1 })
t("key", { ns: "namespace" })
t("key", { defaultValue: "..." })
t("key", { interpolation: { escapeValue: false }, value: getData() })

// Global calls
i18n.t("key")
i18next.t("key")
```

## Translation Function Acquisition

How to obtain the `t` function. Affects namespace and keyPrefix scope.

```tsx
// react-i18next
const { t } = useTranslation()
const { t } = useTranslation("namespace")
const { t } = useTranslation(["ns1", "ns2"])
const { t } = useTranslation("namespace", { keyPrefix: "section" })
const { t: customT } = useTranslation()  // Rename supported

// i18next
const t = i18n.getFixedT(null, "namespace")
const t = i18n.getFixedT(null, "namespace", "keyPrefix")

// next-intl
const t = useTranslations()
const t = useTranslations("namespace")
```

## JSX Components

react-i18next JSX component patterns.

```tsx
// Trans component
<Trans i18nKey="key" />
<Trans i18nKey="key">Fallback content</Trans>
<Trans i18nKey={"key"} />  // JSX expression supported
<Trans i18nKey="key" t={customT} />  // Custom t function

// Translation component (render props)
<Translation>
  {(t) => <span>{t("key")}</span>}
</Translation>
<Translation keyPrefix="section">
  {(t) => <span>{t("key")}</span>}
</Translation>
```

## Namespace Resolution

Namespace specification methods and priority.

| Method | Example | Priority |
|--------|---------|----------|
| `ns` option in `t()` | `t("key", { ns: "common" })` | Highest |
| `useTranslation()` argument | `useTranslation("common")` | High |
| `useTranslation()` array | `useTranslation(["common", "app"])` | High |
| `getFixedT()` argument | `getFixedT(null, "common")` | High |
| `defaultNamespace` in config | `.js-i18n.json` | Low |

## Key Prefix

Automatically prepend a prefix to keys.

```tsx
// useTranslation keyPrefix
const { t } = useTranslation("ns", { keyPrefix: "form.fields" })
t("name")  // → "form.fields.name"

// getFixedT keyPrefix
const t = i18n.getFixedT(null, "ns", "form.fields")
t("name")  // → "form.fields.name"

// Translation component keyPrefix
<Translation keyPrefix="form.fields">
  {(t) => t("name")}  {/* → "form.fields.name" */}
</Translation>
```

## File Types

| Extension | tree-sitter Parser |
|-----------|-------------------|
| `.js` | JavaScript |
| `.jsx` | JavaScript (with JSX) |
| `.ts` | TypeScript |
| `.tsx` | TSX |
