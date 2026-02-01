# LSP Features

## Standard Methods

| Method | Description |
|--------|-------------|
| `textDocument/completion` | Auto-complete translation keys (triggers: `.`, `"`) |
| `textDocument/hover` | Show translation values for a key |
| `textDocument/definition` | Jump to key definition in JSON file |
| `textDocument/references` | Find all usages of a key |
| `textDocument/codeAction` | Quick fixes for missing translations |
| `textDocument/publishDiagnostics` | Report missing translations and unused keys |

## Custom Commands

### `i18n.editTranslation`

Edit a translation value directly. If the key doesn't exist, it will be inserted.

```typescript
arguments: [{ lang: string, key: string, value: string }]
```

### `i18n.deleteUnusedKeys`

Delete all unused translation keys from a translation file.

```typescript
arguments: [{ uri: string }]

returns: {
  deletedCount: number,
  deletedKeys: string[]
}
```

### `i18n.getKeyAtPosition`

Returns the translation key at the given cursor position.

```typescript
arguments: [{
  uri: string,
  position: { line: number, character: number }
}]

returns: { key: string } | null
```

### `i18n.executeClientEditTranslation`

No-op on the server. Intended to be intercepted by the client to show
a translation edit UI. Triggered via code actions (requires `experimental.i18nEditTranslationCodeAction`).

```typescript
arguments: [{ lang: string, key: string }]
```

### `i18n.getTranslationValue`

Returns the value of a translation key for a given language.

```typescript
arguments: [{ lang: string, key: string }]

returns: { value: string } | null
```

### `i18n.getDecorations`

Returns decoration information for inline translation display.

```typescript
arguments: [{
  uri: string,
  language?: string,
  maxLength?: number
}]

returns: Array<{
  range: Range,
  text: string,
  key: string
}>
```

### `i18n.getCurrentLanguage`

Returns the current display language.

```typescript
arguments: none

returns: { language: string | null }
```

### `i18n.setCurrentLanguage`

Set the display language for hover, completion, and code actions.

```typescript
arguments: [{ language?: string }]  // null to reset
```

## Custom Notifications

### `i18n/decorationsChanged`

Sent by the server when decorations need to be refreshed (e.g., translation changes, language changes).
The client should call `i18n.getDecorations` to get updated decoration data.

```typescript
params: null
```

## Client Capabilities

### `experimental.i18nEditTranslationCodeAction`

When set to `true` in the client's `initialize` params, the server generates
"Add/Edit translation for {lang}" code actions on source files.

The code action triggers `i18n.executeClientEditTranslation` with `{ lang, key }`.
The client should intercept this command, prompt the user for a value,
and then call `i18n.editTranslation` with `{ lang, key, value }`.

```json
{
  "capabilities": {
    "experimental": {
      "i18nEditTranslationCodeAction": true
    }
  }
}
```

## Server Capabilities

Capabilities returned in `initialize` response:

| Capability | Value |
|------------|-------|
| `textDocumentSync` | Full |
| `completionProvider` | Trigger characters: `.`, `"` |
| `hoverProvider` | true |
| `definitionProvider` | true |
| `referencesProvider` | true |
| `codeActionProvider` | true |
| `executeCommandProvider` | `i18n.*` commands |

## File Watching

The server watches for changes to:

- `**/.js-i18n.json` - Configuration file
- Translation files matching `translationFiles.filePattern`
- Source files matching `includePatterns`
