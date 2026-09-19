mod artifacts;
mod detect;
mod download;
mod gpu;
mod ini;
mod http;
mod install;
mod library;
mod model;
mod paths;
mod pe;
mod registry;
mod settings;
mod vdf;
mod version;

use artifacts::Artifact;
use install::{ComponentRecord, GameState, InstallOptions, InstallPlan, InstallReport};
use model::{Detection, Game, GpuInfo};
use registry::Component;
use std::path::{Path, PathBuf};
use tauri::Manager;

/// Every command that touches the disk or the network runs off the UI thread:
/// sync commands execute on the main thread and freeze the window.
async fn blocking<T, F>(job: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_library() -> Vec<Game> {
    blocking(|| {
        let games = library::scan_all();
        eprintln!("[dlss5-aio] scan_library -> {} games", games.len());
        games
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn detect_game(install_dir: String, exe_hint: Option<String>) -> Detection {
    blocking(move || {
        let dir = Path::new(&install_dir);
        if !dir.is_dir() {
            return Detection {
                exe_dir: Some(install_dir.clone()),
                error: Some("game folder offline or missing".into()),
                ..Default::default()
            };
        }
        detect::detect(dir, exe_hint.as_deref())
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn system_info() -> GpuInfo {
    blocking(gpu::gpu_info).await.unwrap_or_default()
}

#[tauri::command]
async fn steam_search_appid(term: String) -> Option<String> {
    blocking(move || {
        let url = format!(
            "https://store.steampowered.com/api/storesearch/?term={}&cc=us&l=en",
            http::url_encode(&term)
        );
        let body = http::curl_get(&url)?;
        let j: serde_json::Value = serde_json::from_str(&body).ok()?;
        let id = j.get("items")?.as_array()?.first()?.get("id")?.as_i64()?;
        Some(id.to_string())
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
async fn add_manual_root(path: String) -> Vec<Game> {
    blocking(move || {
        let file = paths::settings_path();
        let mut j: serde_json::Value = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let roots = j
            .as_object_mut()
            .map(|o| o.entry("manualRoots").or_insert_with(|| serde_json::json!([])));
        if let Some(arr) = roots.and_then(|v| v.as_array_mut()) {
            if !arr.iter().any(|v| v.as_str().map(|s| s.eq_ignore_ascii_case(&path)).unwrap_or(false)) {
                arr.push(serde_json::Value::String(path));
            }
        }
        if let Ok(text) = serde_json::to_string_pretty(&j) {
            let _ = std::fs::write(&file, text);
        }
        library::scan_all()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn list_artifacts() -> Vec<Artifact> {
    blocking(|| {
        artifacts::auto_import_once();
        artifacts::list()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn set_preferred_artifact(kind: String, path: Option<String>) -> Vec<Artifact> {
    blocking(move || {
        artifacts::set_preferred(&kind, path.as_deref());
        artifacts::list()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn preferred_artifacts() -> std::collections::BTreeMap<String, String> {
    blocking(artifacts::preferred)
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn import_artifacts(folder: Option<String>) -> Vec<Artifact> {
    blocking(move || match folder {
        Some(f) => artifacts::import_dir(Path::new(&f)),
        None => artifacts::auto_import(),
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
async fn list_components(refresh: bool, allow_beta: bool) -> Vec<Component> {
    blocking(move || registry::list(refresh, allow_beta))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn components_checked_at() -> i64 {
    blocking(registry::last_check).await.unwrap_or(0)
}

#[tauri::command]
async fn download_component(
    app: tauri::AppHandle,
    id: String,
    variant: Option<String>,
    channel: Option<String>,
) -> Result<Artifact, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Emitter;
        let emitter = app.clone();
        let cb = |p: artifacts::DownloadProgress| {
            let _ = emitter.emit("download:progress", p);
        };
        let channel = channel.unwrap_or_else(|| "stable".into());
        download::component(&id, variant.as_deref(), &channel, &cb)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn plan_install(install_dir: String, exe_hint: Option<String>, options: InstallOptions) -> InstallPlan {
    blocking(move || {
        let dir = Path::new(&install_dir);
        let detection = if dir.is_dir() {
            detect::detect(dir, exe_hint.as_deref())
        } else {
            Detection::default()
        };
        install::plan(dir, &detection, &options)
    })
    .await
    .unwrap_or_else(|e| InstallPlan {
        exe_dir: None,
        exe: None,
        proxy: String::new(),
        steps: Vec::new(),
        warnings: vec![e],
    })
}

#[tauri::command]
async fn install_game(
    app: tauri::AppHandle,
    install_dir: String,
    exe_hint: Option<String>,
    options: InstallOptions,
) -> Result<InstallReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Emitter;
        let dir = PathBuf::from(&install_dir);
        if !dir.is_dir() {
            return Err("game folder not found".to_string());
        }
        let detection = detect::detect(&dir, exe_hint.as_deref());
        let emitter = app.clone();
        let cb = move |stage: &str, title: &str, detail: &str, done: bool, error: Option<String>| {
            let _ = emitter.emit(
                "install:progress",
                serde_json::json!({ "stage": stage, "title": title, "detail": detail, "done": done, "error": error }),
            );
        };
        install::run(&cb, &detection, &options)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn game_state(install_dir: String) -> GameState {
    blocking(move || install::state(Path::new(&install_dir)))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn read_game_settings(install_dir: String) -> settings::GameSettings {
    blocking(move || settings::read(Path::new(&install_dir), None))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn write_game_settings(install_dir: String, settings: settings::GameSettings) -> Result<Vec<String>, String> {
    blocking(move || {
        let dir = Path::new(&install_dir);
        if !dir.is_dir() {
            return Err("game folder not found".into());
        }
        settings::write(dir, &settings)
    })
    .await
    .unwrap_or_else(|e| Err(e))
}

#[tauri::command]
async fn rollback_game(install_dir: String) -> Result<String, String> {
    blocking(move || install::rollback(Path::new(&install_dir)))
        .await
        .unwrap_or_else(|e| Err(e))
}

#[tauri::command]
async fn uninstall_game(install_dir: String) -> Result<String, String> {
    blocking(move || install::uninstall(Path::new(&install_dir)))
        .await
        .unwrap_or_else(|e| Err(e))
}

#[tauri::command]
async fn run_installer_file(path: String) -> Result<(), String> {
    blocking(move || {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&path)
            .creation_flags(0)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
    .unwrap_or_else(|e| Err(e))
}

#[tauri::command]
async fn installed_games() -> Vec<(String, Vec<ComponentRecord>)> {
    blocking(|| {
        library::scan_all()
            .into_iter()
            .filter_map(|g| {
                let st = install::state(Path::new(&g.install_dir));
                if st.managed {
                    Some((g.install_dir, st.components))
                } else {
                    None
                }
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_zoom(1.12);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_library,
            detect_game,
            system_info,
            steam_search_appid,
            add_manual_root,
            list_artifacts,
            set_preferred_artifact,
            preferred_artifacts,
            import_artifacts,
            list_components,
            components_checked_at,
            download_component,
            plan_install,
            install_game,
            game_state,
            read_game_settings,
            write_game_settings,
            rollback_game,
            uninstall_game,
            run_installer_file,
            installed_games
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
