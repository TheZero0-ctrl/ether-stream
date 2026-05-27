#!/usr/bin/env bash

set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
REPO="${REPO:-TheZero0-ctrl/ether-stream}"
APP_NAME="ether"
APP_DIR="${PREFIX}/opt/${APP_NAME}"
BIN_DIR="${PREFIX}/bin"
DESKTOP_DIR="${PREFIX}/share/applications"
ICON_DIR="${PREFIX}/share/icons/hicolor"

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "This installer supports Linux only." >&2
  exit 1
fi

if ! command -v bsdtar >/dev/null 2>&1; then
  echo "bsdtar is required (provided by libarchive on Arch)." >&2
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

cd "$TMP_DIR"
data_archive="$(bsdtar -tf "$FILE" | grep '^data\.tar' | head -n 1 || true)"
if [[ -z "$data_archive" ]]; then
  echo "Failed to locate data archive inside ${FILE}." >&2
  exit 1
fi

bsdtar -xf "$FILE" "$data_archive"
mkdir extracted
tar -xf "$data_archive" -C extracted

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR" "$BIN_DIR" "$DESKTOP_DIR"

install -m 755 "$TMP_DIR/extracted/usr/bin/ether" "$APP_DIR/ether"

cat > "$BIN_DIR/ether" <<EOF
#!/usr/bin/env bash
exec "${APP_DIR}/ether" "\$@"
EOF
chmod +x "$BIN_DIR/ether"

if [[ -f "$TMP_DIR/extracted/usr/share/applications/Ether.desktop" ]]; then
  sed \
    -e "s|^Exec=.*|Exec=${BIN_DIR}/ether|" \
    -e 's|^StartupWMClass=.*|StartupWMClass=ether|' \
    "$TMP_DIR/extracted/usr/share/applications/Ether.desktop" > "$DESKTOP_DIR/ether.desktop"
else
  cat > "$DESKTOP_DIR/ether.desktop" <<EOF
[Desktop Entry]
Categories=AudioVideo;
Comment=Ether desktop app
Exec=${BIN_DIR}/ether
StartupWMClass=ether
Icon=ether
Name=Ether
Terminal=false
Type=Application
EOF
fi

if [[ -d "$TMP_DIR/extracted/usr/share/icons/hicolor" ]]; then
  mkdir -p "$ICON_DIR"
  cp -R "$TMP_DIR/extracted/usr/share/icons/hicolor/." "$ICON_DIR/"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  find "$ICON_DIR" -mindepth 1 -maxdepth 1 -type d -print0 | while IFS= read -r -d '' icon_theme_dir; do
    gtk-update-icon-cache -q -t -f "$icon_theme_dir" >/dev/null 2>&1 || true
  done
fi

echo "Installed Ether ${TAG} for Arch-style local use."
echo "Binary: ${BIN_DIR}/ether"
echo "Desktop entry: ${DESKTOP_DIR}/ether.desktop"
