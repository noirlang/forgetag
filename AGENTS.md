# Repository Guidelines

## Project Structure & Module Organization

`forgetag` is a Tauri v2 desktop app with a React frontend and Rust backend.
Frontend code lives in `apps/desktop/src/`; styles are in
`apps/desktop/src/styles.css`, and app assets are under
`apps/desktop/src/assets/`. The Tauri host, IPC commands, icons, and desktop
configuration are in `src-tauri/`. Planned Rust modules are split under
`crates/forgetag-*` for core, database, indexing, search, AI, IPC, and plugin
boundaries. Keep new behavior close to the crate or app layer that owns it.

## Build, Test, and Development Commands

- `npm install`: install workspace dependencies.
- `npm run dev`: run the Vite frontend only.
- `npm run desktop`: run the Tauri desktop app.
- `npm run build`: type-check and build the frontend.
- `cargo check --workspace`: check all Rust crates without producing packages.
- `npx tauri build --config src-tauri/tauri.conf.json`: create release bundles
  only when packaging is explicitly needed.
- `npm run build:arch`: build Arch Linux package from existing release binary.
- `npm run ci:linux`: full Linux CI build (AppImage, DEB, RPM, Arch) inside
  Debian 12 Docker container.
- `npm run build:windows:msi`: build Windows MSI via Tauri + WiX.

## CI/CD Pipeline

The `.github/workflows/ci.yml` GitHub Actions workflow:
- **metadata**: generates asset naming from commit message.
- **quality**: runs `cargo fmt --check`, `cargo test --locked`, `npm run build`.
- **linux-packages**: builds AppImage, DEB, RPM, and Arch `.pkg.tar.zst` inside
  Debian 12 Docker for glibc compatibility.
- **windows-msi**: builds MSI on `windows-2022` with WiX Toolset.
- **release**: publishes a prerelease on every push to `main` with all assets
  and SHA256 checksums.

Website (`website/`) is a Nuxt 3 static site deployed separately and not tracked
in the root repository (`.gitignore` excludes it). The live site is at
`forgetag.noirlang.tr`.

## Coding Style & Naming Conventions

Use TypeScript, React function components, and explicit domain types for IPC
payloads. Keep UI copy short and user-facing; avoid internal architecture text
inside the app. Use Rust 2021 style with `rustfmt`, `serde` DTOs at IPC
boundaries, and `Result<T, String>` only for Tauri command surfaces. File and
directory names should stay lowercase and kebab-case where practical.

## Testing Guidelines

There is no full test suite yet. Before shipping changes, run `npm run build`
for frontend changes and `cargo check --workspace` for Rust changes. Add focused
tests when introducing parsing, archive import/export, metadata migration, or
filesystem behavior. Prefer deterministic fixtures under the relevant crate or
feature directory.

## Commit & Pull Request Guidelines

Current history starts with a simple initial commit, so use clear imperative
messages such as `Add library ZIP import` or `Fix Linux icon packaging`. Pull
requests should include a short summary, changed screens when UI is touched,
verification commands, and any migration or data compatibility notes.

## Security & Configuration Tips

Do not add telemetry by default. Files are copied into the managed library, not
moved from their source path. Validate archive paths during import and keep
Tauri IPC commands narrow, typed, and permission-aware.
