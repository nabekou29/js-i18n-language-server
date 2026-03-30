# svelte-i18n

Support for [svelte-i18n](https://github.com/kaisermann/svelte-i18n).

## Feature Support

| Feature | Status | Note |
|---------|--------|------|
| `$_("key")` / `$t("key")` | ✅ | Svelte store syntax |
| `$format("key")` | ✅ | |
| `$json("key")` | ✅ | Returns raw JSON value |
| Object form `$_({ id: "key" })` | ✅ | |
| `unwrapFunctionStore()` | ✅ | For non-component `.js`/`.ts` files |
| `defineMessages()` | ✅ | Static key definitions |
| Svelte template expressions | ✅ | `{$_("key")}` in markup |
| Plural (ICU) | ✅ | Embedded in translation values |

## Supported Patterns

### Store Functions

Four store-based translation functions are recognized:

```svelte
<script>
  import { _, t, format, json } from 'svelte-i18n'

  $_('key')
  $t('key')
  $format('key')
  $json('key')

  // With interpolation values
  $_('key', { values: { name: 'World' } })
</script>
```

### Template Expressions

```svelte
<p>{$_('key')}</p>
<p>{condition ? $_('a') : $_('b')}</p>
<button onclick={() => alert($_('msg'))}>Click</button>
{@const label = $_('key')}
```

### Object Form

The `id` property is extracted as the translation key. Other properties (`values`, `locale`, `default`) are ignored.

```svelte
$_({ id: 'key' })
$_({ id: 'key', values: { name: 'Alice' } })
$_({ id: 'key', locale: 'en' })
$_({ id: 'key', default: 'Fallback text' })
```

### Non-component Usage (plain JS/TS files)

#### unwrapFunctionStore

Use translation functions outside Svelte components.

```typescript
import { _, format, unwrapFunctionStore } from 'svelte-i18n'

const $format = unwrapFunctionStore(format)
$format('key')

const translate = unwrapFunctionStore(_)
translate('key')
```

#### defineMessages

Statically define translation keys. The `id` property of each entry is extracted.

```typescript
import { defineMessages } from 'svelte-i18n'

const messages = defineMessages({
  greeting: { id: 'greeting' },
  farewell: { id: 'farewell' },
})
```

## Plural Handling

svelte-i18n uses **ICU MessageFormat**. Plurals are embedded in translation values, not in keys.

```json
{
  "items": "{count, plural, one {# item} other {# items}}"
}
```

## Supported File Types

| Extension | Notes |
|-----------|-------|
| `.svelte` | `<script>` blocks and template expressions |
| `.js` | Store functions, `unwrapFunctionStore`, `defineMessages` |
| `.ts` | Store functions, `unwrapFunctionStore`, `defineMessages` |
