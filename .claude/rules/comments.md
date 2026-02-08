---
paths: "src/**/*.rs"
---

# Code Comment Policy

## Core Principle

> Code → How, Tests → What, Commits → Why, Comments → **Why not** (non-obvious decisions)

## Language

All comments in English (international OSS).

## DO Comment

1. **Why not** — Non-obvious decisions (e.g., `// Use integer arithmetic to avoid floating-point imprecision`)
2. **External references** — Links to specs, RFCs, other implementations
3. **Edge cases and gotchas** — Backward compatibility, surprising behavior
4. **Public API docs** — Functions with non-obvious behavior, types with usage patterns
5. **Complex algorithms** — High-level explanation of approach

## DON'T Comment

1. Self-evident from naming (`new()`, `is_indexing_completed()`)
2. Restating the code (`// Return true if name is 't'`)
3. Obvious struct fields (`/// The name` on `pub name: String`)
4. Type information already in signature (`// Returns Option<String>`)

## Module Docs (`//!`)

One line describing purpose: `//! Completion provider for translation keys`

## Doc Comments (`///`)

Required for public items with non-obvious behavior. Optional for simple getters, obvious constructors, single-field wrappers.

## Inline Comments (`//`)

Place on line above, not at end. Focus on "why" not "what". Use sparingly.

## TODO/FIXME

Include issue reference: `// TODO(#123): Support nested namespaces`
