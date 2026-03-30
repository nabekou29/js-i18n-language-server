# i18next / react-i18next

Support for [i18next](https://www.i18next.com/) and [react-i18next](https://react.i18next.com/).

## Feature Support

| Feature | Status | Note |
|---------|--------|------|
| `t("key")` | ✅ | |
| `i18next.t()` / `i18n.t()` | ✅ | Global function calls |
| `useTranslation()` | ✅ | Namespace, keyPrefix, function rename |
| `getFixedT()` | ✅ | `i18n.getFixedT(lang, ns, keyPrefix)` |
| Namespace | ✅ | Via `useTranslation`, `getFixedT`, `ns` option, config |
| Key Prefix | ✅ | Via `useTranslation`, `getFixedT`, `<Translation>` |
| Selector API | ✅ | `t($ => $.key)` (i18next v25.4.0+) |
| `<Trans>` component | ✅ | String key and Selector API |
| `<Translation>` component | ✅ | With `keyPrefix` prop |
| Plural (suffix-based) | ✅ | `_zero`, `_one`, `_two`, `_few`, `_many`, `_other` |

## Supported Patterns

### Translation Function Calls

```tsx
// Basic
t("key")
t("nested.key.path")

// With options
t("key", { count: 1 })
t("key", { ns: "namespace" })
t("key", { defaultValue: "..." })

// With namespace override
t("key", { ns: "common" })

// Global calls
i18n.t("key")
i18next.t("key")
```

### Selector API

[Selector API](https://www.i18next.com/overview/typescript#selector-api) format (i18next v25.4.0+).

```tsx
t(($) => $.key)
t(($) => $.nested.key.path)
t(($) => $.key, { count: 1 })
t($ => $.key) // Without parens

// With namespace override
t(($) => $.key, { ns: "common" })
```

### Acquiring Translation Functions

#### useTranslation

```tsx
const { t } = useTranslation()
const { t } = useTranslation("namespace")
const { t } = useTranslation(["ns1", "ns2"])
const { t } = useTranslation("namespace", { keyPrefix: "section" })

// Function rename
const { t: customT } = useTranslation()
```

#### getFixedT

```tsx
const t = i18n.getFixedT(null, "namespace")
const t = i18n.getFixedT(null, "namespace", "keyPrefix")
```

### JSX Components

#### Trans

```tsx
<Trans i18nKey="key" />
<Trans i18nKey="key">Fallback content</Trans>
<Trans i18nKey={"key"} />
<Trans i18nKey="key" t={customT} />

// Selector API
<Trans i18nKey={($) => $.key} />
```

#### Translation

```tsx
<Translation>
  {(t) => <span>{t("key")}</span>}
</Translation>
<Translation keyPrefix="section">
  {(t) => <span>{t("key")}</span>}
</Translation>
```

### Namespace Resolution

Namespace is resolved with the following priority (highest first):

| Method | Example |
|--------|---------|
| `ns` option in `t()` | `t("key", { ns: "common" })` |
| `useTranslation()` argument | `useTranslation("common")` |
| `useTranslation()` array | `useTranslation(["common", "app"])` |
| `getFixedT()` argument | `getFixedT(null, "common")` |
| `defaultNamespace` in config | `.js-i18n.json` |

### Key Prefix

Key prefix is automatically prepended to all keys.

```tsx
// useTranslation keyPrefix
const { t } = useTranslation("ns", { keyPrefix: "form.fields" })
t("name")  // -> "form.fields.name"

// getFixedT keyPrefix
const t = i18n.getFixedT(null, "ns", "form.fields")
t("name")  // -> "form.fields.name"

// Translation component keyPrefix
<Translation keyPrefix="form.fields">
  {(t) => t("name")}  {/* -> "form.fields.name" */}
</Translation>

// Selector API with keyPrefix
const { t } = useTranslation("ns", { keyPrefix: "form.fields" })
t(($) => $.name)  // -> "form.fields.name"
```

## Plural Handling

i18next uses **suffix-based** plural keys.

```json
{
  "item_one": "{{count}} item",
  "item_other": "{{count}} items"
}
```

Supported suffixes: `_zero`, `_one`, `_two`, `_few`, `_many`, `_other`

## Supported File Types

| Extension | Notes |
|-----------|-------|
| `.js` | |
| `.jsx` | + `<Trans>`, `<Translation>` components |
| `.ts` | |
| `.tsx` | + `<Trans>`, `<Translation>` components |

## Configuration

### `frameworks.i18next.preferSelector`

`boolean` (default: `false`)

When `true`, completions insert Selector API format instead of string format.

```json
{
  "frameworks": {
    "i18next": {
      "preferSelector": true
    }
  }
}
```

| Value | Completion result |
|-------|-------------------|
| `false` | `t("common.hello")` |
| `true` | `t(($) => $.common.hello)` |
