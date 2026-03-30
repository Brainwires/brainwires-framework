# Release Process for Brainwires CLI

This document describes how to release new versions of the Brainwires CLI.

## Prerequisites

### GitHub Repository Secrets

Before running the first release, add these secrets to your GitHub repository:

1. Go to your repo → Settings → Secrets and variables → Actions
2. Add the following repository secrets:

| Secret Name | Description | Where to Find |
|-------------|-------------|---------------|
| `SUPABASE_URL` | Your Supabase project URL | Supabase Dashboard → Settings → API → Project URL |
| `SUPABASE_SERVICE_KEY` | Service role key for uploads | Supabase Dashboard → Settings → API → service_role key |

**Important:** Use the `service_role` key, not the `anon` key. The service role key is needed to upload binaries to the storage bucket.

## Creating a Release

### Option 1: Tag-based Release (Recommended)

1. Update version in `Cargo.toml`:
   ```toml
   version = "0.6.0"
   ```

2. Update `CHANGELOG.md` with release notes

3. Commit changes:
   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore: bump version to 0.6.0"
   ```

4. Create and push a version tag:
   ```bash
   git tag v0.6.0
   git push origin main --tags
   ```

5. The GitHub Actions workflow will automatically:
   - Build binaries for all 6 platforms
   - Upload to Supabase Storage
   - Create a GitHub Release

### Option 2: Manual Workflow Dispatch

1. Go to Actions → Release workflow
2. Click "Run workflow"
3. Enter the version number (e.g., `0.6.0`)
4. Click "Run workflow"

## Supported Platforms

The release workflow builds for these platforms:

| Platform | Target Triple | Runner |
|----------|---------------|--------|
| Linux x64 | `x86_64-unknown-linux-gnu` | ubuntu-latest |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | ubuntu-24.04-arm64 |
| Linux ARMv7 | `armv7-unknown-linux-gnueabihf` | ubuntu-latest (cross) |
| macOS Intel | `x86_64-apple-darwin` | macos-13 |
| macOS ARM | `aarch64-apple-darwin` | macos-latest |
| Windows x64 | `x86_64-pc-windows-msvc` | windows-latest |

## Storage Structure

Binaries are uploaded to Supabase Storage bucket `cli-releases`:

```
cli-releases/
├── manifest.json           # Version manifest with checksums
├── stable/                 # Latest stable release
│   ├── linux-x64/
│   ├── linux-arm64/
│   ├── linux-armv7/
│   ├── macos-x64/
│   ├── macos-arm64/
│   └── windows-x64/
└── {version}/              # Versioned releases
    ├── linux-x64/
    └── ...
```

## User Installation

After release, users can install via:

**macOS / Linux:**
```bash
curl -fsSL https://brainwires.studio/api/install | sh
```

**Windows (PowerShell):**
```powershell
irm https://brainwires.studio/api/install/windows | iex
```

**Direct Download:**
Visit https://brainwires.studio/cli/downloads

## Troubleshooting

### Build Failures

- **ARMv7 cross-compilation:** Requires `gcc-arm-linux-gnueabihf` which is installed by the workflow
- **macOS signing:** Currently not implemented; binaries may trigger Gatekeeper warnings

### Upload Failures

- Verify `SUPABASE_URL` and `SUPABASE_SERVICE_KEY` secrets are correct
- Check that the `cli-releases` bucket exists and has public read access
- Ensure the service role key has write permissions

### Missing Binaries

If a platform build fails, the workflow continues with other platforms. Check the Actions log for specific errors.
