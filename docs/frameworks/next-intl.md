# next-intl

Support for [next-intl](https://next-intl.dev/).

## Feature Support

| Feature | Status | Note |
|---------|--------|------|
| `t("key")` | ✅ | |
| `t.rich()` | ✅ | Rich text with components |
| `t.markup()` | ✅ | HTML markup tags |
| `t.raw()` | ✅ | Raw unprocessed text |
| `useTranslations()` | ✅ | With optional namespace |
| Namespace | ✅ | Via `useTranslations()` argument (acts as key prefix) |
| `t.has()` | ❌ | Key existence check |
| `getTranslations()` | ❌ | Server Components async API |
| Plural (ICU) | ✅ | Embedded in translation values |

## Supported Patterns

### Translation Function Calls

```tsx
t("key")
t("nested.key.path")
t("key", { name: "World" })
```

### Method Chains

```tsx
// Rich text: embed React components within translations
t.rich("terms", {
  link: (chunks) => <a href="/terms">{chunks}</a>,
  bold: (chunks) => <strong>{chunks}</strong>,
})

// Markup: HTML tag handlers
t.markup("bold", {
  b: (chunks) => `<strong>${chunks}</strong>`,
})

// Raw: unprocessed translation string
t.raw("htmlContent")
```

### Acquiring Translation Functions

```tsx
const t = useTranslations()
const t = useTranslations("namespace")
const t = useTranslations("home.hero") // Nested namespace
```

The `useTranslations()` argument acts as a **key prefix** — all keys are automatically prepended:

```tsx
const t = useTranslations("common")
t("hello")  // -> "common.hello"

const t = useTranslations("home.hero")
t("heading")  // -> "home.hero.heading"
```

## Plural Handling

next-intl uses **ICU MessageFormat**. Plurals are embedded in translation values, not in keys.

```json
{
  "items": "You have {count, plural, one {# item} other {# items}}"
}
```

## Supported File Types

| Extension | Notes |
|-----------|-------|
| `.js` | |
| `.jsx` | |
| `.ts` | |
| `.tsx` | |
