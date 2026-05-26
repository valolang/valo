#!/usr/bin/env bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VALO_DIR="$HOME/.valo"
BIN_DIR="$VALO_DIR/bin"
DIRS=("$BIN_DIR" "$VALO_DIR/cache" "$VALO_DIR/packages" "$VALO_DIR/toolchains" "$VALO_DIR/tmp")

log() { echo -e "${BLUE}[Valo]${NC} $1"; }
success() { echo -e "${GREEN}[Valo]${NC} $1"; }
error() { echo -e "${RED}[Valo]${NC} $1"; exit 1; }

# Prerequisites
command -v curl >/dev/null 2>&1 || error "curl is required to install Valo."

# 1. Platform Detection
case "$(uname -s)" in
    Linux*)     PLATFORM="linux" ;;
    Darwin*)    PLATFORM="macos" ;;
    *)          error "Unsupported OS: $(uname -s)" ;;
esac

ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  ARCH="x64" ;;
    aarch64) ARCH="arm64" ;;
esac

# 2. Setup Directory Structure
log "Creating runtime structure in $VALO_DIR..."
for dir in "${DIRS[@]}"; do mkdir -p "$dir"; done

# 3. Download Latest Release
LATEST_URL="https://github.com/valolang/valo/releases/latest/download/valo-$PLATFORM-$ARCH"
log "Downloading Valo from $LATEST_URL..."
curl -fsSL "$LATEST_URL" -o "$BIN_DIR/valo"
chmod +x "$BIN_DIR/valo"

# 4. PATH Configuration
PROFILE=""
case "$SHELL" in
    */zsh)  PROFILE="$HOME/.zshrc" ;;
    */bash) PROFILE="$HOME/.bashrc" ;;
esac

if [ -n "$PROFILE" ] && [ -f "$PROFILE" ]; then
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        log "Adding $BIN_DIR to $PROFILE..."
        echo "export PATH=\"\$PATH:$BIN_DIR\"" >> "$PROFILE"
        success "Installation added to PATH. Please restart your terminal."
    else
        log "$BIN_DIR is already in PATH."
    fi
else
    log "Could not detect active shell profile. Please add $BIN_DIR to your PATH manually."
fi

# 5. Validation
log "Validating installation..."
if "$BIN_DIR/valo" version > /dev/null 2>&1; then
    success "Valo installed successfully!"
    "$BIN_DIR/valo" version
else
    error "Installation failed validation."
fi
