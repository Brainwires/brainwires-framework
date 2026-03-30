# Buggy Calculator - Test Fixture

This is a test fixture containing a simple calculator library with a known bug.

## The Bug

The `divide()` function has the wrong operator - it uses multiplication (`*`) instead of division (`/`).

```rust
// Current (WRONG):
Ok(a * b)

// Should be:
Ok(a / b)
```

This bug also affects the `average()` function since it relies on `divide()`.

## Expected Behavior

- `divide(6.0, 2.0)` should return `3.0`, not `12.0`
- `divide(10.0, 5.0)` should return `2.0`, not `50.0`
- `average([2.0, 4.0, 6.0])` should return `4.0`, not `36.0`

## Running Tests

```bash
cargo test
```

Currently 2 tests fail:
- `test_divide` - expects division, gets multiplication
- `test_average` - expects correct average, gets wrong result due to buggy divide

## The Fix

Change line 31 in `src/lib.rs` from:
```rust
Ok(a * b)
```

to:
```rust
Ok(a / b)
```

After this fix, all tests should pass.
