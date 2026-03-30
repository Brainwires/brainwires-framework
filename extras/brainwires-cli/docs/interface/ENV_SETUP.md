# Environment Variables Setup

## Overview

Brainwires CLI uses `.env` files to manage environment variables without committing secrets to git.

## Quick Start

```bash
# 1. Copy the example file
cp .env.example .env

# 2. Edit .env with your values
nano .env  # or your preferred editor

# 3. Run tests or development commands
cargo test -- --ignored
```

## What's in .env?

### Test Configuration

**`TEST_API_KEY`** - Required for AI integration tests
```bash
TEST_API_KEY=bw_your_test_key_here
```

Get your test key from: https://brainwires.studio

### Optional Runtime Configuration

**`BRAINWIRES_API_KEY`** - Default API key
```bash
BRAINWIRES_API_KEY=bw_your_api_key_here
```

**`BRAINWIRES_BACKEND_URL`** - Backend endpoint
```bash
BRAINWIRES_BACKEND_URL=https://api.brainwires.studio  # Production
# or
BRAINWIRES_BACKEND_URL=http://localhost:3000  # Local development
```

**`BRAINWIRES_MODEL`** - Default model
```bash
BRAINWIRES_MODEL=claude-3-5-sonnet-20241022
```

**`RUST_LOG`** - Logging level
```bash
RUST_LOG=debug  # For development
RUST_LOG=info   # For production
```

## How It Works

### For Tests

The integration tests automatically load `.env` using the `dotenvy` crate:

```rust
// tests/ai_code_fix_test.rs
fn load_env() {
    let _ = dotenvy::dotenv();
}

#[test]
fn test_something() {
    load_env();  // Loads .env automatically
    // ...
}
```

### For Runtime

While tests use `.env` automatically, runtime commands can also use environment variables:

```bash
# Option 1: Use .env file
source .env
brainwires chat

# Option 2: Inline
BRAINWIRES_API_KEY=bw_xxx brainwires chat

# Option 3: Export
export BRAINWIRES_API_KEY=bw_xxx
brainwires chat
```

## File Structure

```
brainwires-cli/
├── .env.example      ← Template (committed to git)
├── .env              ← Your secrets (NEVER committed)
├── .gitignore        ← Excludes .env files
└── tests/
    └── ai_code_fix_test.rs  ← Loads .env
```

## Security Best Practices

### ✅ DO:
- Use `.env` for development secrets
- Copy from `.env.example` template
- Keep `.env` local only
- Use different keys for test/dev/prod
- Rotate keys regularly

### ❌ DON'T:
- Commit `.env` to git
- Share `.env` files directly
- Use production keys for testing
- Store `.env` in cloud sync folders
- Use weak API keys

## Different Environments

### Development
```bash
# .env.development
TEST_API_KEY=bw_dev_xxx
BRAINWIRES_BACKEND_URL=http://localhost:3000
RUST_LOG=debug
```

### Testing
```bash
# .env.test
TEST_API_KEY=bw_test_xxx
BRAINWIRES_BACKEND_URL=https://test-api.brainwires.studio
```

### Production
```bash
# .env.production
BRAINWIRES_API_KEY=bw_prod_xxx
BRAINWIRES_BACKEND_URL=https://api.brainwires.studio
RUST_LOG=info
```

Load specific environment:
```bash
# Copy the right one
cp .env.development .env

# Or use direnv (if installed)
ln -s .env.development .env
```

## Troubleshooting

### "TEST_API_KEY not set"

**Problem**: Test can't find API key

**Solutions**:
```bash
# Check if .env exists
ls -la .env

# Verify content
cat .env | grep TEST_API_KEY

# Ensure no trailing spaces
TEST_API_KEY=bw_xxx  # ✅ Good
TEST_API_KEY=bw_xxx   # ❌ Bad (trailing space)

# Copy from template if missing
cp .env.example .env
```

### .env not loading

**Problem**: Tests don't see environment variables

**Causes**:
1. `.env` file doesn't exist
2. `.env` in wrong directory (should be project root)
3. Syntax errors in `.env`

**Debug**:
```bash
# Check file location
pwd                     # Should be project root
ls -la .env            # Should exist

# Validate syntax
cat .env               # Check for errors

# Test manually
source .env
echo $TEST_API_KEY     # Should print your key
```

### Keys not working

**Problem**: Valid key but tests fail

**Solutions**:
```bash
# 1. Check key format
TEST_API_KEY=bw_dev_xxx  # Dev key
TEST_API_KEY=bw_xxx      # Production key

# 2. Verify key is active
# Log into brainwires.studio and check

# 3. Check backend URL matches key type
# Dev keys need dev backend
# Prod keys need prod backend
```

## CI/CD Integration

For GitHub Actions:

```yaml
# .github/workflows/test.yml
jobs:
  test:
    steps:
      - name: Run AI tests
        env:
          TEST_API_KEY: ${{ secrets.BRAINWIRES_TEST_KEY }}
        run: cargo test -- --ignored
```

Add secret in GitHub:
Settings → Secrets → Actions → New repository secret

## Using direnv (Optional)

For automatic environment loading:

```bash
# Install direnv
# macOS: brew install direnv
# Linux: apt install direnv

# Setup
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
source ~/.bashrc

# Create .envrc (links to .env)
echo 'dotenv' > .envrc
direnv allow

# Now .env loads automatically when entering directory!
cd brainwires-cli  # ← .env loaded
```

## Best Tools

- **direnv** - Auto-load .env when entering directory
- **1Password** - Store API keys securely
- **pass** - CLI password manager
- **envchain** - Secure environment variables

## Alternative: System Keyring

For maximum security, use system keyring:

```bash
# macOS Keychain
security add-generic-password -a brainwires -s test_api_key -w

# Linux Secret Service
secret-tool store --label='Brainwires Test Key' service brainwires key test_api_key

# Then retrieve in code
# (requires additional dependencies)
```

## More Information

- **dotenvy**: https://crates.io/crates/dotenvy
- **direnv**: https://direnv.net/
- **12-factor app**: https://12factor.net/config
