---
paths: "src/**/*.rs"
---

# Testing Rules

## Framework

- **rstest** - `#[rstest]`, `#[case]`, `#[fixture]`
- **googletest** - `assert_that!`, `eq()`, `len()`, `some()`, `none()`

## In-Source Tests

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use googletest::prelude::*;
    use rstest::*;
    use super::*;

    #[rstest]
    fn descriptive_test_name() {
        assert_that!(result, eq("expected"));
    }
}
```

## Key Points

- Place tests at file bottom
- Use `super::*` for parent module access
- Use `crate::test_utils::create_translation` for test data
- Name tests descriptively: `truncate_value_short_text`, not `test1`
