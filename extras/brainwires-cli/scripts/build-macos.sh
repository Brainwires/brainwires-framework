#!/bin/bash

# Build and upload Brainwires CLI releases for macOS platforms
# Run this script on macOS 13+ (Ventura or later)
# Supports building for both Intel (x86_64) and Apple Silicon (aarch64)
#
# Usage:
#   ./build-macos.sh              # Build all macOS targets
#   ./build-macos.sh macos-x64    # Build only Intel
#   ./build-macos.sh macos-arm64  # Build only Apple Silicon
#
# Environment variables:
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
echo -e "${GREEN}Brainwires CLI macOS Release Build${NC}"
echo -e "${GREEN}Version: ${CLI_VERSION}${NC}"
if [ "$TARGET_FILTER" != "all" ]; then
    echo -e "${GREEN}Target: ${TARGET_FILTER}${NC}"
fi
echo -e "${GREEN}========================================${NC}"
echo ""

# Ensure we're in the right directory
cd "$(dirname "$0")/.."

# Check we're on macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo -e "${RED}Error: This script must be run on macOS${NC}"
    exit 1
fi

# Pull latest and update submodules (skip if SKIP_GIT_UPDATE is set)
if [ -z "$SKIP_GIT_UPDATE" ]; then
    echo -e "${YELLOW}→ Updating repository...${NC}"
    git fetch --all --tags
    # Only checkout tag if TAG_NAME is set, otherwise stay on current branch
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

# Function to build and upload a macOS target
build_and_upload_macos() {
    local TARGET=$1
    local NAME=$2
    local IS_NATIVE=$3

    echo -e "${YELLOW}→ Building ${NAME} (${TARGET})...${NC}"

    # Add the target if not native
    if [ "$IS_NATIVE" != "true" ]; then
        rustup target add "$TARGET" 2>/dev/null || true
    fi

    cargo build --release --target "$TARGET" || {
        echo -e "${RED}  ✗ ${NAME} build failed${NC}"
        return 1
    }

    local BINARY_PATH="target/${TARGET}/release/brainwires"

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

        # Generate checksum (macOS uses shasum)
        shasum -a 256 "${ARCHIVE_NAME}" > "${ARCHIVE_NAME}.sha256"
        CHECKSUM=$(cat "${ARCHIVE_NAME}.sha256" | cut -d' ' -f1)
        FILESIZE=$(stat -f%z "${ARCHIVE_NAME}")

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

# Helper function to check if we should build a target
should_build() {
    local name=$1
    [ "$TARGET_FILTER" = "all" ] || [ "$TARGET_FILTER" = "$name" ]
}

# Detect current architecture
CURRENT_ARCH=$(uname -m)
echo -e "${YELLOW}Detected architecture: ${CURRENT_ARCH}${NC}"
echo ""

# Build macOS targets
echo -e "${GREEN}Building macOS targets...${NC}"
echo ""

if [ "$CURRENT_ARCH" = "x86_64" ]; then
    # Running on Intel Mac
    # Build Intel (native)
    if should_build "macos-x64"; then
        build_and_upload_macos "x86_64-apple-darwin" "macos-x64" "true"
    fi

    # Build Apple Silicon (cross-compile)
    if should_build "macos-arm64"; then
        build_and_upload_macos "aarch64-apple-darwin" "macos-arm64" "false"
    fi
elif [ "$CURRENT_ARCH" = "arm64" ]; then
    # Running on Apple Silicon Mac
    # Build Apple Silicon (native)
    if should_build "macos-arm64"; then
        build_and_upload_macos "aarch64-apple-darwin" "macos-arm64" "true"
    fi

    # Build Intel (cross-compile)
    if should_build "macos-x64"; then
        build_and_upload_macos "x86_64-apple-darwin" "macos-x64" "false"
    fi
else
    echo -e "${RED}Unknown architecture: ${CURRENT_ARCH}${NC}"
    exit 1
fi

# Note: We don't update the main manifest here - run update-manifest.sh after all platforms are built
# Or manually merge this into the existing manifest

if [ -n "$MANIFEST_PLATFORMS" ]; then
    echo ""
    echo -e "${YELLOW}macOS platforms built:${NC}"
    echo "$MANIFEST_PLATFORMS"
    echo ""
    echo -e "${YELLOW}Note: Run the Linux build script to update the main manifest with all platforms.${NC}"
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}macOS release build complete!${NC}"
echo -e "${GREEN}========================================${NC}"
