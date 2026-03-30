#!/bin/bash

# Build and upload Brainwires CLI releases for Linux/Windows platforms
# This script is triggered by the CI webhook when a tag is pushed
#
# Usage:
#   ./build-release.sh              # Build all targets
#   ./build-release.sh linux-x64    # Build only Linux x64
#   ./build-release.sh linux-arm64  # Build only Linux ARM64
#   ./build-release.sh linux-armv7  # Build only Linux ARMv7
#   ./build-release.sh windows-x64  # Build only Windows x64
#
# Environment variables:
#   WINDOWS_MSVC=true   # Also build Windows MSVC target
#   SKIP_GIT_UPDATE=1   # Skip git fetch/checkout (useful for testing)

set -e

# Load environment variables from .env if it exists
SCRIPT_DIR="$(dirname "$0")"
if [ -f "$SCRIPT_DIR/../.env" ]; then
    export $(grep -v '^#' "$SCRIPT_DIR/../.env" | grep -v '^$' | xargs)
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Target selection (first argument, or "all")
TARGET_FILTER="${1:-all}"

# Get version from environment or Cargo.toml
if [ -n "$VERSION" ]; then
    CLI_VERSION="$VERSION"
else
    CLI_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
fi

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Brainwires CLI Release Build${NC}"
echo -e "${GREEN}Version: ${CLI_VERSION}${NC}"
if [ "$TARGET_FILTER" != "all" ]; then
    echo -e "${GREEN}Target: ${TARGET_FILTER}${NC}"
fi
echo -e "${GREEN}========================================${NC}"
echo ""

# Ensure we're in the right directory
cd "$(dirname "$0")/.."

# Pull latest and update submodules (skip if SKIP_GIT_UPDATE is set)
if [ -z "$SKIP_GIT_UPDATE" ]; then
    echo -e "${YELLOW}→ Updating repository...${NC}"
    git fetch --all --tags
    # Only checkout tag if TAG_NAME is set (webhook trigger), otherwise stay on current branch
    if [ -n "$TAG_NAME" ]; then
        git checkout "$TAG_NAME" 2>/dev/null || git checkout main
    fi
    git submodule update --init --recursive
else
    echo -e "${YELLOW}→ Skipping git update (SKIP_GIT_UPDATE set)${NC}"
fi

# Initialize manifest data
MANIFEST_PLATFORMS=""
BUILD_DATE=$(date +%Y-%m-%d)

# Function to build and upload a target
build_and_upload() {
    local TARGET=$1
    local NAME=$2
    local USE_CROSS=$3
    local BINARY_PATH=""

    echo -e "${YELLOW}→ Building ${NAME} (${TARGET})...${NC}"

    if [ "$USE_CROSS" = "true" ]; then
        # Check if cross is installed
        if ! command -v cross &> /dev/null; then
            echo -e "${YELLOW}  Installing cross tool...${NC}"
            cargo install cross --version 0.2.5 || {
                echo -e "${RED}  ✗ Failed to install cross${NC}"
                return 1
            }
        fi
        cross build --release --target "$TARGET" || {
            echo -e "${RED}  ✗ ${NAME} build failed${NC}"
            return 1
        }
        BINARY_PATH="target/${TARGET}/release/brainwires"
    else
        cargo build --release --target "$TARGET" || {
            echo -e "${RED}  ✗ ${NAME} build failed${NC}"
            return 1
        }
        BINARY_PATH="target/${TARGET}/release/brainwires"
    fi

    if [ ! -f "$BINARY_PATH" ]; then
        # Try without target subfolder for native builds
        BINARY_PATH="target/release/brainwires"
    fi

    if [ -f "$BINARY_PATH" ]; then
        # Create archive
        rm -rf dist
        mkdir -p dist
        cp "$BINARY_PATH" dist/
        cp README.md LICENSE CHANGELOG.md dist/ 2>/dev/null || true

        ARCHIVE_NAME="brainwires-${CLI_VERSION}-${NAME}.tar.xz"
        cd dist
        tar -cJf "../${ARCHIVE_NAME}" *
        cd ..

        # Generate checksum
        sha256sum "${ARCHIVE_NAME}" > "${ARCHIVE_NAME}.sha256"
        CHECKSUM=$(cat "${ARCHIVE_NAME}.sha256" | cut -d' ' -f1)
        FILESIZE=$(stat -c%s "${ARCHIVE_NAME}")

        echo -e "${GREEN}  ✓ Built ${ARCHIVE_NAME} (${FILESIZE} bytes)${NC}"

        # Upload to Supabase Storage
        if [ -n "$SUPABASE_URL" ] && [ -n "$SUPABASE_SERVICE_KEY" ]; then
            echo -e "${YELLOW}  Uploading to Supabase Storage...${NC}"

            # Upload versioned
            curl -X POST "${SUPABASE_URL}/storage/v1/object/cli-releases/${CLI_VERSION}/${NAME}/${ARCHIVE_NAME}" \
                -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
                -H "Content-Type: application/x-xz" \
                --data-binary "@${ARCHIVE_NAME}" \
                --silent --show-error && echo -e "${GREEN}  ✓ Uploaded versioned${NC}"

            # Upload to stable
            STABLE_ARCHIVE="brainwires-latest-${NAME}.tar.xz"
            cp "${ARCHIVE_NAME}" "${STABLE_ARCHIVE}"
            curl -X POST "${SUPABASE_URL}/storage/v1/object/cli-releases/stable/${NAME}/${STABLE_ARCHIVE}" \
                -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
                -H "Content-Type: application/x-xz" \
                --data-binary "@${STABLE_ARCHIVE}" \
                --silent --show-error && echo -e "${GREEN}  ✓ Uploaded stable${NC}"

            # Add to manifest platforms (global variable)
            if [ -n "$MANIFEST_PLATFORMS" ]; then
                MANIFEST_PLATFORMS="${MANIFEST_PLATFORMS},"
            fi
            MANIFEST_PLATFORMS="${MANIFEST_PLATFORMS}
        \"${TARGET}\": {
          \"url\": \"/${CLI_VERSION}/${NAME}/${ARCHIVE_NAME}\",
          \"sha256\": \"${CHECKSUM}\",
          \"size\": ${FILESIZE},
          \"archive\": \"tar.xz\"
        }"

            rm -f "${STABLE_ARCHIVE}"
        else
            echo -e "${YELLOW}  ⚠ SUPABASE_URL or SUPABASE_SERVICE_KEY not set, skipping upload${NC}"
        fi

        # Cleanup archive
        rm -f "${ARCHIVE_NAME}" "${ARCHIVE_NAME}.sha256"
        rm -rf dist
    else
        echo -e "${RED}  ✗ Binary not found at ${BINARY_PATH}${NC}"
        return 1
    fi
}

# Function to build and upload Windows target
build_and_upload_windows() {
    local TARGET=$1
    local NAME=$2

    echo -e "${YELLOW}→ Building ${NAME} (${TARGET})...${NC}"

    # Use cross for Windows builds
    if ! command -v cross &> /dev/null; then
        echo -e "${YELLOW}  Installing cross tool...${NC}"
        cargo install cross --version 0.2.5 || {
            echo -e "${RED}  ✗ Failed to install cross${NC}"
            return 1
        }
    fi

    cross build --release --target "$TARGET" || {
        echo -e "${RED}  ✗ ${NAME} build failed${NC}"
        return 1
    }

    local BINARY_PATH="target/${TARGET}/release/brainwires.exe"

    if [ -f "$BINARY_PATH" ]; then
        # Create archive (zip for Windows)
        rm -rf dist
        mkdir -p dist
        cp "$BINARY_PATH" dist/
        cp README.md LICENSE CHANGELOG.md dist/ 2>/dev/null || true

        ARCHIVE_NAME="brainwires-${CLI_VERSION}-${NAME}.zip"
        cd dist
        zip -q "../${ARCHIVE_NAME}" *
        cd ..

        # Generate checksum
        sha256sum "${ARCHIVE_NAME}" > "${ARCHIVE_NAME}.sha256"
        CHECKSUM=$(cat "${ARCHIVE_NAME}.sha256" | cut -d' ' -f1)
        FILESIZE=$(stat -c%s "${ARCHIVE_NAME}")

        echo -e "${GREEN}  ✓ Built ${ARCHIVE_NAME} (${FILESIZE} bytes)${NC}"

        # Upload to Supabase Storage
        if [ -n "$SUPABASE_URL" ] && [ -n "$SUPABASE_SERVICE_KEY" ]; then
            echo -e "${YELLOW}  Uploading to Supabase Storage...${NC}"

            # Upload versioned
            curl -X POST "${SUPABASE_URL}/storage/v1/object/cli-releases/${CLI_VERSION}/${NAME}/${ARCHIVE_NAME}" \
                -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
                -H "Content-Type: application/zip" \
                --data-binary "@${ARCHIVE_NAME}" \
                --silent --show-error && echo -e "${GREEN}  ✓ Uploaded versioned${NC}"

            # Upload to stable
            STABLE_ARCHIVE="brainwires-latest-${NAME}.zip"
            cp "${ARCHIVE_NAME}" "${STABLE_ARCHIVE}"
            curl -X POST "${SUPABASE_URL}/storage/v1/object/cli-releases/stable/${NAME}/${STABLE_ARCHIVE}" \
                -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
                -H "Content-Type: application/zip" \
                --data-binary "@${STABLE_ARCHIVE}" \
                --silent --show-error && echo -e "${GREEN}  ✓ Uploaded stable${NC}"

            # Add to manifest platforms (global variable)
            if [ -n "$MANIFEST_PLATFORMS" ]; then
                MANIFEST_PLATFORMS="${MANIFEST_PLATFORMS},"
            fi
            MANIFEST_PLATFORMS="${MANIFEST_PLATFORMS}
        \"${TARGET}\": {
          \"url\": \"/${CLI_VERSION}/${NAME}/${ARCHIVE_NAME}\",
          \"sha256\": \"${CHECKSUM}\",
          \"size\": ${FILESIZE},
          \"archive\": \"zip\"
        }"

            rm -f "${STABLE_ARCHIVE}"
        else
            echo -e "${YELLOW}  ⚠ SUPABASE_URL or SUPABASE_SERVICE_KEY not set, skipping upload${NC}"
        fi

        # Cleanup archive
        rm -f "${ARCHIVE_NAME}" "${ARCHIVE_NAME}.sha256"
        rm -rf dist
    else
        echo -e "${RED}  ✗ Binary not found at ${BINARY_PATH}${NC}"
        return 1
    fi
}

# Helper function to check if we should build a target
should_build() {
    local name=$1
    [ "$TARGET_FILTER" = "all" ] || [ "$TARGET_FILTER" = "$name" ]
}

# Build Linux targets
if should_build "linux-x64" || should_build "linux-arm64" || should_build "linux-armv7"; then
    echo ""
    echo -e "${GREEN}Building Linux targets...${NC}"
    echo ""
fi

# Build Linux x64 (native)
if should_build "linux-x64"; then
    rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true
    build_and_upload "x86_64-unknown-linux-gnu" "linux-x64" "false"
fi

# Build Linux ARM64 (cross-compile)
if should_build "linux-arm64"; then
    build_and_upload "aarch64-unknown-linux-gnu" "linux-arm64" "true"
fi

# Build Linux ARMv7 (cross-compile) - Raspberry Pi
if should_build "linux-armv7"; then
    build_and_upload "armv7-unknown-linux-gnueabihf" "linux-armv7" "true"
fi

# Build Windows targets
if should_build "windows-x64" || should_build "windows-x64-msvc"; then
    echo ""
    echo -e "${GREEN}Building Windows targets...${NC}"
    echo ""
fi

# Build Windows GNU (default, works well with cross-rs)
if should_build "windows-x64"; then
    build_and_upload_windows "x86_64-pc-windows-gnu" "windows-x64"
fi

# Build Windows MSVC (optional, set WINDOWS_MSVC=true to enable)
if [ "$WINDOWS_MSVC" = "true" ] && should_build "windows-x64-msvc"; then
    echo -e "${YELLOW}→ MSVC build enabled...${NC}"
    build_and_upload_windows "x86_64-pc-windows-msvc" "windows-x64-msvc"
fi

# Upload manifest with all platforms
if [ -n "$SUPABASE_URL" ] && [ -n "$SUPABASE_SERVICE_KEY" ] && [ -n "$MANIFEST_PLATFORMS" ]; then
    echo ""
    echo -e "${YELLOW}→ Uploading manifest...${NC}"
    cat > manifest.json << EOF
{
  "stable": "${CLI_VERSION}",
  "releases": {
    "${CLI_VERSION}": {
      "date": "${BUILD_DATE}",
      "platforms": {${MANIFEST_PLATFORMS}
      }
    }
  }
}
EOF
    curl -X POST "${SUPABASE_URL}/storage/v1/object/cli-releases/manifest.json" \
        -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
        -H "Content-Type: application/json" \
        --data-binary "@manifest.json" \
        --silent --show-error && echo -e "${GREEN}✓ Uploaded manifest${NC}"
    rm -f manifest.json
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Release build complete!${NC}"
echo -e "${GREEN}========================================${NC}"
