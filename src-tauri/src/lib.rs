use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    fs::File,
    io::{self, Seek, Write},
    path::Component,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

const APP_ICON: &[u8] = include_bytes!("../icons/icon.png");

#[tauri::command]
fn app_name() -> &'static str {
    "forgetag"
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    kind: String,
    source: String,
    title: String,
    date: String,
    tags: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportResponse {
    id: String,
    kind: String,
    source: String,
    title: String,
    date: String,
    tags: String,
    description: String,
    managed_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTarget {
    os: String,
    arch: String,
    package_kind: String,
    extension: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryTransferResponse {
    path: String,
    item_count: usize,
}

#[tauri::command]
fn import_item(request: ImportRequest) -> Result<ImportResponse, String> {
    let source = PathBuf::from(&request.source);
    if !source.exists() {
        return Err(format!("Source not found: {}", request.source));
    }

    let id = make_id();
    let title = if request.title.trim().is_empty() {
        file_stem_or_name(&source)
    } else {
        request.title.trim().to_string()
    };

    let library_root = library_root()?;
    let item_dir = unique_path(&library_root.join(sanitize_name(&title)));
    fs::create_dir_all(&item_dir).map_err(|err| format!("Failed to create item folder: {err}"))?;

    let managed_path = copy_into_item_folder(&source, &item_dir)?;
    let response = ImportResponse {
        id,
        kind: request.kind,
        source: request.source,
        title,
        date: request.date,
        tags: request.tags,
        description: request.description,
        managed_path: managed_path.to_string_lossy().to_string(),
    };

    let metadata_path = item_dir.join("metadata.json");
    let metadata = serde_json::to_vec_pretty(&response)
        .map_err(|err| format!("Failed to serialize metadata: {err}"))?;
    fs::write(&metadata_path, metadata)
        .map_err(|err| format!("Failed to write metadata: {err}"))?;

    Ok(response)
}

#[tauri::command]
fn classify_import_path(path: String) -> Result<String, String> {
    let path = PathBuf::from(path);

    if !path.exists() {
        return Err("Dropped path does not exist.".to_string());
    }

    if path.is_dir() {
        return Ok("folder".to_string());
    }

    if is_archive_path(&path) {
        return Ok("archive".to_string());
    }

    Ok("file".to_string())
}

#[tauri::command]
fn list_items() -> Result<Vec<ImportResponse>, String> {
    let mut items = Vec::new();

    for root in library_roots_for_read()? {
        for entry in fs::read_dir(&root).map_err(|err| format!("Failed to read library: {err}"))? {
            let entry = entry.map_err(|err| format!("Failed to read library entry: {err}"))?;
            let metadata_path = entry.path().join("metadata.json");
            if !metadata_path.is_file() {
                continue;
            }

            let metadata = fs::read(&metadata_path)
                .map_err(|err| format!("Failed to read metadata: {err}"))?;
            let item = serde_json::from_slice::<ImportResponse>(&metadata)
                .map_err(|err| format!("Failed to parse metadata: {err}"))?;
            items.push(item);
        }
    }

    items.sort_by(|left, right| right.id.cmp(&left.id));
    Ok(items)
}

#[tauri::command]
fn export_library(destination: String) -> Result<LibraryTransferResponse, String> {
    let destination = ensure_zip_extension(PathBuf::from(destination));
    let library_root = library_root()?;
    let file = File::create(&destination)
        .map_err(|err| format!("Failed to create export archive: {err}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut item_count = 0;

    for entry in
        fs::read_dir(&library_root).map_err(|err| format!("Failed to read library: {err}"))?
    {
        let entry = entry.map_err(|err| format!("Failed to read library entry: {err}"))?;
        let path = entry.path();

        if path.is_dir() && path.join("metadata.json").is_file() {
            item_count += 1;
        }

        add_path_to_zip(&mut zip, &library_root, &path, options)?;
    }

    zip.finish()
        .map_err(|err| format!("Failed to finish export archive: {err}"))?;

    Ok(LibraryTransferResponse {
        path: destination.to_string_lossy().to_string(),
        item_count,
    })
}

#[tauri::command]
fn import_library_archive(source: String) -> Result<LibraryTransferResponse, String> {
    let source = PathBuf::from(source);

    if !source.is_file() {
        return Err("Import archive not found.".to_string());
    }

    let library_root = library_root()?;
    let file =
        File::open(&source).map_err(|err| format!("Failed to open import archive: {err}"))?;
    let mut archive =
        ZipArchive::new(file).map_err(|err| format!("Failed to read import archive: {err}"))?;
    let mut top_level_map: HashMap<String, PathBuf> = HashMap::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| format!("Failed to read archive entry: {err}"))?;
        let Some(enclosed_name) = entry.enclosed_name().map(PathBuf::from) else {
            continue;
        };

        if enclosed_name.as_os_str().is_empty() {
            continue;
        }

        let destination =
            mapped_import_destination(&library_root, &mut top_level_map, &enclosed_name)?;

        if entry.is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|err| format!("Failed to create imported folder: {err}"))?;
            continue;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create imported folder: {err}"))?;
        }

        let mut output = File::create(&destination)
            .map_err(|err| format!("Failed to write imported file: {err}"))?;
        io::copy(&mut entry, &mut output)
            .map_err(|err| format!("Failed to copy imported file: {err}"))?;
    }

    let mut item_count = 0;
    for item_dir in top_level_map.into_values() {
        if repair_imported_metadata(&item_dir)? {
            item_count += 1;
        }
    }

    Ok(LibraryTransferResponse {
        path: source.to_string_lossy().to_string(),
        item_count,
    })
}

#[tauri::command]
fn reveal_item(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    let target = if path.is_dir() {
        path
    } else {
        path.parent()
            .ok_or_else(|| "Could not resolve containing folder".to_string())?
            .to_path_buf()
    };

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(&target);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(&target);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(&target);
        command
    };

    command
        .spawn()
        .map_err(|err| format!("Failed to open folder: {err}"))?;
    Ok(())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let allowed = url == "https://github.com/noirlang"
        || url == "https://github.com/noirlang/"
        || url == "https://github.com/noirlang/forgetag"
        || url == "https://github.com/noirlang/forgetag/"
        || url.starts_with("https://github.com/noirlang/forgetag/releases");

    if !allowed {
        return Err("External URL is not allowed.".to_string());
    }

    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(&url);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(&url);
        command
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(&url);
        command
    };

    command
        .spawn()
        .map_err(|err| format!("Failed to open URL: {err}"))?;
    Ok(())
}

#[tauri::command]
fn update_target() -> UpdateTarget {
    let os = std::env::consts::OS.to_string();
    let arch = normalized_arch().to_string();
    let package_kind = match os.as_str() {
        "linux" => detect_linux_package_kind().to_string(),
        "macos" => "dmg".to_string(),
        "windows" => "msi".to_string(),
        _ => "unknown".to_string(),
    };
    let extension = match package_kind.as_str() {
        "appimage" => "AppImage",
        "deb" => "deb",
        "rpm" => "rpm",
        "dmg" => "dmg",
        "msi" => "msi",
        _ => "",
    }
    .to_string();

    UpdateTarget {
        os,
        arch,
        package_kind,
        extension,
    }
}

fn library_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "Home directory not found".to_string())?;
    let root = PathBuf::from(home).join("forgetag-library");
    fs::create_dir_all(&root).map_err(|err| format!("Failed to create library folder: {err}"))?;
    Ok(root)
}

fn library_roots_for_read() -> Result<Vec<PathBuf>, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| "Home directory not found".to_string())?;
    let home = PathBuf::from(home);
    let primary = library_root()?;
    let legacy = home.join(["Forge", "Tag Library"].concat());

    if legacy.exists() && legacy != primary {
        Ok(vec![primary, legacy])
    } else {
        Ok(vec![primary])
    }
}

fn add_path_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    root: &Path,
    path: &Path,
    options: SimpleFileOptions,
) -> Result<(), String> {
    let name = zip_entry_name(root, path)?;

    if path.is_dir() {
        if !name.is_empty() {
            zip.add_directory(format!("{name}/"), options)
                .map_err(|err| format!("Failed to add folder to export archive: {err}"))?;
        }

        for entry in fs::read_dir(path).map_err(|err| format!("Failed to read folder: {err}"))? {
            let entry = entry.map_err(|err| format!("Failed to read folder entry: {err}"))?;
            add_path_to_zip(zip, root, &entry.path(), options)?;
        }

        return Ok(());
    }

    zip.start_file(name, options)
        .map_err(|err| format!("Failed to add file to export archive: {err}"))?;
    let mut input = File::open(path).map_err(|err| format!("Failed to read file: {err}"))?;
    io::copy(&mut input, zip).map_err(|err| format!("Failed to write export archive: {err}"))?;
    Ok(())
}

fn zip_entry_name(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|err| format!("Failed to resolve export path: {err}"))?;
    let mut parts = Vec::new();

    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            _ => return Err("Export path contains an unsupported component.".to_string()),
        }
    }

    Ok(parts.join("/"))
}

fn mapped_import_destination(
    library_root: &Path,
    top_level_map: &mut HashMap<String, PathBuf>,
    relative: &Path,
) -> Result<PathBuf, String> {
    let mut components = relative.components();
    let Some(first) = components.next() else {
        return Err("Archive entry is empty.".to_string());
    };

    let Component::Normal(first_name) = first else {
        return Err("Archive entry contains an unsupported path.".to_string());
    };

    let first_key = first_name.to_string_lossy().to_string();
    let top_dir = top_level_map
        .entry(first_key.clone())
        .or_insert_with(|| unique_path(&library_root.join(sanitize_name(&first_key))));
    let mut destination = top_dir.clone();

    for component in components {
        match component {
            Component::Normal(part) => destination.push(part),
            _ => return Err("Archive entry contains an unsupported path.".to_string()),
        }
    }

    Ok(destination)
}

fn repair_imported_metadata(item_dir: &Path) -> Result<bool, String> {
    let metadata_path = item_dir.join("metadata.json");
    if !metadata_path.is_file() {
        return Ok(false);
    }

    let metadata = fs::read(&metadata_path)
        .map_err(|err| format!("Failed to read imported metadata: {err}"))?;
    let mut item = serde_json::from_slice::<ImportResponse>(&metadata)
        .map_err(|err| format!("Failed to parse imported metadata: {err}"))?;

    item.id = make_id();
    item.managed_path = first_managed_child(item_dir)?.to_string_lossy().to_string();

    let metadata = serde_json::to_vec_pretty(&item)
        .map_err(|err| format!("Failed to serialize imported metadata: {err}"))?;
    fs::write(&metadata_path, metadata)
        .map_err(|err| format!("Failed to write imported metadata: {err}"))?;
    Ok(true)
}

fn first_managed_child(item_dir: &Path) -> Result<PathBuf, String> {
    for entry in
        fs::read_dir(item_dir).map_err(|err| format!("Failed to read imported item: {err}"))?
    {
        let entry = entry.map_err(|err| format!("Failed to read imported item entry: {err}"))?;
        if entry.file_name() == "metadata.json" {
            continue;
        }
        return Ok(entry.path());
    }

    Ok(item_dir.to_path_buf())
}

fn ensure_zip_extension(path: PathBuf) -> PathBuf {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return path;
    }

    let mut path = path;
    path.set_extension("zip");
    path
}

fn is_archive_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tgz"
            )
        })
        .unwrap_or(false)
}

fn copy_into_item_folder(source: &Path, item_dir: &Path) -> Result<PathBuf, String> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .map(sanitize_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "imported-item".to_string());
    let destination = unique_path(&item_dir.join(name));

    if source.is_dir() {
        copy_dir(source, &destination)?;
    } else {
        fs::copy(source, &destination).map_err(|err| format!("Failed to copy file: {err}"))?;
    }

    Ok(destination)
}

fn copy_dir(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|err| format!("Failed to create folder: {err}"))?;

    for entry in fs::read_dir(source).map_err(|err| format!("Failed to read folder: {err}"))? {
        let entry = entry.map_err(|err| format!("Failed to read folder entry: {err}"))?;
        let entry_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir(&entry_path, &destination_path)?;
        } else {
            fs::copy(&entry_path, &destination_path)
                .map_err(|err| format!("Failed to copy file: {err}"))?;
        }
    }

    Ok(())
}

fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("item");
    let extension = path.extension().and_then(|extension| extension.to_str());

    for index in 1.. {
        let file_name = match extension {
            Some(extension) => format!("{stem}-{index}.{extension}"),
            None => format!("{stem}-{index}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    path.to_path_buf()
}

fn sanitize_name(value: &str) -> String {
    let mut name = value
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            character if character.is_control() => '-',
            character => character,
        })
        .collect::<String>()
        .trim()
        .trim_matches('.')
        .to_string();

    if name.is_empty() {
        name = "item".to_string();
    }

    name
}

fn file_stem_or_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("item")
        .to_string()
}

fn make_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("item-{millis}")
}

fn normalized_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "arm" => "armv7",
        other => other,
    }
}

fn detect_linux_package_kind() -> &'static str {
    if std::env::var_os("APPIMAGE").is_some() {
        return "appimage";
    }

    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let normalized = os_release.to_lowercase();

    if normalized.contains("id=fedora")
        || normalized.contains("id=\"fedora\"")
        || normalized.contains("id_like=\"rhel")
        || normalized.contains("id_like=rhel")
        || normalized.contains("opensuse")
        || normalized.contains("suse")
    {
        return "rpm";
    }

    if normalized.contains("id=debian")
        || normalized.contains("id=\"debian\"")
        || normalized.contains("id=ubuntu")
        || normalized.contains("id=\"ubuntu\"")
        || normalized.contains("id_like=debian")
        || normalized.contains("id_like=\"debian")
    {
        return "deb";
    }

    "appimage"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) = tauri::image::Image::from_bytes(APP_ICON) {
                    let _ = window.set_icon(icon);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_name,
            classify_import_path,
            export_library,
            import_item,
            import_library_archive,
            list_items,
            reveal_item,
            open_external_url,
            update_target
        ])
        .run(tauri::generate_context!())
        .expect("failed to run forgetag");
}
