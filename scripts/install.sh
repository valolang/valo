#!/usr/bin/env bash

set -euo pipefail

REPO="valolang/valo"
VERSION="${VALO_VERSION:-latest}"
INSTALL_DIR="${VALO_INSTALL_DIR:-$HOME/.valo/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"

TARGET=""

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64 | amd64)
                TARGET="linux-x64"
                ;;
            i386 | i486 | i586 | i686)
                TARGET="linux-x86"
                ;;
            aarch64 | arm64)
                echo "Linux ARM64 release is not available yet."
                echo "For Termux/Android ARM64, build from source for now:"
                echo ""
                echo "  cargo build --release"
                echo ""
                exit 1
                ;;
            *)
                echo "Unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;

    Darwin)
        case "$ARCH" in
            x86_64 | amd64)
                TARGET="macos-x64"
                ;;
            arm64 | aarch64)
                TARGET="macos-arm64"
                ;;
            *)
                echo "Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;

    *)
        echo "Unsupported OS: $OS"
        echo "This installer currently supports Linux and macOS."
        echo "For Windows, download the .zip release manually."
        exit 1
        ;;
esac

FILE="valo-$TARGET.tar.gz"
CHECKSUM_FILE="$FILE.sha256"

if [ "$VERSION" = "latest" ]; then
    BASE_URL="https://github.com/$REPO/releases/latest/download"
else
    BASE_URL="https://github.com/$REPO/releases/download/$VERSION"
fi

URL="$BASE_URL/$FILE"
CHECKSUM_URL="$BASE_URL/$CHECKSUM_FILE"

TMP_DIR="$(mktemp -d)"

cleanup() {
    rm -rf "$TMP_DIR"
}

trap cleanup EXIT

mkdir -p "$INSTALL_DIR"

echo "Installing Valo"
echo "Target: $TARGET"
echo "Version: $VERSION"
echo "Install dir: $INSTALL_DIR"
echo ""

echo "Downloading $FILE..."

if command -v curl >/dev/null 2>&1; then
    curl -fL "$URL" -o "$TMP_DIR/$FILE"
elif command -v wget >/dev/null 2>&1; then
    wget -O "$TMP_DIR/$FILE" "$URL"
else
    echo "Error: curl or wget is required."
    exit 1
fi

echo "Downloading checksum..."

if command -v curl >/dev/null 2>&1; then
    curl -fL "$CHECKSUM_URL" -o "$TMP_DIR/$CHECKSUM_FILE" || true
elif command -v wget >/dev/null 2>&1; then
    wget -O "$TMP_DIR/$CHECKSUM_FILE" "$CHECKSUM_URL" || true
fi

if [ -f "$TMP_DIR/$CHECKSUM_FILE" ]; then
    echo "Verifying checksum..."

    cd "$TMP_DIR"

    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c "$CHECKSUM_FILE"
    elif command -v shasum >/dev/null 2>&1; then
        EXPECTED="$(awk '{print $1}' "$CHECKSUM_FILE")"
        ACTUAL="$(shasum -a 256 "$FILE" | awk '{print $1}')"

        if [ "$EXPECTED" != "$ACTUAL" ]; then
            echo "Checksum verification failed."
            exit 1
        fi
    else
        echo "Warning: sha256sum/shasum not found. Skipping checksum verification."
    fi

    cd - >/dev/null
else
    echo "Warning: checksum file not found. Skipping verification."
fi

echo "Extracting..."

tar -xzf "$TMP_DIR/$FILE" -C "$TMP_DIR"

if [ ! -f "$TMP_DIR/valo/valo" ]; then
    echo "Error: extracted archive does not contain valo/valo."
    exit 1
fi

chmod +x "$TMP_DIR/valo/valo"

mv "$TMP_DIR/valo/valo" "$INSTALL_DIR/valo"

echo ""
echo "Valo installed successfully."
echo ""

case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        ;;
    *)
        echo "Add this to your shell config:"
        echo ""
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
        ;;
esac

echo "Run:"
echo ""
echo "  valo --help"
echo ""
