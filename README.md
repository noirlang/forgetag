<p align="center">
  <img src="./logo.png" alt="forgetag logo" width="128" />
</p>

# forgetag

Created by **noirLang**.

`forgetag` is a local-first desktop library for collecting files, folders,
archives, tags, dates, and short notes without moving or deleting the original
source files. Added items are copied into a managed local library and stored
with readable metadata.

The app is built with Tauri v2, React, TypeScript, TailwindCSS, and Rust.

## Features

- Add files, folders, and archives.
- Drag and drop a file, folder, or archive into the Add screen.
- Save title, date, tags, and description while adding an item.
- Copy added sources into `~/forgetag-library`.
- Search items by title, path, tags, description, and date.
- Filter items by existing tags.
- Preview selected item metadata.
- Open the managed item location from the app.
- Export the managed library as a ZIP archive.
- Import a previously exported ZIP archive.
- Use a simple settings screen for library import/export.
- Open the About screen with noirLang project information.
- Check releases from `github.com/noirlang/forgetag`.

## Repository Layout

```text
apps/desktop/          React + Vite frontend
src-tauri/             Tauri v2 Rust desktop host
crates/forgetag-core/  shared domain boundary
crates/forgetag-db/    future database boundary
crates/forgetag-ipc/   shared IPC types
crates/forgetag-*      planned indexing, search, AI, plugin modules
```

## Development

Install dependencies:

```bash
npm install
```

Run the frontend:

```bash
npm run dev
```

Run the desktop app:

```bash
npm run desktop
```

Build the frontend:

```bash
npm run build
```

Check Rust:

```bash
cargo check --workspace
```

## Linux Packages

Tauri builds Linux packages under `target/release/bundle/`.

Expected release asset names:

```text
forgetag_0.0.2_amd64.AppImage
forgetag_0.0.2_amd64.deb
forgetag-0.0.2-1.x86_64.rpm
```

The AppImage may need to be built from the generated AppDir on some rolling
Linux systems when `linuxdeploy` fails on newer system libraries.

## License

AGPL-3.0-or-later.
