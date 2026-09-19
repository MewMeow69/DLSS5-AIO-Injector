use crate::paths;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub manual_roots: Vec<String>,
    #[serde(default)]
    pub github_token: String,
    #[serde(default = "yes")]
    pub check_updates_on_start: bool,
    #[serde(default = "yes")]
    pub keep_nr_disabled: bool,
    #[serde(default = "auto")]
    pub default_preset: String,
    #[serde(default = "no")]
    pub allow_beta: bool,
}

fn yes() -> bool {
    true
}
fn no() -> bool {
    false
}
fn auto() -> String {
    "auto".into()
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            manual_roots: Vec::new(),
            github_token: String::new(),
            check_updates_on_start: true,
            keep_nr_disabled: true,
            default_preset: "auto".into(),
            allow_beta: false,
        }
    }
}

pub fn load() -> AppSettings {
    std::fs::read_to_string(paths::settings_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(s: &AppSettings) -> Result<(), String> {
    let text = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(paths::settings_path(), text).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub app_dir: String,
    pub downloads_dir: String,
    pub cache_dir: String,
    pub staging_dir: String,
    pub settings_file: String,
    pub version: String,
}

pub fn paths_info() -> AppPaths {
    AppPaths {
        app_dir: paths::app_dir().to_string_lossy().to_string(),
        downloads_dir: paths::downloads_dir().to_string_lossy().to_string(),
        cache_dir: paths::cache_dir().to_string_lossy().to_string(),
        staging_dir: paths::staging_dir().to_string_lossy().to_string(),
        settings_file: paths::settings_path().to_string_lossy().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub downloads_bytes: u64,
    pub staging_bytes: u64,
    pub cache_bytes: u64,
    pub artifacts: usize,
    pub imported: usize,
    pub downloaded: usize,
}

fn dir_size(dir: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                stack.push(e.path());
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

pub fn cache_stats() -> CacheStats {
    let items = crate::artifacts::list();
    CacheStats {
        downloads_bytes: dir_size(&paths::downloads_dir()),
        staging_bytes: dir_size(&paths::staging_dir()),
        cache_bytes: dir_size(&paths::cache_dir()),
        artifacts: items.len(),
        imported: items.iter().filter(|a| a.origin == "import").count(),
        downloaded: items.iter().filter(|a| a.origin == "download").count(),
    }
}

/// Deletes extraction scratch folders left over from earlier installs (they are
/// re-created on demand); keeps the download cache intact.
pub fn tidy_staging(max_age_secs: u64) -> usize {
    let dir = paths::staging_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else { return 0 };
    let now = std::time::SystemTime::now();
    let mut removed = 0;
    for e in rd.flatten() {
        let path = e.path();
        if !path.is_dir() {
            continue;
        }
        let age = e
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| now.duration_since(t).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if age > max_age_secs && std::fs::remove_dir_all(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Deletes downloaded files and the artifacts that pointed at them. Imported
/// payload files stay registered.
pub fn clear_downloads() -> Result<(usize, u64), String> {
    let downloads = paths::downloads_dir();
    let freed = dir_size(&downloads);
    let removed = crate::artifacts::unregister_under(&downloads);
    let _ = std::fs::remove_dir_all(&downloads);
    let _ = std::fs::remove_dir_all(paths::staging_dir());
    Ok((removed, freed))
}

/// Removes everything the app stores (cache, downloads, registry, settings).
pub fn reset_app_data() -> Result<(), String> {
    let dir = paths::app_dir();
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_file(paths::settings_path());
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    pub current: String,
    pub latest: String,
    pub url: String,
    pub newer: bool,
    pub error: Option<String>,
}

pub fn check_app_update() -> AppUpdate {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let url = "https://api.github.com/repos/MewMeow69/DLSS5-AIO-Injector/releases?per_page=5";
    let Some(body) = crate::http::curl_get(url) else {
        return AppUpdate {
            current,
            latest: String::new(),
            url: "https://github.com/MewMeow69/DLSS5-AIO-Injector/releases".into(),
            newer: false,
            error: Some("network unavailable".into()),
        };
    };
    let Ok(list) = serde_json::from_str::<Vec<serde_json::Value>>(&body) else {
        return AppUpdate {
            current,
            latest: String::new(),
            url: "https://github.com/MewMeow69/DLSS5-AIO-Injector/releases".into(),
            newer: false,
            error: Some("unexpected GitHub response".into()),
        };
    };
    let mut best: Option<(String, String)> = None;
    for rel in list {
        if rel.get("draft").and_then(|d| d.as_bool()).unwrap_or(false) {
            continue;
        }
        let tag = rel.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
        let page = rel.get("html_url").and_then(|t| t.as_str()).unwrap_or("");
        if tag.is_empty() {
            continue;
        }
        let version = tag.trim_start_matches('v').replace("-alpha", "").replace("-beta", "");
        let newer = crate::version::extract(&version) > crate::version::extract(&current);
        if newer {
            if best.as_ref().map(|(v, _)| crate::version::extract(&version) > crate::version::extract(v)).unwrap_or(true) {
                best = Some((version, page.to_string()));
            }
        }
    }
    match best {
        Some((latest, page)) => AppUpdate {
            current,
            latest,
            url: page,
            newer: true,
            error: None,
        },
        None => AppUpdate {
            current,
            latest: String::new(),
            url: "https://github.com/MewMeow69/DLSS5-AIO-Injector/releases".into(),
            newer: false,
            error: None,
        },
    }
}
