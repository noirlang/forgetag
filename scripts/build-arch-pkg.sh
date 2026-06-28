#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH_OUT_DIR="${ARCH_OUT_DIR:-$ROOT_DIR/target/release/bundle/arch}"
STAGE_DIR="$ARCH_OUT_DIR/pkg-root"
PACKAGE_NAME="forgetag"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/src-tauri/Cargo.toml" | head -n1)"

if [[ -z "$VERSION" ]]; then
  echo "Cargo.toml version could not be detected" >&2
  exit 1
fi

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required tool not found: $1" >&2
    exit 1
  fi
}

require_tool bsdtar
require_tool zstd

BINARY="$ROOT_DIR/target/release/forgetag"
if [[ ! -x "$BINARY" ]]; then
  echo "Release binary not found at $BINARY. Run 'cargo build --release' first." >&2
  exit 1
fi

rm -rf "$STAGE_DIR"
mkdir -p \
  "$STAGE_DIR/usr/bin" \
  "$STAGE_DIR/usr/share/applications" \
  "$STAGE_DIR/usr/share/icons/hicolor"

install -m 755 "$BINARY" "$STAGE_DIR/usr/bin/forgetag"

for size in 16x16 32x32 64x64 128x128 256x256 512x512; do
  icon_src="$ROOT_DIR/src-tauri/icons/${size}.png"
  if [[ -f "$icon_src" ]]; then
    install -Dm644 "$icon_src" \
      "$STAGE_DIR/usr/share/icons/hicolor/${size}/apps/forgetag.png"
  fi
done

DESKTOP_SRC="$ROOT_DIR/target/release/bundle/deb/forgetag/usr/share/applications/forgetag.desktop"
if [[ -f "$DESKTOP_SRC" ]]; then
  install -m 644 "$DESKTOP_SRC" "$STAGE_DIR/usr/share/applications/forgetag.desktop"
else
  cat > "$STAGE_DIR/usr/share/applications/forgetag.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=forgetag
Comment=Local-first file library with tags, notes, and ZIP backups
Exec=forgetag
Icon=forgetag
Terminal=false
Categories=Utility;FileManager;
StartupWMClass=forgetag
EOF
fi

installed_size="$(du -sb "$STAGE_DIR" | awk '{print $1}')"

cat > "$STAGE_DIR/.PKGINFO" <<EOF
pkgname = $PACKAGE_NAME
pkgbase = $PACKAGE_NAME
pkgver = $VERSION-1
pkgdesc = Local-first file library with tags, notes, and ZIP backups
url = https://forgetag.noirlang.tr
builddate = $(date -u +%s)
packager = noirLang
size = $installed_size
arch = x86_64
license = AGPL-3.0-or-later
depend = gtk3
depend = webkit2gtk-4.1
EOF

mkdir -p "$ARCH_OUT_DIR"
(
  cd "$STAGE_DIR"
  bsdtar --format=gnutar --uid 0 --gid 0 --uname root --gname root -cf - .PKGINFO usr \
    | zstd -f -19 -T0 -o "$ARCH_OUT_DIR/forgetag-linux-x64.pkg.tar.zst"
)

echo "Arch Linux package written to $ARCH_OUT_DIR/forgetag-linux-x64.pkg.tar.zst"
