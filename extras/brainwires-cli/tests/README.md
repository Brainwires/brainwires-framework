# Brainwires CLI Tests

This directory contains comprehensive tests for brainwires-cli, including unit tests, integration tests, and AI-evaluated tests.

## Test Categories

### Unit Tests
Located in `src/` alongside the code they test.

Run with:
```bash
cargo test --lib
```

### Integration Tests  
Located in `tests/` directory. Each file tests a major subsystem:

- `cli_integration_test.rs` - CLI command execution
- `auth_integration_test.rs` - Authentication flows
- `history_integration_test.rs` - Conversation history
- `tool_execution_test.rs` - Tool execution framework
- `file_operations_test.rs` - File operation tools
- `git_tool_test.rs` - Git integration
- `bash_tool_test.rs` - Bash command execution

Run with:
```bash
cargo test --test <test_name>
# Or all integration tests:
cargo test --tests
```

### AI-Evaluated Tests ⭐
**New!** Tests that validate brainwires' core autonomous coding capabilities.

- **`ai_code_fix_test.rs`** - Autonomous code fixing with AI evaluation
  - See: [AI_CODE_FIX_TEST.md](./AI_CODE_FIX_TEST.md) for details
  - Tests the complete loop: analysis → diagnosis → fix → verification
  - Uses AI to evaluate fix quality
  - Requires `TEST_API_KEY` environment variable

Run with:
```bash
export TEST_API_KEY=your_key
cargo test --test ai_code_fix_test -- --ignored --nocapture
```

## Test Fixtures

Test fixtures are located in `tests/fixtures/`:

- `buggy_calculator/` - Simple Rust project with a known bug
  - Used by `ai_code_fix_test.rs`
  - Contains failing tests that should pass after AI fixes the bug

## Running Tests

### All Tests (Fast)
```bash
cargo test
```

### All Tests Including Ignored
```bash
cargo test -- --ignored
```

### Integration Tests Only
```bash
cargo test --tests
```

### Specific Test with Output
```bash
cargo test test_name -- --nocapture
```

### With API Key (for AI tests)

**Option 1: Using .env file (Recommended)**
```bash
# Copy and edit .env file
cp .env.example .env
# Add your key: TEST_API_KEY=bw_your_key

# Run tests
cargo test --test ai_code_fix_test -- --ignored --nocapture
```

**Option 2: Using environment variable**
```bash
export TEST_API_KEY=your_brainwires_api_key
cargo test --test ai_code_fix_test -- --ignored --nocapture
```

## Test Requirements

### Standard Tests
- No special requirements
- Run offline
- Fast execution

### AI-Evaluated Tests
- Require `TEST_API_KEY` environment variable
- Need network access (API calls)
- Slower execution (10-30s per test)
- Cost ~$0.01-0.05 per run

## Writing New Tests

### Integration Test
Create `tests/new_feature_test.rs`:
```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_new_feature() {
    Command::cargo_bin("brainwires")
        .unwrap()
        .arg("command")
        .assert()
        .success();
}
```

### AI-Evaluated Test
1. Create fixture in `tests/fixtures/your_project/`
2. Add test in `ai_code_fix_test.rs`
3. Mark with `#[ignore]` attribute
4. Document in `AI_CODE_FIX_TEST.md`

## Common Helpers

Located in `tests/common/mod.rs`:
- Test utilities
- Shared fixtures
- Helper functions

## CI/CD

Tests run automatically on:
- Pull requests
- Main branch pushes
- Release tags

AI-evaluated tests run separately (due to API key requirement).

## Test Coverage

Check coverage with:
```bash
cargo tarpaulin --out Html
# Open tarpaulin-report.html
```

## Debugging Tests

### Show Output
```bash
cargo test -- --nocapture
```

### Run Single Test
```bash
cargo test test_name
```

### Show Test List
```bash
cargo test -- --list
```

### Backtrace on Failure
```bash
RUST_BACKTRACE=1 cargo test
```

## Performance

- Unit tests: ~1-2s total
- Integration tests: ~5-10s total  
- AI tests: ~10-30s each

## Contributing

When adding new features:
1. Add unit tests in `src/`
2. Add integration test in `tests/`
3. For core capabilities, add AI-evaluated test
4. Update this README

All tests must pass before merging.
