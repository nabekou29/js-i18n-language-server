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
  "primaryLanguages": null,
  "diagnostics": {
    "missingTranslation": {
      "enabled": true,
      "severity": "warning",
      "requiredLanguages": null,
      "optionalLanguages": null
    },
    "unusedTranslation": {
      "enabled": true,
      "severity": "hint",
      "ignorePatterns": []
    }
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

## primaryLanguages

`string[]?` (default: `null`)

Fallback priority for display (hover, virtual text). The first available language is used.

---

## diagnostics.missingTranslation

Configuration for missing translation key diagnostics.

### diagnostics.missingTranslation.enabled

`boolean` (default: `true`)

Enable or disable missing translation diagnostics.

### diagnostics.missingTranslation.severity

`"error" | "warning" | "information" | "hint"` (default: `"warning"`)

Severity level for missing translation diagnostics.

### diagnostics.missingTranslation.requiredLanguages

`string[]?` (default: `null`)

Languages that must have translations. If `null`, all detected languages are required.

Mutually exclusive with `optionalLanguages`.

### diagnostics.missingTranslation.optionalLanguages

`string[]?` (default: `null`)

Languages where missing translations are ignored (no diagnostics).

Mutually exclusive with `requiredLanguages`.

---

## diagnostics.unusedTranslation

Configuration for unused translation key diagnostics.

### diagnostics.unusedTranslation.enabled

`boolean` (default: `true`)

Enable or disable unused translation diagnostics.

### diagnostics.unusedTranslation.severity

`"error" | "warning" | "information" | "hint"` (default: `"hint"`)

Severity level for unused translation diagnostics.

### diagnostics.unusedTranslation.ignorePatterns

`string[]` (default: `[]`)

Glob patterns for translation keys to exclude from unused key detection.

Example:
```json
{
  "diagnostics": {
    "unusedTranslation": {
      "ignorePatterns": ["debug.*", "internal.**"]
    }
  }
}
```

---

## indexing.numThreads

`number?` (default: 40% of CPU cores)

Number of parallel threads for workspace indexing.
