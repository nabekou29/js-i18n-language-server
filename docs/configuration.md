# Configuration Reference

Configuration can be provided via:
- `.js-i18n.json` file in project root
- LSP `initializationOptions`
- LSP `workspace/didChangeConfiguration`

## Default Configuration

```json
{
  "translationFiles": {
    "filePattern": "**/{locales,messages}/**/*.json"
  },
  "includePatterns": ["**/*.{js,jsx,ts,tsx}"],
  "excludePatterns": ["node_modules/**"],
  "keySeparator": ".",
  "namespaceSeparator": null,
  "defaultNamespace": null,
  "requiredLanguages": null,
  "optionalLanguages": null,
  "primaryLanguages": null,
  "diagnostics": {
    "unusedKeys": true
  },
  "virtualText": {
    "maxLength": 30
  },
  "indexing": {
    "numThreads": null
  }
}
```

---

## translationFiles.filePattern

`string` (default: `"**/{locales,messages}/**/*.json"`)

Glob pattern to find translation JSON files.

---

## includePatterns

`string[]` (default: `["**/*.{js,jsx,ts,tsx}"]`)

Glob patterns for source files to analyze.

---

## excludePatterns

`string[]` (default: `["node_modules/**"]`)

Glob patterns to exclude from analysis.

---

## keySeparator

`string` (default: `"."`)

Separator for nested keys.

Example:
```
"user.profile.name" → { "user": { "profile": { "name": "..." } } }
```

---

## namespaceSeparator

`string?` (default: `null`)

Separator for namespaces. Set to `":"` for i18next-style namespaces.

Example:
```
"common:button.save" → namespace "common", key "button.save"
```

---

## defaultNamespace

`string?` (default: `null`)

Default namespace when not specified in code.

---

## requiredLanguages

`string[]?` (default: `null`)

Languages that must have translations. If `null`, all detected languages are required.

Mutually exclusive with `optionalLanguages`.

---

## optionalLanguages

`string[]?` (default: `null`)

Languages where missing translations are ignored (no diagnostics).

Mutually exclusive with `requiredLanguages`.

---

## primaryLanguages

`string[]?` (default: `null`)

Fallback priority for display (hover, virtual text). The first available language is used.

---

## diagnostics.unusedKeys

`boolean` (default: `true`)

Report unused translation keys in JSON files.

---

## virtualText.maxLength

`number` (default: `30`)

Max characters for inline virtual text display.

---

## indexing.numThreads

`number?` (default: 40% of CPU cores)

Number of parallel threads for workspace indexing.
