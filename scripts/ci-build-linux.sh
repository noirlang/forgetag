#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="/work"
LINUXDEPLOY="/tmp/linuxdeploy-x86_64.AppImage"
RUNTIME_FILE="/tmp/runtime-x86_64"
GLIBC_CEILING="2.35"

restore_output_owner() {
  if [[ -n "${HOST_UID:-}" && -n "${HOST_GID:-}" ]]; then
    chown -R "$HOST_UID:$HOST_GID" \
      "$ROOT_DIR/target" \
      "$ROOT_DIR/apps/desktop/dist" \
      "$ROOT_DIR/node_modules" \
      "$ROOT_DIR/apps/desktop/node_modules" \
      "$ROOT_DIR/dist" \
      2>/dev/null || true
  fi
}
trap restore_output_owner EXIT

export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  curl \
  file \
  git \
  imagemagick \
  libarchive-tools \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  perl \
  pkg-config \
  python3 \
  rpm \
  zstd \
  desktop-file-utils \
  appstream \
  libfuse2 \
  xdg-utils

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal --default-toolchain stable
export PATH="$HOME/.cargo/bin:$PATH"

curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
apt-get install -y nodejs

curl -fL \
  https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage \
  -o "$LINUXDEPLOY"
curl -fL \
  https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64 \
  -o "$RUNTIME_FILE"
chmod +x "$LINUXDEPLOY"

rm -rf /tmp/squashfs-root
(
  cd /tmp
  "$LINUXDEPLOY" --appimage-extract >/dev/null
)
EXTRACT_DIR="/tmp/squashfs-root"
LINUXDEPLOY_RUN="$EXTRACT_DIR/AppRun"

cd "$ROOT_DIR"

npm ci

npx tauri build --config src-tauri/tauri.conf.json --bundles appimage,deb,rpm

# ---- Post-process AppImage ----
APPDIR="target/release/bundle/appimage/forgetag.AppDir"
APPIMAGE=$(find target/release/bundle/appimage -maxdepth 1 -name "*.AppImage" ! -name "*.tauri" | head -n 1)

if [[ -n "$APPIMAGE" && -d "$APPDIR" ]]; then
  for size in 16x16 32x32 64x64 128x128 256x256 512x512; do
    if [[ -f "src-tauri/icons/${size}.png" ]]; then
      install -Dm644 \
        "src-tauri/icons/${size}.png" \
        "${APPDIR}/usr/share/icons/hicolor/${size}/apps/forgetag.png"
    fi
  done

  LARGEST_ICON=""
  for size in 512x512 256x256 128x128 64x64 32x32 16x16; do
    if [[ -f "${APPDIR}/usr/share/icons/hicolor/${size}/apps/forgetag.png" ]]; then
      LARGEST_ICON="usr/share/icons/hicolor/${size}/apps/forgetag.png"
      break
    fi
  done

  DESKTOP_PATH="usr/share/applications/forgetag.desktop"
  if [[ -n "$LARGEST_ICON" && -f "${APPDIR}/${DESKTOP_PATH}" ]]; then
    sed -i 's/^Icon=.*/Icon=forgetag/' "${APPDIR}/${DESKTOP_PATH}"
    if grep -q '^StartupWMClass=' "${APPDIR}/${DESKTOP_PATH}"; then
      sed -i 's/^StartupWMClass=.*/StartupWMClass=forgetag/' "${APPDIR}/${DESKTOP_PATH}"
    else
      printf 'StartupWMClass=forgetag\n' >> "${APPDIR}/${DESKTOP_PATH}"
    fi
    cp -f --remove-destination "${APPDIR}/${DESKTOP_PATH}" "${APPDIR}/forgetag.desktop"
    cp -f --remove-destination "${APPDIR}/${LARGEST_ICON}" "${APPDIR}/forgetag.png"
    cp -f --remove-destination "${APPDIR}/${LARGEST_ICON}" "${APPDIR}/.DirIcon"
    desktop-file-validate "${APPDIR}/${DESKTOP_PATH}" || true
  fi

  TAURI_APPIMAGE="${APPIMAGE}.tauri"
  mv "$APPIMAGE" "$TAURI_APPIMAGE"

  TAURI_RUNTIME="/tmp/forgetag-tauri-appimage-runtime"
  APPIMAGE_OFFSET="$(APPIMAGELAUNCHER_DISABLE=1 "$TAURI_APPIMAGE" --appimage-offset)"
  head -c "$APPIMAGE_OFFSET" "$TAURI_APPIMAGE" > "$TAURI_RUNTIME"

  APPIMAGETOOL="/tmp/appimagetool-x86_64.AppImage"
  if [[ ! -f "$APPIMAGETOOL" ]]; then
    curl -L -o "$APPIMAGETOOL" \
      https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$APPIMAGETOOL"
  fi

  ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 APPIMAGELAUNCHER_DISABLE=1 "$APPIMAGETOOL" \
    --runtime-file "$TAURI_RUNTIME" \
    "$APPDIR" \
    "$APPIMAGE"

  rm -f "$TAURI_APPIMAGE" "$TAURI_RUNTIME"

  if command -v readelf >/dev/null 2>&1; then
    temp_file=$(mktemp)
    find "$APPDIR" -type f -print0 | while IFS= read -r -d '' candidate; do
      if file -b "$candidate" 2>/dev/null | grep -q '^ELF'; then
        readelf --version-info "$candidate" 2>/dev/null \
          | grep -oE 'GLIBC_[0-9.]+' \
          | sed 's/^GLIBC_//' >> "$temp_file" || true
      fi
    done
    highest_glibc=$(sort -Vu "$temp_file" | tail -n1 || true)
    rm -f "$temp_file"

    if [[ -n "$highest_glibc" ]]; then
      if dpkg --compare-versions "$highest_glibc" gt "$GLIBC_CEILING" 2>/dev/null; then
        echo "Warning: AppImage requires GLIBC_$highest_glibc; ceiling is GLIBC_$GLIBC_CEILING" >&2
      else
        echo "AppImage GLIBC requirement verified: GLIBC_$highest_glibc"
      fi
    fi
  fi
fi

# ---- Build Arch Linux package ----
bash ./scripts/build-arch-pkg.sh
