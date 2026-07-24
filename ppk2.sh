#!/usr/bin/env bash
set -eu

REPO="matterizelabs/nrf-ppk2-cli"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BIN="ppk2"
CMD="${1:-install}"

if [ "$CMD" = "remove" ]; then
    sudo rm -f "${INSTALL_DIR}/${BIN}"
    echo "removed: ${INSTALL_DIR}/${BIN}"
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

PKG="${BIN}-${OS}-${ARCH}.${EXT}"
URL="https://github.com/${REPO}/releases/latest/download/${PKG}"

echo "fetch: ${URL}"
TMPDIR="$(mktemp -d)"
trap "rm -rf ${TMPDIR}" EXIT

curl -fsSL "${URL}" -o "${TMPDIR}/${PKG}"
if [ "${EXT}" = "zip" ]; then
    unzip -q "${TMPDIR}/${PKG}" -d "${TMPDIR}"
else
    tar -xzf "${TMPDIR}/${PKG}" -C "${TMPDIR}"
fi

sudo install -m 755 "${TMPDIR}/${BIN}" "${INSTALL_DIR}/${BIN}"
echo "installed: ${INSTALL_DIR}/${BIN}"
