<p align="center">
  <img src="./logo.png" alt="forgetag logo" width="140" />
</p>

# forgetag

Created by **noirLang**.

`forgetag` is a local-first desktop library for developers, designers, and
technical users who want one clean place to collect files, folders, archives,
tags, short notes, dates, and searchable metadata without giving up ownership of
their data.

The project is built as a privacy-focused knowledge layer over the local
machine. It is not a cloud service, it does not require an account, and it should
never move original files unless the user explicitly asks for that behavior.

## Current Status

`forgetag` is in early desktop prototype stage.

The current app already focuses on the core workflow:

- add a file
- add a folder
- add an archive such as ZIP, RAR, 7Z, TAR, GZ, BZ2, or XZ
- copy the selected source into a managed local library
- attach a title, date, tags, and description while adding it
- list added items
- filter by tags
- search by title, path, tags, description, and date
- preview metadata in a side panel
- open the containing folder of an added item

The implementation is intentionally being kept simple while the product shape is
still changing. The current persistence layer writes per-item metadata next to
managed library files. The next storage milestone is a SQLite-backed library
database for items, tags, relationships, search state, and import history.

## Product Goal

`forgetag` should become a fast local knowledge and asset operating system for:

- source code
- repositories
- markdown notes
- PDFs
- images
- screenshots
- videos
- design files
- PSD, Krita, and Blender projects
- archives
- documentation
- logs
- AI conversations
- snippets
- API collections
- Docker projects
- server configs
- Android bugreports
- research files
- downloaded files
- scripts
- binaries

The long-term goal is to feel like a local intelligence layer for a developer's
digital life: faster than a normal file manager, more structured than a folder
tree, and less fragile than a manual notes system.

## Core Principles

- Local first: the app must work fully offline.
- Privacy first: no telemetry by default.
- User-owned files: original files stay where they are unless the user chooses
  to copy or move them.
- Managed library mode: added sources can be copied into a local `forgetag`
  library folder.
- Open data: metadata must remain readable and exportable.
- Scalable architecture: the app should be designed for very large collections.
- Developer focused: code, repositories, logs, archives, and project assets are
  first-class use cases.
- AI ready: local AI features should be optional and modular, not required for
  the core app to work.

## What The App Does Now

### Library

The Library screen shows the current managed items. Each item can represent a
file, a folder, or an archive copied into the local library.

Clicking an item selects it and opens the folder where the managed copy lives.

### Add

The Add screen opens only when the Add button is clicked.

Supported add flows:

- File
- Folder
- Archive

After a source is selected, the app shows a compact metadata form:

- Title
- Date
- Tags
- Description

When Add is confirmed, the source is copied into the local managed library. The
original source is not deleted or moved.

### Tags

The Tags screen lists tags that already exist on added items.

Selecting tags filters the library immediately.

### Search

The Search screen searches across:

- title
- original source path
- managed path
- tags
- description
- date

The search UI is intentionally separate from the Library screen so the main
layout stays clean.

### Preview

The right preview panel shows the selected item's metadata:

- title
- type
- date
- tags
- description
- managed path
- latest action/status

## Planned Storage Model

The intended storage direction is SQLite first.

SQLite should store:

- items
- tags
- item-tag links
- descriptions
- dates
- original source paths
- managed paths
- import history
- relationships
- saved searches
- preview state
- future indexing state

The codebase is structured so PostgreSQL can be supported later behind the same
repository/service boundaries.

## Planned Search Model

Search should evolve in stages:

1. Basic in-memory filtering for the early prototype.
2. SQLite-backed search and filtering.
3. Tantivy-backed full-text search.
4. Faceted search by type, tag, date, extension, and project.
5. Optional semantic search through local embeddings.

Example future queries:

```text
tag:linux ext:pdf
type:image tag:logo
archive:7z date:2026-05-24
project:kernel has:description
```

## Planned AI Direction

AI is optional and local-first.

Future AI features should be built around:

- Ollama
- llama.cpp
- local embedding models
- OCR
- image captioning
- auto-tagging
- duplicate detection
- clustering
- summarization
- local RAG over indexed files

The base app must remain useful without any AI runtime installed.

## Tech Stack

Frontend:

- Tauri v2
- TypeScript
- React
- Vite
- TailwindCSS
- Zustand
- TanStack Query
- Framer Motion
- lucide-react

Backend:

- Rust
- Tokio-ready architecture
- Serde
- Tauri commands
- modular Rust crates

Storage direction:

- SQLite first
- PostgreSQL-compatible architecture later

Search direction:

- Tantivy planned
- semantic search planned

## Repository Layout

```text
forgetag/
  apps/
    desktop/              React + Vite desktop frontend
  crates/
    forgetag-core/        domain types and shared application contracts
    forgetag-db/          database boundary for SQLite/PostgreSQL work
    forgetag-indexer/     indexing pipeline boundary
    forgetag-search/      search query and backend boundary
    forgetag-code/        future git and Tree-sitter code intelligence
    forgetag-ai/          future local AI provider contracts
    forgetag-plugin/      future plugin manifest and capability model
    forgetag-ipc/         IPC DTOs shared with the Tauri layer
  src-tauri/              Tauri v2 Rust host
```

There is intentionally no separate documentation folder right now. This README
is the main project document.

## Development

Install dependencies:

```bash
npm install
```

Run the frontend only:

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

On Linux, the desktop script currently starts Tauri with X11-oriented
environment flags because that path is more reliable in the current development
environment.

## Naming

- Project name: `forgetag`
- Repository slug: `forgetag`
- Rust crate prefix: `forgetag-*`
- Binary name: `forgetag`
- Creator: `noirLang`

## Roadmap

Short-term:

- replace prototype metadata persistence with SQLite
- add real item, tag, and relationship tables
- improve add/edit/delete flows
- add better file previews
- add duplicate-safe import handling
- add archive extraction metadata
- add reliable import error states

Mid-term:

- add full-text search
- add saved searches
- add richer tag management
- add relationship graph basics
- add repository-aware indexing
- add code symbol extraction
- add filesystem watching

Long-term:

- add plugin APIs
- add local AI pipelines
- add OCR
- add image tagging
- add semantic search
- add portable library export/import
- add encrypted metadata support

## License

AGPL-3.0-or-later.

<p align="center">
  <img src="./logo.png" alt="forgetag logo" width="96" />
</p>
