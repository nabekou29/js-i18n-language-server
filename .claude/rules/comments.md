# Code Comment Policy

## Core Principle

> **Document at the right layer:**
> - Code → How (implementation)
> - Tests → What (expected behavior)
> - Commits → Why (change reason)
> - Comments → **Why not** (non-obvious decisions)

## Language

**All comments must be in English** for international OSS.

## When to Comment

### ✅ DO Comment

1. **Why not** - Explain non-obvious decisions
   ```rust
   // Use integer arithmetic to avoid floating-point imprecision
   let num_threads = (cpu_count * 2) / 5;
   ```

2. **External references** - Link to specs, RFCs, other implementations
   ```rust
   // Based on ccls approach: 40% of CPU cores for LSP servers
   ```

3. **Edge cases and gotchas**
   ```rust
   // Empty namespace means "search all translation files" (backward compatibility)
   ```

4. **Public API documentation** - For library users
   ```rust
   /// Generate completion items for translation keys.
   ///
   /// # Arguments
   /// * `translations` - All translation data
   ///
   /// # Returns
   /// List of completion items sorted by relevance
   pub fn generate_completions(...) -> Vec<CompletionItem>
   ```

5. **Complex algorithms** - High-level explanation of approach
   ```rust
   // Two-phase indexing: translations first (enables LSP features early),
   // then source files with parallelism limit
   ```

### ❌ DON'T Comment

1. **Self-evident from naming**
   ```rust
   // Bad: "Create a new indexer"
   pub fn new() -> Self

   // Bad: "Check if indexing is completed"
   pub fn is_indexing_completed(&self) -> bool

   // Bad: "Error message"
   pub message: String
   ```

2. **Restating the code**
   ```rust
   // Bad: "Return true if name is 't'"
   if trans_fn_name == "t" { return true; }
   ```

3. **Obvious struct fields**
   ```rust
   // Bad
   pub struct Config {
       /// The name
       pub name: String,
       /// The path
       pub path: PathBuf,
   }

   // Good (no comments needed - names are clear)
   pub struct Config {
       pub name: String,
       pub path: PathBuf,
   }
   ```

4. **Type information already in signature**
   ```rust
   // Bad: "Returns Option<String>"
   fn get_name() -> Option<String>
   ```

## Module Documentation (`//!`)

Keep module docs brief - one line describing purpose:

```rust
//! Completion provider for translation keys
```

Avoid verbose descriptions that repeat what the code shows.

## Doc Comments (`///`) Guidelines

### Public Items

Required for:
- Public functions with non-obvious behavior
- Public types with usage patterns
- Public constants with specific values

Optional for:
- Simple getters/setters
- Obvious constructors (`new`, `default`)
- Single-field wrappers

### Format

```rust
/// Brief description (one line).
///
/// Extended description if needed (optional).
///
/// # Arguments (only if non-obvious)
/// * `param` - Description
///
/// # Returns (only if non-obvious)
///
/// # Errors (required if Result)
///
/// # Panics (required if can panic)
///
/// # Examples (for complex APIs)
```

## Inline Comments (`//`)

- Place on line above, not at end of line
- Use sparingly - prefer clear code over comments
- Focus on "why" not "what"

```rust
// Good: explains why
// Acquire lock before state check to prevent race condition
let guard = self.lock.lock();

// Bad: explains what (obvious from code)
// Lock the mutex
let guard = self.lock.lock();
```

## Section Headers

Use sparingly for large files:

```rust
// === Public API ===

// === Internal Helpers ===
```

Avoid in small files or when structure is obvious.

## TODO/FIXME

Include issue reference when possible:

```rust
// TODO(#123): Support nested namespaces
// FIXME: Handle Unicode normalization
```
