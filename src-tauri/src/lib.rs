use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

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
    fs::write(&metadata_path, metadata).map_err(|err| format!("Failed to write metadata: {err}"))?;

    Ok(response)
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
            import_item,
            list_items,
            reveal_item,
            open_external_url,
            update_target
        ])
        .run(tauri::generate_context!())
        .expect("failed to run forgetag");
}
