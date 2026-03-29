# Supported Syntax

## Supported Libraries

- [i18next](https://www.i18next.com/)
- [react-i18next](https://react.i18next.com/)
- [next-intl](https://next-intl-docs.vercel.app/)
- [svelte-i18n](https://github.com/kaisermann/svelte-i18n)
- [vue-i18n](https://vue-i18n.intlify.dev/)

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

// Selector API (i18next v25.4.0+)
t(($) => $.key)
t(($) => $.nested.key.path)
t(($) => $.key, { count: 1 })
t($ => $.key)  // Without parens
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
<Trans i18nKey={($) => $.key} />  // Selector API

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

// Selector API with keyPrefix
const { t } = useTranslation("ns", { keyPrefix: "form.fields" })
t(($) => $.name)  // → "form.fields.name"
```

## svelte-i18n

Svelte store-based translation functions. Works in `.svelte` files (both `<script>` and template) and plain `.js`/`.ts` files.

```svelte
<script>
  import { _, t, format, json } from 'svelte-i18n'

  // String form
  $_('key')
  $t('key')
  $format('key')
  $json('key')

  // With values
  $_('key', { values: { name: 'World' } })

  // Object form
  $_({ id: 'key' })
  $_({ id: 'key', values: { name: 'Alice' }, locale: 'en', default: 'Fallback' })
</script>

<!-- Template expressions -->
<p>{$_('key')}</p>
<p>{condition ? $_('a') : $_('b')}</p>
<button onclick={() => alert($_('msg'))}>Click</button>
{@const label = $_('key')}
```

### Non-component usage (plain JS/TS files)

```typescript
import { _, format, unwrapFunctionStore, defineMessages } from 'svelte-i18n'

// unwrapFunctionStore: use translation functions outside Svelte components
const $format = unwrapFunctionStore(format)
$format('key')

const translate = unwrapFunctionStore(_)
translate('key')

// defineMessages: statically define translation keys
const messages = defineMessages({
  greeting: { id: 'greeting' },
  farewell: { id: 'farewell' },
})
```

## vue-i18n

Vue.js i18n library support. Works in `.vue` files (both `<script>` and `<template>`) and plain `.js`/`.ts` files. Targets v9+ (Vue 3 Composition API) with v8 (Legacy API) compatibility.

Also covers `petite-vue-i18n` and `@nuxtjs/i18n` (same API).

### Composition API

```vue
<script setup>
import { useI18n } from 'vue-i18n'

// Destructuring
const { t, te, tm } = useI18n()
t('key')
te('key')  // Key existence check
tm('key')  // Raw message object

// Object access pattern
const i18n = useI18n()
i18n.t('key')
</script>
```

### Options API

```vue
<script>
export default {
  computed: {
    title() { return this.$t('greeting') },
    exists() { return this.$te('optional') }
  }
}
</script>
```

### Template Patterns

```vue
<template>
  <!-- Mustache interpolation -->
  {{ $t('message.hello') }}

  <!-- Attribute binding -->
  <input :placeholder="$t('form.placeholder')" />

  <!-- Directives -->
  <span v-if="$te('optional')">{{ $t('optional') }}</span>
  <span v-show="$te('visible')">{{ $t('visible') }}</span>

  <!-- Translation component (v9+) -->
  <i18n-t keypath="terms" tag="p">
    <template #link><a href="/tos">{{ $t('tos') }}</a></template>
  </i18n-t>

  <!-- Translation component (v8) -->
  <i18n path="terms" tag="p">
    <a slot="link" href="/tos">{{ $t('tos') }}</a>
  </i18n>

  <!-- v-t directive -->
  <p v-t="'message.hello'"></p>
  <p v-t="{ path: 'message.hello', args: { name: userName } }"></p>
</template>
```

### Global Functions

| Function | Purpose |
|----------|---------|
| `$t(key)` | Translate |
| `$tc(key, count)` | Pluralize (deprecated v10) |
| `$te(key)` | Key existence check |
| `$tm(key)` | Raw message object |

## File Types

| Extension | tree-sitter Parser | Notes |
|-----------|-------------------|-------|
| `.js` | JavaScript | |
| `.jsx` | JavaScript (with JSX) | |
| `.ts` | TypeScript | |
| `.tsx` | TSX | |
| `.svelte` | TypeScript (extracted) | `<script>` blocks and template expressions are extracted and parsed as TypeScript |
| `.vue` | TypeScript (extracted) | `<script>` blocks and template expressions are extracted and parsed as TypeScript |
