#!/usr/bin/env bash
set -euo pipefail

REPO="matterizelabs/nrf-ppk2-cli"
VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
    Linux)  OS=linux ;;
    Darwin) OS=darwin ;;
    *)      echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

case "$(uname -m)" in
    x86_64|amd64) ARCH=x86_64 ;;
    aarch64|arm64) ARCH=arm64 ;;
    *)            echo "unsupported arch: $(uname -m)" >&2; exit 1 ;;
esac

ASSET="ppk2-${OS}-${ARCH}.tar.gz"

if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi

echo "fetching ppk2 ${VERSION} for ${OS}/${ARCH}..."
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL --retry 3 -o "$TMPDIR/$ASSET" "$URL"
tar -xzf "$TMPDIR/$ASSET" -C "$TMPDIR"

mkdir -p "$INSTALL_DIR"
install -m 755 "$TMPDIR/ppk2" "$INSTALL_DIR/ppk2"

echo "installed ppk2 to $INSTALL_DIR/ppk2"

if ! echo "$PATH" | tr ':' '\n' | grep -qxF "$INSTALL_DIR"; then
    echo "note: add $INSTALL_DIR to your PATH"
fi
