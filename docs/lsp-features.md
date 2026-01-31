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

Open translation file and position cursor at the key's value. If the key doesn't exist, it will be inserted.

```typescript
arguments: [lang: string, key: string]
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

### `i18n.setCurrentLanguage`

Set the display language for hover, completion, and code actions.

```typescript
arguments: [{ language?: string }]  // null to reset
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
