import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  Archive,
  CalendarDays,
  Check,
  Download,
  ExternalLink,
  FilePlus2,
  Folder,
  FolderPlus,
  Github,
  Info,
  Plus,
  RefreshCw,
  Search,
  Settings,
  Tags,
  Upload,
  X,
} from "lucide-react";
import logoUrl from "./assets/logo.png";
import "./styles.css";

const queryClient = new QueryClient();
const CURRENT_VERSION = "0.0.2";
const CREATOR_URL = "https://github.com/noirlang";
const UPDATE_REPO = "noirlang/forgetag";
const UPDATE_REPO_URL = `https://github.com/${UPDATE_REPO}`;

type ImportKind = "file" | "folder" | "archive";
type ActivePanel = "library" | "search" | "add" | "tags" | "settings" | "info";

type ImportDraft = {
  kind: ImportKind;
  source: string;
  title: string;
  date: string;
  tags: string;
  description: string;
};

type ImportedItem = ImportDraft & {
  id: string;
  managedPath: string;
};

type UpdateState = {
  status: "idle" | "checking" | "latest" | "available" | "error";
  message: string;
  version?: string;
  url?: string;
  assetName?: string;
};

type UpdateTarget = {
  os: string;
  arch: string;
  packageKind: "appimage" | "deb" | "rpm" | "dmg" | "msi" | "unknown";
  extension: string;
};

type GitHubReleaseAsset = {
  name?: string;
  browser_download_url?: string;
};

type GitHubRelease = {
  tag_name?: string;
  html_url?: string;
  assets?: GitHubReleaseAsset[];
};

type LibraryTransferResult = {
  path: string;
  itemCount: number;
};

const importLabels: Record<ImportKind, string> = {
  file: "File",
  folder: "Folder",
  archive: "Archive",
};

const archiveExtensions = new Set(["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "tgz"]);

function todayIso() {
  return new Date().toISOString().slice(0, 10);
}

function fileNameFromPath(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function createDraft(kind: ImportKind, source: string): ImportDraft {
  const name = fileNameFromPath(source);

  return {
    kind,
    source,
    title: name.replace(/\.[^.]+$/, ""),
    date: todayIso(),
    tags: "",
    description: "",
  };
}

function inferImportKindFromPath(path: string): ImportKind {
  const extension = fileNameFromPath(path).split(".").pop()?.toLowerCase() ?? "";
  return archiveExtensions.has(extension) ? "archive" : "file";
}

function parseTags(value: string) {
  return value
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean);
}

function normalizeVersion(version: string) {
  return version.trim().replace(/^v/i, "").split(/[+-]/)[0];
}

function compareVersions(left: string, right: string) {
  const leftParts = normalizeVersion(left).split(".").map((part) => Number.parseInt(part, 10) || 0);
  const rightParts = normalizeVersion(right).split(".").map((part) => Number.parseInt(part, 10) || 0);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < length; index += 1) {
    const leftPart = leftParts[index] ?? 0;
    const rightPart = rightParts[index] ?? 0;
    if (leftPart > rightPart) return 1;
    if (leftPart < rightPart) return -1;
  }

  return 0;
}

function debArch(arch: string) {
  if (arch === "x86_64") return "amd64";
  if (arch === "aarch64") return "arm64";
  return arch;
}

function rpmArch(arch: string) {
  if (arch === "aarch64") return "aarch64";
  if (arch === "x86_64") return "x86_64";
  return arch;
}

function linuxAssetNames(version: string, target: UpdateTarget) {
  const cleanVersion = normalizeVersion(version);
  const appImageName = `forgetag_${cleanVersion}_${debArch(target.arch)}.AppImage`;
  const debName = `forgetag_${cleanVersion}_${debArch(target.arch)}.deb`;
  const rpmName = `forgetag-${cleanVersion}-1.${rpmArch(target.arch)}.rpm`;
  const preferred = {
    appimage: [appImageName, debName, rpmName],
    deb: [debName, appImageName, rpmName],
    rpm: [rpmName, appImageName, debName],
    dmg: [appImageName, debName, rpmName],
    msi: [appImageName, debName, rpmName],
    unknown: [appImageName, debName, rpmName],
  };

  return preferred[target.packageKind] ?? preferred.unknown;
}

function pickReleaseAsset(assets: GitHubReleaseAsset[], version: string, target: UpdateTarget) {
  if (target.os !== "linux") return null;

  const names = linuxAssetNames(version, target).map((name) => name.toLowerCase());
  const exact = assets.find((asset) => asset.name && names.includes(asset.name.toLowerCase()));
  if (exact) return exact;

  const preferredExtension = target.extension.toLowerCase();
  return (
    assets.find((asset) => asset.name?.toLowerCase().endsWith(`.${preferredExtension}`)) ??
    assets.find((asset) => asset.name?.toLowerCase().endsWith(".appimage")) ??
    assets.find((asset) => asset.name?.toLowerCase().endsWith(".deb")) ??
    assets.find((asset) => asset.name?.toLowerCase().endsWith(".rpm")) ??
    null
  );
}

function BrandTitle({
  title,
  subtitle,
  size = "default",
}: {
  title: string;
  subtitle?: string;
  size?: "default" | "large";
}) {
  return (
    <div className={`brand-title ${size}`}>
      <img src={logoUrl} alt="" aria-hidden="true" />
      <span>
        <strong>{title}</strong>
        {subtitle ? <small>{subtitle}</small> : null}
      </span>
    </div>
  );
}

function App() {
  const fallbackInputRef = React.useRef<HTMLInputElement>(null);
  const fallbackKindRef = React.useRef<ImportKind>("file");
  const [activePanel, setActivePanel] = React.useState<ActivePanel>("library");
  const [draft, setDraft] = React.useState<ImportDraft | null>(null);
  const [importedItems, setImportedItems] = React.useState<ImportedItem[]>([]);
  const [status, setStatus] = React.useState("Ready.");
  const [isAdding, setIsAdding] = React.useState(false);
  const [isDragging, setIsDragging] = React.useState(false);
  const [isTransferring, setIsTransferring] = React.useState(false);
  const [searchQuery, setSearchQuery] = React.useState("");
  const [selectedTags, setSelectedTags] = React.useState<string[]>([]);
  const [selectedItemId, setSelectedItemId] = React.useState<string | null>(null);
  const [updateState, setUpdateState] = React.useState<UpdateState>({
    status: "idle",
    message: `Current version ${CURRENT_VERSION}.`,
  });

  const allTags = React.useMemo(() => {
    const tags = new Set<string>();

    for (const item of importedItems) {
      for (const tag of parseTags(item.tags)) {
        tags.add(tag);
      }
    }

    return [...tags].sort((a, b) => a.localeCompare(b));
  }, [importedItems]);

  const filteredItems = React.useMemo(() => {
    if (selectedTags.length === 0) return importedItems;

    return importedItems.filter((item) => {
      const itemTags = parseTags(item.tags);
      return selectedTags.every((tag) => itemTags.includes(tag));
    });
  }, [importedItems, selectedTags]);

  const visibleItems = React.useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) return filteredItems;

    return filteredItems.filter((item) =>
      [item.title, item.source, item.tags, item.description, item.date]
        .join(" ")
        .toLowerCase()
        .includes(query),
    );
  }, [filteredItems, searchQuery]);

  const selectedItem =
    importedItems.find((item) => item.id === selectedItemId) ?? visibleItems[0] ?? null;

  async function refreshItems() {
    try {
      const items = await invoke<ImportedItem[]>("list_items");
      setImportedItems(items);
      setSelectedItemId((current) =>
        current && items.some((item) => item.id === current) ? current : items[0]?.id ?? null,
      );
    } catch {
      // Browser preview cannot call Tauri commands.
    }
  }

  React.useEffect(() => {
    void refreshItems();
  }, []);

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;

    void import("@tauri-apps/api/webview")
      .then(({ getCurrentWebview }) =>
        getCurrentWebview().onDragDropEvent((event) => {
          if (event.payload.type === "enter" || event.payload.type === "over") {
            setIsDragging(true);
            setActivePanel("add");
            return;
          }

          if (event.payload.type === "leave") {
            setIsDragging(false);
            return;
          }

          if (event.payload.type === "drop") {
            setIsDragging(false);
            const [path] = event.payload.paths;
            if (path) {
              void useDroppedPath(path);
            }
          }
        }),
      )
      .then((cleanup) => {
        unlisten = cleanup;
      })
      .catch(() => {
        // Browser preview cannot access Tauri drag/drop events.
      });

    return () => {
      unlisten?.();
    };
  }, []);

  function openPanel(panel: ActivePanel) {
    setActivePanel((current) => (current === panel ? "library" : panel));
  }

  async function chooseImport(kind: ImportKind) {
    setActivePanel("add");
    fallbackKindRef.current = kind;
    setStatus(`Selecting ${importLabels[kind].toLowerCase()}...`);

    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selection = await open({
        multiple: false,
        directory: kind === "folder",
        filters:
          kind === "archive"
            ? [
                {
                  name: "Archives",
                  extensions: ["zip", "rar", "7z", "tar", "gz", "bz2", "xz"],
                },
              ]
            : undefined,
      });

      if (!selection) {
        setStatus("Selection cancelled.");
        return;
      }

      const source = Array.isArray(selection) ? selection[0] : selection;
      setDraft(createDraft(kind, source));
      setStatus(`${importLabels[kind]} selected.`);
    } catch {
      const input = fallbackInputRef.current;
      if (!input) return;

      input.value = "";
      input.accept = kind === "archive" ? ".zip,.rar,.7z,.tar,.tar.gz,.tgz,.gz,.bz2,.xz" : "";
      if (kind === "folder") {
        input.setAttribute("webkitdirectory", "");
      } else {
        input.removeAttribute("webkitdirectory");
      }
      input.click();
    }
  }

  function handleFallbackSelection(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) {
      setStatus("Selection cancelled.");
      return;
    }

    const source = file.webkitRelativePath || file.name;
    const kind = fallbackKindRef.current;
    setActivePanel("add");
    setDraft(createDraft(kind, source));
    setStatus(`${importLabels[kind]} selected.`);
  }

  async function useDroppedPath(path: string) {
    const kind = await invoke<ImportKind>("classify_import_path", { path }).catch(() =>
      inferImportKindFromPath(path),
    );

    setActivePanel("add");
    setDraft(createDraft(kind, path));
    setStatus(`${importLabels[kind]} dropped.`);
  }

  function updateDraft(field: keyof ImportDraft, value: string) {
    setDraft((current) => (current ? { ...current, [field]: value } : current));
  }

  function cancelDraft() {
    setDraft(null);
    setStatus("Import cancelled.");
  }

  async function addDraft() {
    if (!draft) return;

    setIsAdding(true);
    setStatus("Copying...");

    try {
      const item = await invoke<ImportedItem>("import_item", {
        request: {
          ...draft,
          title: draft.title.trim() || fileNameFromPath(draft.source),
        },
      });

      setImportedItems((items) => [item, ...items]);
      setSelectedItemId(item.id);
      setDraft(null);
      setActivePanel("library");
      setStatus(`${importLabels[item.kind]} added.`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setIsAdding(false);
    }
  }

  function toggleTag(tag: string) {
    setSelectedTags((current) =>
      current.includes(tag) ? current.filter((item) => item !== tag) : [...current, tag],
    );
  }

  function clearTagFilter() {
    setSelectedTags([]);
  }

  async function openItem(itemId: string) {
    const item = importedItems.find((item) => item.id === itemId);
    if (!item) return;

    setSelectedItemId(item.id);
    try {
      await invoke("reveal_item", { path: item.managedPath });
      setStatus("Folder opened.");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function openExternalUrl(url: string) {
    try {
      await invoke("open_external_url", { url });
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    }
  }

  async function exportLibraryArchive() {
    setIsTransferring(true);
    setStatus("Exporting...");

    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const destination = await save({
        defaultPath: `forgetag-library-${todayIso()}.zip`,
        filters: [{ name: "ZIP archive", extensions: ["zip"] }],
      });

      if (!destination) {
        setStatus("Export cancelled.");
        return;
      }

      const result = await invoke<LibraryTransferResult>("export_library", { destination });
      setStatus(`Exported ${result.itemCount} items.`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setIsTransferring(false);
    }
  }

  async function importLibraryArchive() {
    setIsTransferring(true);
    setStatus("Importing...");

    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selection = await open({
        multiple: false,
        filters: [{ name: "ZIP archive", extensions: ["zip"] }],
      });

      if (!selection) {
        setStatus("Import cancelled.");
        return;
      }

      const source = Array.isArray(selection) ? selection[0] : selection;
      const result = await invoke<LibraryTransferResult>("import_library_archive", { source });
      await refreshItems();
      setActivePanel("library");
      setStatus(`Imported ${result.itemCount} items.`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setIsTransferring(false);
    }
  }

  async function checkForUpdates() {
    setUpdateState({
      status: "checking",
      message: "Checking GitHub...",
    });
    setStatus("Checking updates...");

    try {
      let latestVersion = "";
      let latestUrl = UPDATE_REPO_URL;
      let releaseAssets: GitHubReleaseAsset[] = [];

      const updateTarget = await invoke<UpdateTarget>("update_target").catch(() => ({
        os: "linux",
        arch: "x86_64",
        packageKind: "appimage" as const,
        extension: "AppImage",
      }));

      const releaseResponse = await fetch(`https://api.github.com/repos/${UPDATE_REPO}/releases/latest`, {
        headers: { Accept: "application/vnd.github+json" },
      });

      if (releaseResponse.ok) {
        const release = (await releaseResponse.json()) as GitHubRelease;
        latestVersion = release.tag_name ?? "";
        latestUrl = release.html_url ?? latestUrl;
        releaseAssets = release.assets ?? [];
      } else {
        const tagResponse = await fetch(`https://api.github.com/repos/${UPDATE_REPO}/tags?per_page=1`, {
          headers: { Accept: "application/vnd.github+json" },
        });

        if (!tagResponse.ok) {
          throw new Error("GitHub update check failed.");
        }

        const tags = (await tagResponse.json()) as Array<{
          name?: string;
          commit?: { url?: string };
        }>;
        latestVersion = tags[0]?.name ?? "";
        latestUrl = latestVersion ? `${UPDATE_REPO_URL}/releases/tag/${latestVersion}` : latestUrl;
      }

      if (!latestVersion) {
        throw new Error("No release or tag found.");
      }

      if (compareVersions(latestVersion, CURRENT_VERSION) > 0) {
        const asset = pickReleaseAsset(releaseAssets, latestVersion, updateTarget);
        const downloadUrl = asset?.browser_download_url ?? latestUrl;
        const assetName = asset?.name;

        setUpdateState({
          status: "available",
          message: assetName
            ? `Update available: ${latestVersion} (${assetName}).`
            : `Update available: ${latestVersion}. Release asset not found, opening release page.`,
          version: latestVersion,
          url: downloadUrl,
          assetName,
        });
        setStatus("Update available.");
        return;
      }

      setUpdateState({
        status: "latest",
        message: `You are on the latest version: ${CURRENT_VERSION}.`,
        version: latestVersion,
        url: latestUrl,
      });
      setStatus("No update found.");
    } catch (error) {
      setUpdateState({
        status: "error",
        message: error instanceof Error ? error.message : String(error),
      });
      setStatus("Update check failed.");
    }
  }

  function renderWorkspace() {
    if (activePanel === "search") {
      return (
        <section className="detail-card search-panel" aria-label="Search">
          <div className="detail-header">
            <BrandTitle title="Search" />
          </div>
          <label className="search-box">
            <Search size={17} aria-hidden="true" />
            <input
              autoFocus
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="Search files, tags, notes..."
            />
          </label>
          <ItemList items={visibleItems} selectedItemId={selectedItem?.id ?? null} onSelect={openItem} />
        </section>
      );
    }

    if (activePanel === "tags") {
      return (
        <section className="detail-card" aria-label="Tags">
          <div className="detail-header row-title">
            <BrandTitle title="Tags" />
            {selectedTags.length > 0 ? (
              <button onClick={clearTagFilter}>Clear</button>
            ) : null}
          </div>
          {allTags.length === 0 ? (
            <div className="simple-empty">
              <p>No tags yet.</p>
            </div>
          ) : (
            <>
              <div className="tag-row">
                {allTags.map((tag) => (
                  <button
                    key={tag}
                    className={selectedTags.includes(tag) ? "selected" : undefined}
                    onClick={() => toggleTag(tag)}
                  >
                    {tag}
                  </button>
                ))}
              </div>
              <ItemList items={visibleItems} selectedItemId={selectedItem?.id ?? null} onSelect={openItem} />
            </>
          )}
        </section>
      );
    }

    if (activePanel === "add") {
      return (
        <section className="detail-card import-board" aria-label="Add">
          <div className="detail-header import-header">
            <BrandTitle title="Add" />
            <div className="import-actions" aria-label="Import actions">
              <button onClick={() => chooseImport("file")}>
                <FilePlus2 size={17} aria-hidden="true" />
                File
              </button>
              <button onClick={() => chooseImport("folder")}>
                <FolderPlus size={17} aria-hidden="true" />
                Folder
              </button>
              <button onClick={() => chooseImport("archive")}>
                <Archive size={17} aria-hidden="true" />
                Archive
              </button>
            </div>
          </div>

          <section className={`drop-zone ${isDragging ? "dragging" : ""}`} aria-label="Drop import target">
            <Upload size={22} aria-hidden="true" />
            <span>
              <strong>{isDragging ? "Drop to add" : "Drop file, folder, or archive"}</strong>
              <small>Metadata opens after selection.</small>
            </span>
          </section>

          {draft ? (
            <section className="metadata-panel" aria-label="Import metadata">
              <div className="selected-source">
                <span>{importLabels[draft.kind]}</span>
                <strong>{fileNameFromPath(draft.source)}</strong>
                <small>{draft.source}</small>
              </div>
              <div className="metadata-grid">
                <label>
                  Title
                  <input value={draft.title} onChange={(event) => updateDraft("title", event.target.value)} />
                </label>
                <label>
                  Date
                  <span className="date-input">
                    <CalendarDays size={16} aria-hidden="true" />
                    <input
                      value={draft.date}
                      onChange={(event) => updateDraft("date", event.target.value)}
                      placeholder="2026-05-24"
                    />
                  </span>
                </label>
                <label className="wide">
                  Tags
                  <input
                    value={draft.tags}
                    onChange={(event) => updateDraft("tags", event.target.value)}
                    placeholder="work, screenshot, archive"
                  />
                </label>
                <label className="wide">
                  Description
                  <textarea
                    value={draft.description}
                    onChange={(event) => updateDraft("description", event.target.value)}
                    placeholder="Short note"
                  />
                </label>
              </div>
              <div className="form-actions">
                <button className="secondary-action" onClick={cancelDraft}>
                  <X size={16} aria-hidden="true" />
                  Cancel
                </button>
                <button className="primary-action" disabled={isAdding} onClick={addDraft}>
                  <Check size={16} aria-hidden="true" />
                  {isAdding ? "Copying" : "Add"}
                </button>
              </div>
            </section>
          ) : (
            <section className="import-empty" aria-label="Import empty state">
              <h3>Choose something to add</h3>
            </section>
          )}
        </section>
      );
    }

    if (activePanel === "settings") {
      return (
        <section className="detail-card settings-panel" aria-label="Settings">
          <div className="detail-header">
            <BrandTitle title="Settings" />
          </div>

          <section className="settings-stack">
            <article className="settings-card">
              <div>
                <strong>Library backup</strong>
                <span>Export or restore the managed library as a ZIP archive.</span>
              </div>
              <div className="settings-actions">
                <button className="secondary-action" disabled={isTransferring} onClick={exportLibraryArchive}>
                  <Download size={16} aria-hidden="true" />
                  Export ZIP
                </button>
                <button className="primary-action" disabled={isTransferring} onClick={importLibraryArchive}>
                  <Upload size={16} aria-hidden="true" />
                  Import ZIP
                </button>
              </div>
            </article>
          </section>
        </section>
      );
    }

    if (activePanel === "info") {
      return (
        <section className="detail-card info-panel" aria-label="Info">
          <div className="detail-header about-header">
            <BrandTitle
              title="forgetag"
              subtitle="Local-first knowledge library by noirLang."
              size="large"
            />
            <button className="secondary-action" onClick={() => openExternalUrl(UPDATE_REPO_URL)}>
              <Github size={16} aria-hidden="true" />
              Repository
            </button>
          </div>
          <section className="about-stack">
            <div className="about-hero">
              <img className="about-logo" src={logoUrl} alt="forgetag logo" />
              <span>Version {CURRENT_VERSION}</span>
              <h3>Local-first desktop library for developers, designers, and technical users.</h3>
              <p>
                forgetag gives you one clean place to collect files, folders, archives, tags,
                short notes, dates, and searchable metadata without giving up ownership of your
                data.
              </p>
            </div>

            <div className="about-grid">
              <article>
                <strong>Local and private</strong>
                <p>
                  The app is built as a privacy-focused knowledge layer over the local machine.
                  It does not require an account, does not require cloud sync, and should remain
                  fully useful offline.
                </p>
              </article>
              <article>
                <strong>Managed library</strong>
                <p>
                  Added files, folders, and archives are copied into the local forgetag library.
                  The original source is not deleted or moved.
                </p>
              </article>
              <article>
                <strong>Searchable metadata</strong>
                <p>
                  Titles, tags, dates, descriptions, source paths, and managed paths are kept as
                  readable metadata now, with SQLite planned as the next storage milestone.
                </p>
              </article>
              <article>
                <strong>Long-term direction</strong>
                <p>
                  The roadmap includes SQLite, full-text search, richer tags, relationships,
                  local AI, OCR, semantic search, and plugin support.
                </p>
              </article>
            </div>

            <div className="about-actions">
              <button className="secondary-action" onClick={() => openExternalUrl(CREATOR_URL)}>
                <Github size={16} aria-hidden="true" />
                noirLang
                <ExternalLink size={14} aria-hidden="true" />
              </button>
              <button
                className="secondary-action"
                disabled={updateState.status === "checking"}
                onClick={checkForUpdates}
              >
                <RefreshCw size={16} aria-hidden="true" />
                {updateState.status === "checking" ? "Checking" : "Check for updates"}
              </button>
              {updateState.status === "available" && updateState.url ? (
                <button className="primary-action" onClick={() => openExternalUrl(updateState.url!)}>
                  Download update
                  <ExternalLink size={14} aria-hidden="true" />
                </button>
              ) : null}
            </div>

            <p className={`update-message ${updateState.status}`}>{updateState.message}</p>
          </section>
        </section>
      );
    }

    return (
      <section className="detail-card library-panel" aria-label="Library">
        <div>
          <BrandTitle title="forgetag" />
          <button onClick={() => setActivePanel("add")}>
            <Plus size={18} aria-hidden="true" />
            Add
          </button>
        </div>
        <ItemList items={visibleItems} selectedItemId={selectedItem?.id ?? null} onSelect={openItem} />
      </section>
    );
  }

  return (
    <QueryClientProvider client={queryClient}>
      <main className="app-shell min-h-screen text-slate-100">
        <input
          ref={fallbackInputRef}
          className="hidden-file-input"
          type="file"
          onChange={handleFallbackSelection}
        />
        <aside className="activity-bar shrink-0" aria-label="Primary navigation">
          <button
            aria-label="forgetag"
            className="brand-mark"
            onClick={() => setActivePanel("library")}
          >
            <img src={logoUrl} alt="" aria-hidden="true" />
          </button>
          <button
            aria-label="Library"
            aria-pressed={activePanel === "library"}
            className={activePanel === "library" ? "active" : undefined}
            onClick={() => openPanel("library")}
          >
            <Folder size={18} aria-hidden="true" />
          </button>
          <button
            aria-label="Search"
            aria-pressed={activePanel === "search"}
            className={activePanel === "search" ? "active" : undefined}
            onClick={() => openPanel("search")}
          >
            <Search size={18} aria-hidden="true" />
          </button>
          <button
            aria-label="Add"
            aria-pressed={activePanel === "add"}
            className={activePanel === "add" ? "active" : undefined}
            onClick={() => openPanel("add")}
          >
            <Plus size={19} aria-hidden="true" />
          </button>
          <button
            aria-label="Tags"
            aria-pressed={activePanel === "tags"}
            className={activePanel === "tags" ? "active" : undefined}
            onClick={() => openPanel("tags")}
          >
            <Tags size={18} aria-hidden="true" />
          </button>
          <button
            aria-label="Settings"
            aria-pressed={activePanel === "settings"}
            className={activePanel === "settings" ? "active" : undefined}
            onClick={() => openPanel("settings")}
          >
            <Settings size={18} aria-hidden="true" />
          </button>
          <span className="activity-spacer" aria-hidden="true" />
          <button
            aria-label="Info"
            aria-pressed={activePanel === "info"}
            className={activePanel === "info" ? "active" : undefined}
            onClick={() => openPanel("info")}
          >
            <Info size={18} aria-hidden="true" />
          </button>
        </aside>
        <section className="workspace min-w-0">{renderWorkspace()}</section>
        <aside className="inspector" aria-label="Preview and metadata">
          <div className="inspector-title">
            <img src={logoUrl} alt="" aria-hidden="true" />
            <h2>Preview</h2>
          </div>
          <p>{status}</p>
          <PreviewItem item={selectedItem} />
        </aside>
      </main>
    </QueryClientProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

function ItemList({
  compact = false,
  items,
  selectedItemId,
  onSelect,
}: {
  compact?: boolean;
  items: ImportedItem[];
  selectedItemId: string | null;
  onSelect: (id: string) => void;
}) {
  if (items.length === 0) {
    return (
      <div className={compact ? "pane-empty" : "simple-empty"}>
        <p>No items yet.</p>
      </div>
    );
  }

  return (
    <div className={compact ? "pane-list" : "item-list"}>
      {items.map((item) => (
        <button
          key={item.id}
          className={item.id === selectedItemId ? "selected" : undefined}
          onClick={() => onSelect(item.id)}
        >
          <span>
            <strong>{item.title}</strong>
            {item.tags ? <em>{item.tags}</em> : null}
          </span>
          <small>
            {importLabels[item.kind]} · {item.date}
          </small>
        </button>
      ))}
    </div>
  );
}

function PreviewItem({ item }: { item: ImportedItem | null }) {
  if (!item) {
    return (
      <div className="preview-empty">
        <span>No item selected.</span>
      </div>
    );
  }

  return (
    <div className="preview-card">
      <img className="preview-logo" src={logoUrl} alt="" aria-hidden="true" />
      <strong>{item.title}</strong>
      <small>
        {importLabels[item.kind]} · {item.date}
      </small>
      {item.tags ? <em>{item.tags}</em> : null}
      {item.description ? <p>{item.description}</p> : null}
      <code>{item.managedPath}</code>
    </div>
  );
}
