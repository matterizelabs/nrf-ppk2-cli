#!/usr/bin/env bash
set -euo pipefail

REPO="matterizelabs/nrf-ppk2-cli"
VERSION="${1:-latest}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN="ppk2"

if [ "$VERSION" = "remove" ]; then
    rm -f "${INSTALL_DIR}/${BIN}"
    echo "removed ${INSTALL_DIR}/${BIN}"
    exit 0
fi

case "$(uname -s)" in
    Linux)  OS="linux"; EXT="tar.gz" ;;
    Darwin) OS="darwin"; EXT="tar.gz" ;;
    MINGW*|MSYS*|CYGWIN*) OS="windows"; EXT="zip" ;;
    *)      echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

case "$(uname -m)" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *)            echo "unsupported arch: $(uname -m)" >&2; exit 1 ;;
esac

ASSET="${BIN}-${OS}-${ARCH}.${EXT}"

if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi

echo "fetching ppk2 ${VERSION} for ${OS}/${ARCH}..."
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fsSL --retry 3 -o "$TMPDIR/$ASSET" "$URL"

if [ "$EXT" = "zip" ]; then
    unzip -q "$TMPDIR/$ASSET" -d "$TMPDIR"
else
    tar -xzf "$TMPDIR/$ASSET" -C "$TMPDIR"
fi

mkdir -p "$INSTALL_DIR"
install -m 755 "$TMPDIR/${BIN}.exe" "$INSTALL_DIR/${BIN}.exe" 2>/dev/null || \
install -m 755 "$TMPDIR/${BIN}" "$INSTALL_DIR/${BIN}"

echo "installed ppk2 to ${INSTALL_DIR}/${BIN}"

if ! echo "$PATH" | tr ':' '\n' | grep -qxF "$INSTALL_DIR"; then
    echo "note: add ${INSTALL_DIR} to your PATH"
fi
