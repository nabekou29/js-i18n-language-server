# vue-i18n

Support for [vue-i18n](https://vue-i18n.intlify.dev/) (v9+ / Vue 3). Also covers [petite-vue-i18n](https://github.com/intlify/vue-i18n/tree/master/packages/petite-vue-i18n) and [@nuxtjs/i18n](https://i18n.nuxtjs.org/) (same API).

## Feature Support

| Feature | Status | Note |
|---------|--------|------|
| `$t()` / `$tc()` / `$te()` / `$tm()` | ✅ | Global functions in templates and Options API |
| `useI18n()` destructuring | ✅ | `const { t, te, tm } = useI18n()` (`tc` is not supported) |
| `useI18n()` object access | ✅ | `const i18n = useI18n(); i18n.t()` |
| `this.$t()` (Options API) | ✅ | |
| Template interpolation | ✅ | `{{ $t('key') }}` |
| Attribute binding | ✅ | `:placeholder="$t('key')"` |
| `v-t` directive | ✅ | String and object syntax |
| `<i18n-t>` component | ✅ | v9+ (`keypath` attribute) |
| `<i18n>` component | ✅ | v8 legacy (`path` attribute) |
| `$d()` / `$n()` | ➖ | Date/number formatting (not translation keys) |
| Plural (pipe-separated) | ✅ | Embedded in translation values |

## Supported Patterns

### Composition API

```vue
<script setup>
import { useI18n } from 'vue-i18n'

// Destructuring
const { t, te, tm } = useI18n()
t('key')
te('key')   // Key existence check
tm('key')   // Raw message object

// Object access
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

  <!-- Event handlers -->
  <button @click="alert($t('message.hello'))">
    {{ $t('form.submit') }}
  </button>
</template>
```

### v-t Directive

```vue
<!-- String syntax -->
<p v-t="'message.hello'"></p>

<!-- Object syntax -->
<p v-t="{ path: 'message.hello', args: { name: userName } }"></p>
```

### Translation Components

```vue
<!-- i18n-t (v9+) — also supports PascalCase: <I18nT> -->
<i18n-t keypath="terms" tag="p">
  <template #link><a href="/tos">{{ $t('tos') }}</a></template>
</i18n-t>

<!-- i18n (v8 legacy) -->
<i18n path="terms" tag="p">
  <a slot="link" href="/tos">{{ $t('tos') }}</a>
</i18n>
```

### Global Functions

| Function | Purpose |
|----------|---------|
| `$t(key)` | Translate |
| `$tc(key, count)` | Pluralize (deprecated in v10) |
| `$te(key)` | Key existence check |
| `$tm(key)` | Raw message object |

## Plural Handling

vue-i18n plurals are embedded in translation values, not in keys (no suffix-based keys like `_one`, `_other`).

```json
{
  "items": "no items | one item | {count} items"
}
```

## Supported File Types

| Extension | Notes |
|-----------|-------|
| `.vue` | `<script>`, `<script setup>`, and `<template>` |
| `.js` | Composition API (`useI18n`) and global functions |
| `.ts` | Composition API (`useI18n`) and global functions |
