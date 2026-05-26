#!/usr/bin/env bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VALO_DIR="$HOME/.valo"
BIN_DIR="$VALO_DIR/bin"
CACHE_DIR="$VALO_DIR/cache"
PKG_DIR="$VALO_DIR/packages"
TOOL_DIR="$VALO_DIR/toolchains"
TMP_DIR="$VALO_DIR/tmp"

log() { echo -e "${BLUE}[Valo]${NC} $1"; }
success() { echo -e "${GREEN}[Valo]${NC} $1"; }
error() { echo -e "${RED}[Valo]${NC} $1"; exit 1; }

# 1. Platform Detection
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
    Linux*)     PLATFORM="linux" ;;
    Darwin*)    PLATFORM="macos" ;;
    *)          error "Unsupported OS: $OS" ;;
esac

if [ "$ARCH" = "x86_64" ]; then ARCH="x64"; fi

# 2. Setup Directory Structure
log "Creating runtime structure in $VALO_DIR..."
mkdir -p "$BIN_DIR" "$CACHE_DIR" "$PKG_DIR" "$TOOL_DIR" "$TMP_DIR"

# 3. Download Latest Release
LATEST_URL="https://github.com/valolang/valo/releases/latest/download/valo-$PLATFORM-$ARCH"
log "Downloading Valo from $LATEST_URL..."
curl -fsSL "$LATEST_URL" -o "$BIN_DIR/valo"
chmod +x "$BIN_DIR/valo"

# 4. PATH Configuration
PROFILE=""
if [ -n "$ZSH_VERSION" ]; then
    PROFILE="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ]; then
    PROFILE="$HOME/.bashrc"
fi

if [ -n "$PROFILE" ]; then
    if ! grep -q "$BIN_DIR" "$PROFILE"; then
        log "Adding $BIN_DIR to $PROFILE..."
        echo "export PATH=\"\$PATH:$BIN_DIR\"" >> "$PROFILE"
        success "Added to PATH. Please run 'source $PROFILE' or restart your terminal."
    else
        log "$BIN_DIR already in PATH."
    fi
else
    log "Could not detect shell profile. Please add $BIN_DIR to your PATH manually."
fi

# 5. Validation
log "Validating installation..."
if "$BIN_DIR/valo" version > /dev/null; then
    success "Valo installed successfully!"
    "$BIN_DIR/valo" version
else
    error "Installation failed validation."
fi
