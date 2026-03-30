# AI Code Fix Integration Test

## Overview

This integration test validates brainwires' **core capability**: autonomous code fixing with AI evaluation.

**Test Location**: `tests/ai_code_fix_test.rs`
**Test Fixture**: `tests/fixtures/buggy_calculator/`

## What It Tests

1. **Code Analysis**: Can the AI identify a bug in real code?
2. **Code Modification**: Can it make the correct changes?
3. **Test Validation**: Does the fix make failing tests pass?
4. **AI Evaluation**: Does an AI evaluator confirm the fix is correct?

## Test Flow

```
┌─────────────────────────────────────┐
│ 1. Copy buggy project to temp dir  │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 2. Run tests (should FAIL)          │
│    ✗ test_divide                    │
│    ✗ test_average                   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 3. Run brainwires CLI to fix bug    │
│    "Fix the calculator's divide     │
│     function bug..."                │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 4. Verify code was modified         │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 5. Run tests again (should PASS)    │
│    ✓ test_divide                    │
│    ✓ test_average                   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│ 6. AI Evaluation                    │
│    - Correctness (1-10)             │
│    - Safety (1-10)                  │
│    - Quality (1-10)                 │
│    - Explanation clarity (1-10)     │
│    - overall_pass: true/false       │
└─────────────────────────────────────┘
```

## The Bug

**File**: `tests/fixtures/buggy_calculator/src/lib.rs`
**Line**: 31
**Issue**: Division function uses multiplication operator

```rust
// WRONG (current):
pub fn divide(&self, a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Cannot divide by zero".to_string())
    } else {
        Ok(a * b)  // ❌ Should be division!
    }
}

// CORRECT (expected fix):
pub fn divide(&self, a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Cannot divide by zero".to_string())
    } else {
        Ok(a / b)  // ✅ Correct operator
    }
}
```

## Running the Test

### Prerequisites

1. **Set API key** (choose one method):

   **Option A: Use .env file (Recommended)**
   ```bash
   # Copy the example file
   cp .env.example .env

   # Edit .env and add your key
   # TEST_API_KEY=bw_your_test_key_here
   ```

   **Option B: Export environment variable**
   ```bash
   export TEST_API_KEY=your_api_key_here
   ```

2. Ensure brainwires is built:
   ```bash
   cargo build --release
   ```

### Run the Test

```bash
# Run the ignored test explicitly
cargo test --test ai_code_fix_test test_fix_calculator_bug -- --ignored --nocapture

# Or run all AI tests
cargo test --test ai_code_fix_test -- --ignored --nocapture
```

### Expected Output

```
📁 Test project copied to: /tmp/.tmpXXXXXX/calculator_project
🧪 Running initial tests (should FAIL)...
✓ Confirmed: Tests fail as expected

🤖 Running brainwires to fix the bug...
📝 AI Response:
[AI's analysis and fix explanation]

✓ Code was modified
🧪 Running tests after fix (should PASS)...
✓ All tests pass!
✓ Correct operator (/) found in code

🔍 Evaluating fix quality with AI...
📊 AI Evaluation:
{
  "correctness": 10,
  "safety": 10,
  "quality": 9,
  "explanation_clarity": 9,
  "overall_pass": true,
  "reasoning": "The fix correctly changes..."
}
✓ Evaluation parsed successfully
  correctness: 10/10
  safety: 10/10
  quality: 9/10
  explanation_clarity: 9/10
✅ AI evaluation: PASS
```

## Test Assertions

The test verifies:

1. ✅ **Initial state**: Tests fail before fix
2. ✅ **Code modification**: File is actually changed
3. ✅ **Test success**: Tests pass after fix
4. ✅ **Correct operator**: Division (`/`) is used instead of multiplication (`*`)
5. ✅ **AI evaluation**: Evaluator confirms the fix is correct

## AI Evaluation Criteria

The test uses a secondary AI (Claude Haiku for cost efficiency) to evaluate:

### Metrics (1-10 scale):
- **Correctness**: Does it fix the bug?
- **Safety**: No new bugs introduced?
- **Quality**: Clean, well-written code?
- **Explanation Clarity**: Did the AI explain well?

### Overall Pass/Fail:
- `overall_pass: true` - Fix is good ✅
- `overall_pass: false` - Fix has issues ❌

## Why This Test Matters

This test validates the **entire value proposition** of brainwires:

1. **Autonomous Understanding**: AI must comprehend the codebase
2. **Accurate Diagnosis**: Must identify the specific bug
3. **Correct Modification**: Must make the right change
4. **Verification**: Must confirm the fix works
5. **Quality**: Fix should be production-quality code

Unlike unit tests that check individual functions, this tests the **complete autonomous coding loop** that users rely on.

## Future Test Ideas

The test file includes placeholders for:

- `test_fix_with_multiple_bugs`: Test fixing multiple related bugs
- `test_fix_preserves_working_code`: Ensure working code isn't broken

Additional test scenarios:
- Type errors in TypeScript
- Logic bugs in complex algorithms
- API integration issues
- Security vulnerabilities
- Performance bottlenecks

## Troubleshooting

### Test Fails: "Tests should fail before the fix"
The buggy calculator tests are passing - check that `src/lib.rs` has the bug:
```rust
Ok(a * b)  // Should be here on line 31
```

### Test Fails: "Tests should pass after the fix"
The AI didn't fix the bug correctly. Check:
- Is the API key valid?
- Is the model available?
- Is the prompt clear enough?

### Test Fails: AI evaluation
The evaluation AI might not return valid JSON. This is a known limitation of the current implementation. The main test (code fix) still passes.

### Test is Ignored
Use `-- --ignored` flag to run ignored tests:
```bash
cargo test -- --ignored
```

## Configuration

### Models Used:
- **Fixer**: `claude-3-5-sonnet-20241022` (most capable)
- **Evaluator**: `claude-3-haiku-20240307` (fast, cheap)

### Timeouts:
- No timeout (test runs until completion)
- Typical runtime: 10-30 seconds

### Cost:
- ~$0.01-0.05 per test run (depending on token usage)

## Contributing

To add new test fixtures:

1. Create directory in `tests/fixtures/your_project/`
2. Add buggy code with failing tests
3. Document the bug in `README.md`
4. Add test case in `ai_code_fix_test.rs`

Example:
```rust
#[test]
#[ignore]
fn test_fix_your_bug() {
    // Copy fixture
    // Run brainwires
    // Verify fix
    // Evaluate with AI
}
```
