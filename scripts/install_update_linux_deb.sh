#!/usr/bin/env bash

set -euo pipefail

REPO="${REPO:-TheZero0-ctrl/ether-stream}"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "This installer supports Linux only." >&2
  exit 1
fi

if ! command -v dpkg >/dev/null 2>&1; then
  echo "This installer requires dpkg and is intended for Debian-based systems." >&2
  exit 1
fi

ARCH_RAW="$(uname -m)"
case "$ARCH_RAW" in
  x86_64|amd64) ARCH="amd64" ;;
  *)
    echo "Unsupported architecture: $ARCH_RAW (supported: x86_64/amd64)" >&2
    exit 1
    ;;
esac

resolve_latest_tag() {
  local latest_url="https://api.github.com/repos/${REPO}/releases/latest"
  local list_url="https://api.github.com/repos/${REPO}/releases"
  local tag

  tag="$(curl -fsSL "$latest_url" 2>/dev/null | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1 || true)"
  if [[ -n "$tag" ]]; then
    printf '%s\n' "$tag"
    return 0
  fi

  tag="$(curl -fsSL "$list_url" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1 || true)"
  if [[ -n "$tag" ]]; then
    printf '%s\n' "$tag"
    return 0
  fi

  return 1
}

if [[ -n "${VERSION:-}" ]]; then
  TAG="$VERSION"
else
  TAG="$(resolve_latest_tag)"
fi

if [[ -z "$TAG" ]]; then
  echo "Failed to resolve release tag. Set VERSION explicitly, e.g. VERSION=v0.1.0." >&2
  exit 1
fi

VERSION_NO_V="${TAG#v}"
FILE="Ether_${VERSION_NO_V}_${ARCH}.deb"
URL="https://github.com/${REPO}/releases/download/${TAG}/${FILE}"

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "Downloading ${URL}"
curl -fL "$URL" -o "$TMP_DIR/$FILE"

if [[ "${EUID}" -ne 0 ]]; then
  if command -v sudo >/dev/null 2>&1; then
    sudo dpkg -i "$TMP_DIR/$FILE"
  else
    echo "Root privileges required to install the .deb package." >&2
    echo "Re-run as root or install sudo." >&2
    exit 1
  fi
else
  dpkg -i "$TMP_DIR/$FILE"
fi

echo "Installed Ether ${TAG} from ${FILE}"
