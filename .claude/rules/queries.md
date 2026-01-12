---
paths: "queries/**/*.scm"
---

# Tree-sitter Query Rules

## Structure

```
queries/
  javascript/   # JS-specific queries
  typescript/   # TS-specific queries (no JSX)
  tsx/          # TSX-specific queries (with JSX)
```

## Capture Naming

All captures use `@i18n.` prefix:

| Capture | Purpose |
|---------|---------|
| `@i18n.get_trans_fn` | Translation function definition scope |
| `@i18n.get_trans_fn_name` | Variable name (e.g., `t`) |
| `@i18n.call_trans_fn` | Translation call scope |
| `@i18n.trans_key` | Key string content |
| `@i18n.namespace` | Single namespace |
| `@i18n.namespace_item` | Array namespace element |
| `@i18n.explicit_namespace` | `ns` option value |
| `@i18n.trans_key_prefix` | `keyPrefix` option |

## Pattern Tips

Use `[]` alternatives with different captures for variant handling:

```scheme
[
  (string (string_fragment) @i18n.namespace)
  (array (string (string_fragment) @i18n.namespace_item))
]?
```

## Sync Requirement

Keep JS/TS/TSX queries synchronized when modifying patterns.
