use std::path::PathBuf;

pub const APP_FOLDER: &str = "DLSS5-AIO-Injector";
pub const LEGACY_FOLDER: &str = "NeuroDeck";

/// One-time move of the old data folder so caches and the artifact registry
/// survive the rename.
fn migrate(base: &std::path::Path, dir: &std::path::Path) {
    let legacy = base.join(LEGACY_FOLDER);
    if !dir.exists() && legacy.is_dir() {
        let _ = std::fs::rename(&legacy, dir);
    }
}

pub fn app_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join(APP_FOLDER);
    migrate(&base, &dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn downloads_dir() -> PathBuf {
    let dir = app_dir().join("downloads");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn cache_dir() -> PathBuf {
    let dir = app_dir().join("cache");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn staging_dir() -> PathBuf {
    let dir = app_dir().join("staging");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn settings_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join(APP_FOLDER);
    migrate(&base, &dir);
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}
