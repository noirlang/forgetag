#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${FORGETAG_BUILDER_IMAGE:-forgetag-builder:latest}"
DOCKER_BIN="${DOCKER:-docker}"

if [[ "${FORGETAG_IN_UBUNTU_APPIMAGE_BUILD:-}" != "1" ]]; then
  # Check if docker image exists, if not build it
  if ! ${DOCKER_BIN} image inspect "${IMAGE}" >/dev/null 2>&1; then
    echo "Docker image ${IMAGE} not found. Building it..."
    ${DOCKER_BIN} build -t "${IMAGE}" -f "${ROOT_DIR}/scripts/Dockerfile.builder" "${ROOT_DIR}"
  fi

  # Create persistent volumes for caching cargo and npm packages
  ${DOCKER_BIN} volume create forgetag-cargo-registry >/dev/null 2>&1 || true
  ${DOCKER_BIN} volume create forgetag-cargo-git >/dev/null 2>&1 || true
  ${DOCKER_BIN} volume create forgetag-npm-cache >/dev/null 2>&1 || true

  echo "Starting AppImage build container..."
  exec ${DOCKER_BIN} run --rm \
    -e HOST_UID="$(id -u)" \
    -e HOST_GID="$(id -g)" \
    -e FORGETAG_IN_UBUNTU_APPIMAGE_BUILD=1 \
    -e APPIMAGE_EXTRACT_AND_RUN=1 \
    -v "${ROOT_DIR}:/work" \
    -v forgetag-cargo-registry:/root/.cargo/registry \
    -v forgetag-cargo-git:/root/.cargo/git \
    -v forgetag-npm-cache:/root/.npm \
    -w /work \
    "${IMAGE}" \
    bash scripts/build-appimage-ubuntu.sh
fi

trap 'chown -R "${HOST_UID:-1000}:${HOST_GID:-1000}" /work/target /work/apps/desktop/dist /work/node_modules /work/apps/desktop/node_modules 2>/dev/null || true' EXIT

# Inside the container, check if packages/compilers are present.
# If they are not (fallback / direct run outside builder container), install them.
if ! command -v node >/dev/null 2>&1 || ! command -v rustc >/dev/null 2>&1; then
  echo "Installing compiler tools, Node.js and Rust (fallback)..."
  export DEBIAN_FRONTEND=noninteractive
  apt-get update
  apt-get install -y \
    apt-transport-https \
    appstream \
    build-essential \
    ca-certificates \
    curl \
    desktop-file-utils \
    file \
    gnupg \
    libayatana-appindicator3-dev \
    libfuse2 \
    libgtk-3-dev \
    librsvg2-dev \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    patchelf \
    pkg-config \
    wget \
    xdg-utils \
    xz-utils

  if ! command -v node >/dev/null 2>&1; then
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
    apt-get install -y nodejs
  fi

  if ! command -v rustc >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  fi
fi

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  . "${HOME}/.cargo/env"
fi

# Use npm install with caching if node_modules already exists to avoid fresh re-download
if [[ -d "node_modules" ]]; then
  echo "Node modules folder exists, updating dependencies with caching..."
  npm install --prefer-offline --no-audit
else
  echo "Fresh install of node dependencies..."
  npm ci
fi
npx tauri build --config src-tauri/tauri.conf.json --bundles appimage

APPDIR="target/release/bundle/appimage/forgetag.AppDir"
APPIMAGE=$(find target/release/bundle/appimage -maxdepth 1 -name "*.AppImage" -not -name "*.tauri" | head -n 1)

if [[ -z "${APPIMAGE}" ]]; then
  echo "Error: No AppImage found in target/release/bundle/appimage" >&2
  exit 1
fi

DESKTOP_PATH="usr/share/applications/forgetag.desktop"
TAURI_APPIMAGE="${APPIMAGE}.tauri"
TAURI_RUNTIME="/tmp/forgetag-tauri-appimage-runtime"

# Ensure all icon sizes are installed in AppDir
for size in 16x16 32x32 64x64 128x128 256x256 512x512; do
  if [[ -f "src-tauri/icons/${size}.png" ]]; then
    install -Dm644 \
      "src-tauri/icons/${size}.png" \
      "${APPDIR}/usr/share/icons/hicolor/${size}/apps/forgetag.png"
  fi
done

# Find the largest icon available in the AppDir hicolor theme dynamically
LARGEST_ICON=""
for size in 1024x1024 512x512 256x256 128x128 64x64 32x32 16x16; do
  if [[ -f "${APPDIR}/usr/share/icons/hicolor/${size}/apps/forgetag.png" ]]; then
    LARGEST_ICON="usr/share/icons/hicolor/${size}/apps/forgetag.png"
    break
  fi
done

if [[ -z "${LARGEST_ICON}" ]]; then
  echo "Error: No app icon found in AppDir hicolor theme!" >&2
  exit 1
fi
ICON_PATH="${LARGEST_ICON}"
echo "Using largest icon: ${ICON_PATH}"

sed -i 's/^Icon=.*/Icon=forgetag/' "${APPDIR}/${DESKTOP_PATH}"
if grep -q '^StartupWMClass=' "${APPDIR}/${DESKTOP_PATH}"; then
  sed -i 's/^StartupWMClass=.*/StartupWMClass=forgetag/' "${APPDIR}/${DESKTOP_PATH}"
else
  printf 'StartupWMClass=forgetag\n' >> "${APPDIR}/${DESKTOP_PATH}"
fi
cp -f --remove-destination "${APPDIR}/${DESKTOP_PATH}" "${APPDIR}/forgetag.desktop"
cp -f --remove-destination "${APPDIR}/${ICON_PATH}" "${APPDIR}/forgetag.png"
cp -f --remove-destination "${APPDIR}/${ICON_PATH}" "${APPDIR}/.DirIcon"
desktop-file-validate "${APPDIR}/${DESKTOP_PATH}"

mv "${APPIMAGE}" "${TAURI_APPIMAGE}"
APPIMAGE_OFFSET="$(APPIMAGELAUNCHER_DISABLE=1 "${TAURI_APPIMAGE}" --appimage-offset)"
head -c "${APPIMAGE_OFFSET}" "${TAURI_APPIMAGE}" > "${TAURI_RUNTIME}"

if command -v appimagetool >/dev/null 2>&1; then
  APPIMAGETOOL="appimagetool"
else
  APPIMAGETOOL="/tmp/appimagetool-x86_64.AppImage"
  if [[ ! -f "${APPIMAGETOOL}" ]]; then
    echo "Downloading appimagetool..."
    curl -L -o "${APPIMAGETOOL}" \
      https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "${APPIMAGETOOL}"
  fi
fi

ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 APPIMAGELAUNCHER_DISABLE=1 "${APPIMAGETOOL}" \
  --runtime-file "${TAURI_RUNTIME}" \
  "${APPDIR}" \
  "${APPIMAGE}"

ls -lh "${APPIMAGE}"
file "${APPIMAGE}"
APPIMAGELAUNCHER_DISABLE=1 "${APPIMAGE}" --appimage-version
rm -f "${TAURI_APPIMAGE}"
