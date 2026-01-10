# Testing Rules

Applies to: `**/*test*`, `**/tests/**`

## Test Framework

- **rstest** - Parameterized tests with `#[rstest]`
- **googletest** - Assertions with `assert_that!`, `eq()`, `len()`

## Test Module Setup

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;
    use super::*;
}
```

## Test Helpers

Use `test_utils.rs` for common setup:
- `create_translation()` - Create test Translation instances
- `I18nDatabaseImpl::default()` - Fresh Salsa database

## Parameterized Tests

```rust
#[rstest]
#[case("input1", "expected1")]
#[case("input2", "expected2")]
fn test_something(#[case] input: &str, #[case] expected: &str) {
    assert_that!(process(input), eq(expected));
}
```

## Assertions

```rust
// Equality
assert_that!(result, eq("expected"));

// Length
assert_that!(items, len(eq(3)));

// Empty
assert_that!(items, is_empty());

// Contains
assert_that!(text, contains_substring("foo"));
```
