FROM ubuntu:22.04

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install core utilities, compiler tools, and Tauri dependencies
RUN apt-get update && apt-get install -y \
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
  xz-utils \
  && rm -rf /var/lib/apt/lists/*

# Install Node.js v22
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
  && apt-get install -y nodejs \
  && rm -rf /var/lib/apt/lists/*

# Install Rustup and stable rust toolchain in a shared system location
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain stable \
  && chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# Install appimagetool (so we don't download it on every build run)
RUN curl -L -o /usr/local/bin/appimagetool \
  https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage \
  && chmod +x /usr/local/bin/appimagetool

WORKDIR /work
