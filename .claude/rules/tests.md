---
paths: "src/**/*.rs"
---

# Testing Rules

## Test Framework

- **rstest** - Parameterized tests with `#[rstest]`
- **googletest** - Assertions with `assert_that!`, `eq()`, `len()`

## In-Source Test Module

Place tests at the bottom of the source file:

```rust
// ... production code above ...

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::HashMap;

    use googletest::prelude::*;
    use rstest::*;

    use super::*;
    use crate::db::I18nDatabaseImpl;
    use crate::test_utils::create_translation;

    #[rstest]
    fn test_function_name() {
        // ...
    }
}
```

### Key Points

1. **Place at file bottom** - After all production code
2. **Allow test-only lints** - `unwrap_used`, `indexing_slicing` are OK in tests
3. **Import order** - std → external crates → super → crate modules
4. **Use `super::*`** - Access all items from parent module

## Test Helpers

Use `test_utils.rs` for common setup:

```rust
use crate::test_utils::create_translation;

let translation = create_translation(
    &db,
    "ja",                           // language
    "/test/locales/ja.json",        // file_path
    HashMap::from([("key", "value")]),
);
```

## Parameterized Tests with rstest

### Basic Cases

```rust
#[rstest]
#[case("input1", "expected1")]
#[case("input2", "expected2")]
fn test_something(#[case] input: &str, #[case] expected: &str) {
    assert_that!(process(input), eq(expected));
}
```

### Named Cases

```rust
#[rstest]
#[case::empty_input("", "")]
#[case::single_char("a", "A")]
#[case::japanese("あ", "あ")]
fn test_uppercase(#[case] input: &str, #[case] expected: &str) {
    // Test names appear in output: test_uppercase::empty_input
}
```

### Fixtures

```rust
#[fixture]
fn db() -> I18nDatabaseImpl {
    I18nDatabaseImpl::default()
}

#[rstest]
fn test_with_db(db: I18nDatabaseImpl) {
    // db is automatically provided
}
```

## Assertions (googletest)

```rust
// Equality
assert_that!(result, eq("expected"));

// Numeric comparison
assert_that!(count, gt(0));
assert_that!(value, ge(10));

// Length
assert_that!(items, len(eq(3)));

// Empty
assert_that!(items, is_empty());

// Contains
assert_that!(text, contains_substring("foo"));

// Option/Result
assert_that!(option, some(eq("value")));
assert_that!(result, ok(eq(42)));

// Collections
assert_that!(vec![1, 2, 3], contains(eq(2)));
assert_that!(vec![1, 2, 3], each(gt(0)));
```

## Test Naming Convention

Use descriptive names that explain what is being tested:

```rust
#[rstest]
fn truncate_value_short_text() { }      // OK: describes scenario
fn truncate_value_exact_length() { }    // OK: describes edge case
fn truncate_value_japanese_text() { }   // OK: describes special case

fn test_truncate() { }                  // Too vague
fn test1() { }                          // Meaningless
```
