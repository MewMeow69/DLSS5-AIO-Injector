use crate::http;
use crate::paths;
use crate::registry::ReleaseInfo;
use crate::version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub kind: String,
    pub version: String,
    pub variant: Option<String>,
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub origin: String,
    pub added: i64,
    #[serde(default)]
    pub mtime: i64,
    #[serde(default)]
    pub note: Option<String>,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn store_path() -> PathBuf {
    paths::app_dir().join("artifacts.json")
}

/// Name-based fork detection for OptiScaler builds, used when the release
/// cache has not seen the exact asset name (offline imports, renames).
pub fn fork_kind_for_name(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    if lower.contains("nightly") || lower.ends_with(".7z") || lower.starts_with("optiscaler_v") {
        return Some("optiscaler-nightly");
    }
    if lower.starts_with("optiscaler-nr-") {
        return Some("optiscaler-wilsjo2");
    }
    if lower.contains("optiscalermfg") || lower.contains("optiscaler-mfg") {
        return Some("optiscaler-mfg");
    }
    if lower.contains("-v0.1.") || lower.contains("auto-exposure")
    {
        return Some("optiscaler-janblade");
    }
    if lower.contains("menu-revamp")
        || lower.contains("tier-presets")
        || lower.contains("clamp-fix")
        || lower.contains("optimized-defaults")
        || lower.contains("enlarge-filter")
        || lower.contains("sgsr1")
        || lower.contains("pass-presets")
    {
        return Some("optiscaler-janblade");
    }
    if lower.contains("integrated") {
        return Some("optiscaler-wilsjo2");
    }
    None
}

pub fn fork_kind_for_file(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_string();
    if let Some(kind) = crate::registry::cached_asset_fork(&name) {
        return Some(kind);
    }
    fork_kind_for_name(&name).map(|k| k.to_string())
}

/// Older registrations kept every OptiScaler build in one pool; re-tag them per
/// fork so each fork lists its own versions. Cheap and idempotent.
fn migrate_forks(items: &mut [Artifact]) -> bool {
    let mut changed = false;
    // the data folder was renamed: repoint recorded paths that moved with it
    let legacy = format!("\\{}\\", crate::paths::LEGACY_FOLDER);
    let current = format!("\\{}\\", crate::paths::APP_FOLDER);
    for a in items.iter_mut() {
        if a.path.contains(&legacy) {
            let fixed = a.path.replace(&legacy, &current);
            if Path::new(&fixed).is_file() {
                a.path = fixed;
                changed = true;
            }
        }
    }
    for a in items.iter_mut() {
        if a.kind != "optiscaler" {
            continue;
        }
        if let Some(kind) = fork_kind_for_file(Path::new(&a.path)) {
            a.kind = kind;
            changed = true;
        }
    }
    changed
}

pub fn list() -> Vec<Artifact> {
    let mut items: Vec<Artifact> = std::fs::read_to_string(store_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    if migrate_forks(&mut items) {
        save(&items);
    }
    items
}

fn save(items: &[Artifact]) {
    if let Ok(t) = serde_json::to_string_pretty(items) {
        let _ = std::fs::write(store_path(), t);
    }
}

pub fn sha256_file(path: &Path) -> Option<String> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

pub fn register(kind: &str, version: &str, variant: Option<&str>, path: &Path, origin: &str) -> Option<Artifact> {
    register_full(kind, version, variant, path, origin, None)
}

pub fn register_full(
    kind: &str,
    version: &str,
    variant: Option<&str>,
    path: &Path,
    origin: &str,
    note: Option<&str>,
) -> Option<Artifact> {
    let sha = sha256_file(path)?;
    let size = std::fs::metadata(path).ok()?.len();
    let mtime = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let note = note.map(|n| n.to_string()).or_else(|| {
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .and_then(|n| extract_note(&n))
    });
    let file_key = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut items = list();
    // the same file must not sit in two pools after a fork re-tag
    items.retain(|a| {
        !(a.kind != kind
            && Path::new(&a.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase() == file_key)
                .unwrap_or(false)
            && Path::new(&a.path).to_string_lossy().eq_ignore_ascii_case(&path.to_string_lossy()))
    });
    if let Some(existing) = items.iter_mut().find(|a| {
        a.kind == kind
            && a.version == version
            && a.variant.as_deref() == variant
            && Path::new(&a.path)
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase() == file_key)
                .unwrap_or(false)
    }) {
        existing.path = path.to_string_lossy().to_string();
        existing.sha256 = sha;
        existing.size = size;
        existing.mtime = mtime;
        existing.note = note;
        existing.origin = origin.to_string();
        existing.added = now_secs();
        let out = existing.clone();
        save(&items);
        return Some(out);
    }
    let art = Artifact {
        kind: kind.into(),
        version: version.into(),
        variant: variant.map(|v| v.to_string()),
        path: path.to_string_lossy().to_string(),
        sha256: sha,
        size,
        origin: origin.into(),
        added: now_secs(),
        mtime,
        note,
    };
    items.push(art.clone());
    save(&items);
    Some(art)
}

/// Files arrive with their instructions in the name:
/// "renodx-dlss5.addon64 (Set HDR Transfer Strength to 0 if you see blobs).addon64"
pub fn extract_note(file_name: &str) -> Option<String> {
    let open = file_name.find('(')?;
    let close = file_name.rfind(')')?;
    if close <= open + 1 {
        return None;
    }
    let note = file_name[open + 1..close].trim().to_string();
    if note.is_empty() {
        None
    } else {
        Some(note)
    }
}

/// Best OptiScaler build across every fork pool: an explicit pin wins, else the
/// newest version anywhere. Keeps "Auto" working now that forks are separate.
pub fn best_optiscaler() -> Option<Artifact> {
    const KINDS: [&str; 4] = ["optiscaler-wilsjo2", "optiscaler-janblade", "optiscaler-nightly", "optiscaler"];
    let pins = preferred();
    for kind in KINDS {
        if let Some(path) = pins.get(kind) {
            if let Some(hit) = list().into_iter().find(|a| a.kind == kind && &a.path == path) {
                return Some(hit);
            }
        }
    }
    let mut winner: Option<Artifact> = None;
    for kind in KINDS {
        if let Some(a) = best(kind, None) {
            winner = Some(match winner {
                None => a,
                Some(b) => {
                    if version::extract(&a.version) > version::extract(&b.version) {
                        a
                    } else {
                        b
                    }
                }
            });
        }
    }
    winner
}

pub fn best(kind: &str, variant: Option<&str>) -> Option<Artifact> {
    let items = list();
    if let Some(path) = preferred().get(kind) {
        if let Some(hit) = items
            .iter()
            .find(|a| a.kind == kind && &a.path == path)
            .filter(|a| match variant {
                Some(v) => a.variant.as_deref() == Some(v),
                None => true,
            })
        {
            return Some(hit.clone());
        }
    }
    items
        .into_iter()
        .filter(|a| a.kind == kind)
        .filter(|a| match variant {
            Some(v) => a.variant.as_deref() == Some(v),
            None => true,
        })
        .max_by(|a, b| {
            version::extract(&a.version)
                .cmp(&version::extract(&b.version))
                .then(a.added.cmp(&b.added))
        })
}

fn preferred_path() -> PathBuf {
    paths::app_dir().join("preferred.json")
}

pub fn preferred() -> std::collections::BTreeMap<String, String> {
    std::fs::read_to_string(preferred_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Drops registrations whose files live under `dir` (used when the download
/// cache is cleared). Imported payload files are untouched.
pub fn unregister_under(dir: &Path) -> usize {
    let prefix = dir.to_string_lossy().to_lowercase();
    let mut items = list();
    let before = items.len();
    items.retain(|a| !a.path.to_lowercase().starts_with(&prefix));
    if items.len() != before {
        save(&items);
    }
    before - items.len()
}

pub fn set_preferred(kind: &str, path: Option<&str>) {    let mut map = preferred();
    match path {
        Some(p) => {
            map.insert(kind.to_string(), p.to_string());
        }
        None => {
            map.remove(kind);
        }
    }
    if let Ok(t) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(preferred_path(), t);
    }
}

const KNOWN_NR_NVIDIA: &str = "e16bcf15e16e13f527491cdf7845b2fe6521a738d8f7c9c721866a8496e1fc8e";
const KNOWN_NR_COMPAT: &str = "e67dee209320cdafe0e93e45675d7aa34323a53acc57a72b2e40a181581c989a";

fn kind_for_dll(name: &str, sha: &str, size: u64) -> Option<(&'static str, &'static str)> {
    let n = name.to_lowercase();
    if n == "nvngx_dlssnr.dll" {
        if sha.eq_ignore_ascii_case(KNOWN_NR_NVIDIA) || size > 150_000_000 {
            return Some(("runtime-nr-nvidia", "310.8.0"));
        }
        if sha.eq_ignore_ascii_case(KNOWN_NR_COMPAT) || size > 100_000_000 {
            return Some(("runtime-nr-compat", "310.8-SF-v2"));
        }
        return Some(("runtime-nr-nvidia", "unknown"));
    }
    if n == "nvngx_dlss.dll" {
        return Some(("runtime-dlss", "310.9.1"));
    }
    if n == "nvngx_dlssd.dll" {
        return Some(("runtime-dlssd", "310.8.0"));
    }
    if n == "nvngx_dlssg.dll" {
        return Some(("runtime-dlssg", "310.8.0"));
    }
    if n == "dlss-enabler-headless.dll" {
        return Some(("dlss-enabler", "headless"));
    }
    if n == "fakenvapi.dll" {
        return Some(("fakenvapi", "local"));
    }
    None
}

fn streamline_version(name: &str) -> String {
    let lower = name.to_lowercase();
    match lower.find("streamline") {
        Some(i) => version::find_in_text(&name[i..]).unwrap_or_else(|| "local".into()),
        None => version::find_in_text(name).unwrap_or_else(|| "local".into()),
    }
}

pub fn import_dir(root: &Path) -> Vec<Artifact> {
    let mut added = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0u32)];
    let mut visited = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        if visited > 20_000 {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut has_sm86_dll = false;
        let mut has_sm86_ini = false;
        for e in rd.flatten() {
            visited += 1;
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                let lname = name.to_lowercase();
                let skip = ["target", "node_modules", ".git", "dist", "artifacts", "win-unpacked"];
                if depth < 3 && !skip.contains(&lname.as_str()) {
                    stack.push((p, depth + 1));
                }
                continue;
            }
            let lower = name.to_lowercase();
            let reg = |kind: &str, ver: &str, variant: Option<&str>| -> Option<Artifact> {
                let a = register(kind, ver, variant, &p, "import");
                a
            };
            if lower.ends_with(".addon64") {
                if lower.starts_with("renodx-dlss5") {
                    let variant = if lower.contains("performance") {
                        "performance"
                    } else if lower.contains("speedlemur") {
                        "speedlemur"
                    } else {
                        "dlss5"
                    };
                    if let Some(a) = register_full("renodx", "local", Some(variant), &p, "import", None) {
                        added.push(a);
                    }
                } else if lower.starts_with("renodx-dlss") {
                    if let Some(a) = register_full("renodx", "local", Some("base"), &p, "import", None) {
                        added.push(a);
                    }
                }
            } else if lower.ends_with(".zip") {
                if lower.starts_with("optiscaler-nr-") || lower.starts_with("optiscaler-dlssnr-") || lower.starts_with("optiscaler-dlss5-integrated") {
                    let variant = if lower.contains("rtx40-mfg") {
                        "rtx40-mfg"
                    } else if lower.contains("integrated") && !lower.contains("dlssnr") {
                        "integrated"
                    } else {
                        "standard"
                    };
                    let ver = version::find_in_text(&name).unwrap_or_else(|| "local".into());
                    let kind = fork_kind_for_file(&p).unwrap_or_else(|| "optiscaler".to_string());
                    let a = reg(&kind, &ver, Some(variant));
                    if let Some(a) = a {
                        added.push(a);
                    }
                } else if lower.starts_with("dlss5-feeder") {
                    let ver = version::find_in_text(&name).unwrap_or_else(|| "local".into());
                    if let Some(a) = reg("feeder", &ver, None) {
                        added.push(a);
                    }
                } else if lower.starts_with("streamline") || lower.contains("streamline") && lower.contains("dlss") {
                    let ver = streamline_version(&name);
                    if let Some(a) = reg("streamline", &ver, None) {
                        added.push(a);
                    }
                } else if lower.starts_with("lumenitefx") {
                    if let Some(a) = reg("lumenite", "mainline", None) {
                        added.push(a);
                    }
                } else if lower.starts_with("streamline") {
                    if let Some(a) = reg("streamline", "310.9.1", None) {
                        added.push(a);
                    }
                }
            } else if lower.starts_with("reshade_setup_") && lower.ends_with(".exe") {
                let ver: String = name
                    .trim_start_matches("ReShade_Setup_")
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                let ver = ver.trim_end_matches('.').to_string();
                if let Some(a) = reg("reshade", if ver.is_empty() { "local" } else { &ver }, None) {
                    added.push(a);
                }
            } else if lower.starts_with("fakenvapi") {
                if let Some(a) = reg("fakenvapi", &version::find_in_text(&name).unwrap_or_else(|| "local".into()), None) {
                    added.push(a);
                }
            } else if lower.starts_with("dlss-enabler-setup") && lower.ends_with(".exe") {
                if let Some(a) = reg("dlss-enabler-installer", "installer", None) {
                    added.push(a);
                }
            } else if lower.starts_with("optipatcher") && lower.ends_with(".asi") {
                let ver = if lower.contains("v0.") {
                    version::extract(&name).nums.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".")
                } else {
                    "rolling".to_string()
                };
                if let Some(a) = reg("optipatcher", &ver, None) {
                    added.push(a);
                }
            } else if lower == "install-dlss5feeder.ps1" {
                if let Some(a) = reg("installer-feeder", "integrated", None) {
                    added.push(a);
                }
            } else if lower == "version.dll" {
                has_sm86_dll = true;
            } else if lower == "dlssg_sm86.ini" {
                has_sm86_ini = true;
            } else if lower.ends_with(".dll") {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                if kind_for_dll(&name, "", size).is_some() {
                    let sha = sha256_file(&p).unwrap_or_default();
                    if let Some((kind, ver)) = kind_for_dll(&name, &sha, size) {
                        if let Some(a) = reg(kind, ver, None) {
                            added.push(a);
                        }
                    }
                }
            }
        }
        if has_sm86_dll && has_sm86_ini {
            let probe = dir.join("version.dll");
            if let Some(a) = register("sm86", "0.3.4", None, &probe, "import") {
                added.push(a);
            }
        }
    }
    added
}

fn payload_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Ok(here) = std::env::current_dir() {
        bases.push(here);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            bases.push(dir.to_path_buf());
        }
    }
    for base in bases {
        let mut cursor = Some(base.as_path());
        for _ in 0..4 {
            let Some(up) = cursor else { break };
            if up.is_dir() {
                // ponytail: bare ancestors are scanned up to 4 levels so the layout on this machine
                // (dlss5\ and Downloads\) is picked up. Other layouts use the Import folder button.
                roots.push(up.to_path_buf());
                for extra in [
                    "Test",
                    "Test\\OptiScaler-DLSS5-Integrated",
                    "Test\\OptiScaler-DLSS5-Integrated\\localfiles",
                ] {
                    let p = up.join(extra);
                    if p.is_dir() {
                        roots.push(p);
                    }
                }
            }
            cursor = up.parent();
        }
    }
    roots.push(paths::downloads_dir());
    roots.push(paths::staging_dir());
    roots.sort();
    roots.dedup();
    roots.sort_by_key(|p| p.components().count());
    let mut filtered: Vec<PathBuf> = Vec::new();
    for r in roots {
        let depth = |base: &Path| r.components().count().saturating_sub(base.components().count());
        let covered = filtered.iter().any(|k| r.starts_with(k) && depth(k) <= 3);
        if !covered {
            filtered.push(r);
        }
    }
    filtered
}

fn looks_like_payload_file(name: &str) -> bool {
    let n = name.to_lowercase();
    n.ends_with(".addon64")
        || n == "dlss-enabler-headless.dll"
        || n.starts_with("optiscaler-")
        || n.starts_with("optiscaler_")
        || n.starts_with("dlss5-feeder")
        || n.starts_with("reshade_setup")
        || n.starts_with("optipatcher")
        || n.starts_with("lumenitefx")
        || n.contains("streamline")
        || n == "install-dlss5feeder.ps1"
        || n == "nvngx_dlssnr.dll"
        || n == "nvngx_dlss.dll"
        || n == "version.dll"
}

/// Once per process: the payload scan walks big folders, and nothing new can
/// appear mid-session that the user would not add through Import Folder.
pub fn auto_import_once() -> Vec<Artifact> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static CHECKED: AtomicBool = AtomicBool::new(false);
    if CHECKED.swap(true, Ordering::SeqCst) {
        return Vec::new();
    }
    auto_import()
}

pub fn auto_import() -> Vec<Artifact> {
    // Import when the payload holds a file the registry has never seen; skip
    // otherwise so startup stays cheap once everything is registered.
    let known = list();
    let has_new = payload_roots().iter().any(|root| {
        walk_dirs_shallow(root)
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .map(|n| looks_like_payload_file(&n.to_string_lossy()))
                    .unwrap_or(false)
            })
            .any(|p| {
                !known
                    .iter()
                    .any(|a| Path::new(&a.path).to_string_lossy().eq_ignore_ascii_case(&p.to_string_lossy()))
            })
    });
    if !has_new {
        return Vec::new();
    }
    let mut out = Vec::new();
    for r in payload_roots() {
        if r.is_dir() {
            out.extend(import_dir(&r));
        }
    }
    out
}

fn walk_dirs_shallow(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0u32)];
    let mut visited = 0usize;
    while let Some((dir, depth)) = stack.pop() {
        if visited > 8_000 {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            visited += 1;
            let p = e.path();
            if p.is_dir() {
                let lname = e.file_name().to_string_lossy().to_lowercase();
                let skip = ["target", "node_modules", ".git", "dist", "artifacts", "win-unpacked"];
                if depth < 3 && !skip.contains(&lname.as_str()) {
                    stack.push((p, depth + 1));
                }
            } else {
                out.push(p);
            }
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub kind: String,
    pub file: String,
    pub received: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
}

pub type Emit<'a> = &'a dyn Fn(DownloadProgress);

fn content_length(url: &str) -> u64 {
    let mut cmd = std::process::Command::new("curl.exe");
    cmd.args(["-sIL", "--max-time", "20", "-A", "DLSS5-AIO/0.1", url]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let Ok(out) = cmd.output() else { return 0 };
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    text.lines()
        .filter(|l| l.starts_with("content-length:"))
        .filter_map(|l| l.split(':').nth(1).and_then(|v| v.trim().parse::<u64>().ok()))
        .max()
        .unwrap_or(0)
}

fn download_with_progress(
    url: &str,
    dest: &Path,
    kind: &str,
    expected_total: u64,
    emit_progress: Emit,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let name = dest.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let total = if expected_total > 0 {
        expected_total
    } else {
        content_length(url)
    };
    let emitted = |received: u64, total: u64, done: bool, error: Option<String>| {
        emit_progress(DownloadProgress {
            kind: kind.into(),
            file: name.clone(),
            received,
            total,
            done,
            error,
        });
    };
    emitted(0, total, false, None);
    let mut child = std::process::Command::new("curl.exe")
        .args(["-s", "-L", "--fail", "--max-time", "1800", "-o"])
        .arg(dest)
        .arg(url)
        .creation_flags(0x0800_0000)
        .spawn()
        .map_err(|e| e.to_string())?;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                let got = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
                emitted(got, total, false, None);
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            Err(e) => {
                emitted(0, total, true, Some(e.to_string()));
                return Err(e.to_string());
            }
        }
    }
    let but_ok = dest.is_file() && std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0) > 0;
    if !but_ok {
        emitted(0, total, true, Some("download failed".into()));
        return Err(format!("download failed: {}", url));
    }
    let size = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    emitted(size, total.max(size), true, None);
    Ok(())
}

/// Downloads one release file into the cache (sha256-verified when the release
/// ships a sidecar) and returns its path. Registration is the caller's job so
/// the variant stays explicit — that used to produce duplicate entries.
pub fn fetch_release(rel: &ReleaseInfo, kind: &str, file_needle: &str, emit_progress: Emit) -> Result<PathBuf, String> {
    let file = crate::registry::release_file(rel, file_needle)
        .ok_or_else(|| format!("release {} has no asset matching '{}'", rel.version, file_needle))?;
    let dest = paths::downloads_dir().join(kind).join(&file.name);
    if !dest.is_file() || std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0) != file.size {
        download_with_progress(&file.url, &dest, kind, file.size, emit_progress)?;
    }
    if let Some(sha_file) = crate::registry::release_file(rel, &format!("{}.sha256", file.name)) {
        if let Some(body) = http::curl_get(&sha_file.url) {
            let want = body
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let got = sha256_file(&dest).unwrap_or_default();
            if !want.is_empty() && want != got {
                let _ = std::fs::remove_file(&dest);
                return Err(format!("sha256 mismatch for {}", file.name));
            }
        }
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_instruction_notes_and_versions() {
        assert_eq!(
            super::extract_note("renodx-dlss5.addon64 (Set HDR Transfer Strength to 0 if you see blobs).addon64").as_deref(),
            Some("Set HDR Transfer Strength to 0 if you see blobs")
        );
        assert_eq!(super::extract_note("plain-file.zip"), None);
        assert_eq!(super::streamline_version("DLSS310.8.0-Streamline2.13.zip"), "2.13");
        assert_eq!(super::streamline_version("streamline.zip"), "local");
    }

    #[test]
    fn detects_forks_by_file_name() {
        assert_eq!(
            super::fork_kind_for_name("OptiScaler-NR-v0.8.3.zip"),
            Some("optiscaler-wilsjo2")
        );
        assert_eq!(
            super::fork_kind_for_name("OptiScaler-DLSSNR-v0.1.8-menu-revamp-tier-presets.zip"),
            Some("optiscaler-janblade")
        );
        assert_eq!(
            super::fork_kind_for_name("OptiScaler-DLSS5-Integrated.zip"),
            Some("optiscaler-wilsjo2")
        );
        assert_eq!(
            super::fork_kind_for_name("OptiScaler_v10.0.0-pre1_20260919.7z"),
            Some("optiscaler-nightly")
        );
        assert_eq!(super::fork_kind_for_name("some-other-mod.zip"), None);
    }

    #[test]
    #[ignore]
    fn live_reimport_payload() {
        let added = super::auto_import();
        println!("imported {} entries", added.len());
        for a in super::list() {
            println!(
                "{:20} {:28} {:12} {:>8} MB {}",
                a.kind,
                a.version,
                a.variant.unwrap_or_else(|| "-".into()),
                a.size / 1_048_576,
                a.origin
            );
        }
    }
}