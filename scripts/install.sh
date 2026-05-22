#!/usr/bin/env bash

set -e

REPO="valolang/valo"
VERSION="latest"

OS="$(uname -s)"
ARCH="$(uname -m)"

TARGET=""

case "$OS" in
    Linux)
        if [ "$ARCH" = "x86_64" ]; then
            TARGET="linux-x64"
        elif [ "$ARCH" = "aarch64" ]; then
            TARGET="termux-arm64"
        fi
        ;;
    Darwin)
        echo "macOS support not added yet."
        exit 1
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

if [ -z "$TARGET" ]; then
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

FILE="valo-$TARGET.tar.gz"

URL="https://github.com/$REPO/releases/$VERSION/download/$FILE"

INSTALL_DIR="$HOME/.valo/bin"

mkdir -p "$INSTALL_DIR"

echo "Downloading Valo for $TARGET..."

curl -L "$URL" -o /tmp/valo.tar.gz

echo "Extracting..."

tar -xzf /tmp/valo.tar.gz -C /tmp

chmod +x /tmp/valo

mv /tmp/valo "$INSTALL_DIR/valo"

echo ""
echo "Valo installed successfully."
echo ""

case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        ;;
    *)
        echo "Add this to your shell config:"
        echo ""
        echo "export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac

echo ""
echo "Run:"
echo "valo --help"