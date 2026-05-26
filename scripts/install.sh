#!/usr/bin/env bash

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VALO_DIR="$HOME/.valo"
BIN_DIR="$VALO_DIR/bin"
DIRS=(
    "$BIN_DIR"
    "$VALO_DIR/cache"
    "$VALO_DIR/packages"
    "$VALO_DIR/toolchains"
    "$VALO_DIR/tmp"
)

log() { echo -e "${BLUE}[Valo]${NC} $1"; }
success() { echo -e "${GREEN}[Valo]${NC} $1"; }
error() { echo -e "${RED}[Valo]${NC} $1" >&2; exit 1; }

# Prerequisites
command -v curl >/dev/null 2>&1 || error "curl is required to install Valo."
command -v tar >/dev/null 2>&1 || error "tar is required to install Valo."
command -v grep >/dev/null 2>&1 || error "grep is required to install Valo."
command -v sed >/dev/null 2>&1 || error "sed is required to install Valo."

# 1. Platform Detection
case "$(uname -s)" in
    Linux*)  PLATFORM="linux" ;;
    Darwin*) PLATFORM="macos" ;;
    *)       error "Unsupported OS: $(uname -s)" ;;
esac

case "$(uname -m)" in
    x86_64 | amd64)
        ARCH="x64"
        ;;
    aarch64 | arm64)
        ARCH="arm64"
        ;;
    i386 | i486 | i586 | i686)
        ARCH="x86"
        ;;
    *)
        error "Unsupported architecture: $(uname -m)"
        ;;
esac

# release.yml does not publish macOS x86
if [ "$PLATFORM" = "macos" ] && [ "$ARCH" = "x86" ]; then
    error "macOS x86 is not supported."
fi

# 2. Setup Directory Structure
log "Creating runtime structure in $VALO_DIR..."
for dir in "${DIRS[@]}"; do
    mkdir -p "$dir"
done

# 3. Download Latest Release
LATEST_TAG="$(
    curl -fsSL https://api.github.com/repos/valolang/valo/releases |
    grep -m 1 '"tag_name":' |
    sed -E 's/.*"([^"]+)".*/\1/'
)"

if [ -z "$LATEST_TAG" ]; then
    error "Could not determine latest Valo release."
fi

DOWNLOAD_URL="https://github.com/valolang/valo/releases/download/$LATEST_TAG/valo-$PLATFORM-$ARCH.tar.gz"
TAR_FILE="$VALO_DIR/valo.tar.gz"

log "Downloading Valo from $DOWNLOAD_URL..."

curl -fL "$DOWNLOAD_URL" -o "$TAR_FILE" || error "Failed to download Valo archive."

rm -rf "$BIN_DIR"/*
tar -xzf "$TAR_FILE" -C "$BIN_DIR" --strip-components=1 || error "Failed to extract Valo archive."
rm -f "$TAR_FILE"

if [ ! -f "$BIN_DIR/valo" ]; then
    error "Valo binary was not found after extraction."
fi

chmod +x "$BIN_DIR/valo"

# 4. PATH Configuration
PROFILE=""
case "${SHELL:-}" in
    */zsh)
        PROFILE="$HOME/.zshrc"
        ;;
    */bash)
        PROFILE="$HOME/.bashrc"
        ;;
esac

if [ -n "$PROFILE" ]; then
    touch "$PROFILE"

    if grep -qF "$BIN_DIR" "$PROFILE"; then
        log "$BIN_DIR is already configured in $PROFILE."
    else
        log "Adding $BIN_DIR to $PROFILE..."
        {
            echo ""
            echo "# Valo"
            echo "export PATH=\"\$PATH:$BIN_DIR\""
        } >> "$PROFILE"

        success "Installation added to PATH. Please restart your terminal."
    fi

    export PATH="$PATH:$BIN_DIR"
else
    log "Could not detect active shell profile. Please add $BIN_DIR to your PATH manually."
    export PATH="$PATH:$BIN_DIR"
fi

# 5. Validation
log "Validating installation..."

if "$BIN_DIR/valo" version > /dev/null 2>&1; then
    success "Valo installed successfully!"
    "$BIN_DIR/valo" version
else
    error "Installation failed validation."
fi
