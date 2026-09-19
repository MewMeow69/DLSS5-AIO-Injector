use crate::model::Game;
use crate::vdf;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

fn reg_str(root: RegKey, path: &str, value: &str) -> Option<String> {
    root.open_subkey(path).ok()?.get_value::<String, _>(value).ok()
}

fn steam_paths() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Some(p) = reg_str(hkcu, r"Software\Valve\Steam", "SteamPath") {
        out.push(PathBuf::from(p.replace('/', "\\")));
    }
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Some(p) = reg_str(hklm, r"SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath") {
        out.push(PathBuf::from(p));
    }
    out.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
    out.retain(|p| p.join("steamapps").is_dir());
    out.sort();
    out.dedup();
    out
}

fn steam_libraries(steam: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam.to_path_buf()];
    let vdf_path = steam.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = std::fs::read_to_string(&vdf_path) {
        let tree = vdf::parse(&text);
        let root = tree.get("libraryfolders").unwrap_or(&tree);
        if let Some(obj) = root.obj() {
            for (_k, v) in obj {
                match v {
                    vdf::Vdf::Str(s) => libs.push(PathBuf::from(s.replace("\\\\", "\\"))),
                    vdf::Vdf::Obj(_) => {
                        if let Some(p) = v.get("path").and_then(|p| p.str()) {
                            libs.push(PathBuf::from(p.replace("\\\\", "\\")));
                        }
                    }
                }
            }
        }
    }
    libs.sort();
    libs.dedup();
    libs
}

fn steam_games() -> Vec<Game> {
    let mut games = Vec::new();
    let mut seen_appids = HashSet::new();
    for steam in steam_paths() {
        for lib in steam_libraries(&steam) {
            let sa = lib.join("steamapps");
            let Ok(rd) = std::fs::read_dir(&sa) else { continue };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
                let tree = vdf::parse(&text);
                let Some(state) = tree.get("AppState") else { continue };
                let appid = state.get("appid").and_then(|v| v.str()).unwrap_or("").to_string();
                let gname = state.get("name").and_then(|v| v.str()).unwrap_or("").to_string();
                let installdir = state.get("installdir").and_then(|v| v.str()).unwrap_or("").to_string();
                let size = state
                    .get("SizeOnDisk")
                    .and_then(|v| v.str())
                    .and_then(|s| s.parse::<u64>().ok());
                if appid.is_empty() || installdir.is_empty() || !seen_appids.insert(appid.clone()) {
                    continue;
                }
                let lname = gname.to_lowercase();
                if lname.contains("steamworks common")
                    || lname == "steamvr"
                    || lname.contains("steam linux runtime")
                    || lname.contains("steam client")
                {
                    continue;
                }
                let dir = sa.join("common").join(&installdir);
                games.push(Game {
                    id: format!("steam:{}", appid),
                    store: "steam".into(),
                    name: if gname.is_empty() { installdir.clone() } else { gname },
                    install_dir: dir.to_string_lossy().to_string(),
                    exe: None,
                    app_id: Some(appid),
                    size_bytes: size,
                    library_online: dir.is_dir(),
                });
            }
        }
    }
    games
}

fn epic_games() -> Vec<Game> {
    let manifests = PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
    let Ok(rd) = std::fs::read_dir(&manifests) else {
        return Vec::new();
    };
    let mut games = Vec::new();
    for e in rd.flatten() {
        if e.path().extension().map(|x| x != "item").unwrap_or(true) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let Ok(j) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let name = j.get("DisplayName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let loc = j.get("InstallLocation").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let launch = j.get("LaunchExecutable").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let app_name = j.get("AppName").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let is_app = j.get("bIsApplication").and_then(|v| v.as_bool()).unwrap_or(true);
        if name.is_empty() || loc.is_empty() || !is_app || launch.is_empty() {
            continue;
        }
        if launch.to_lowercase().contains("unrealengine") || launch.to_lowercase().contains("launcher") {
            continue;
        }
        let dir = PathBuf::from(&loc);
        games.push(Game {
            id: format!("epic:{}", if app_name.is_empty() { name.clone() } else { app_name.clone() }),
            store: "epic".into(),
            name,
            install_dir: dir.to_string_lossy().to_string(),
            exe: Some(dir.join(launch.replace('/', "\\")).to_string_lossy().to_string()),
            app_id: Some(app_name),
            size_bytes: None,
            library_online: dir.is_dir(),
        });
    }
    games
}

fn gog_games() -> Vec<Game> {
    let mut games = Vec::new();
    let roots = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\GOG.com\Games"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\GOG.com\Games"),
        (HKEY_CURRENT_USER, r"SOFTWARE\GOG.com\Games"),
    ];
    let mut seen = HashSet::new();
    for (hive, path) in roots {
        let root = RegKey::predef(hive);
        let Ok(key) = root.open_subkey(path) else { continue };
        for sub in key.enum_keys().flatten() {
            let Ok(k) = key.open_subkey(&sub) else { continue };
            let name = k.get_value::<String, _>("gameName").unwrap_or_default();
            let dir = k.get_value::<String, _>("path").unwrap_or_default();
            if name.is_empty() || dir.is_empty() || !seen.insert(dir.to_lowercase()) {
                continue;
            }
            let exe = k.get_value::<String, _>("exe").unwrap_or_default();
            let dirp = PathBuf::from(&dir);
            games.push(Game {
                id: format!("gog:{}", sub),
                store: "gog".into(),
                name,
                install_dir: dir.clone(),
                exe: if exe.is_empty() { None } else { Some(dirp.join(exe).to_string_lossy().to_string()) },
                app_id: Some(sub),
                size_bytes: None,
                library_online: dirp.is_dir(),
            });
        }
    }
    games
}

fn manual_games() -> Vec<Game> {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return Vec::new();
    };
    let settings = PathBuf::from(appdata).join(crate::paths::APP_FOLDER).join("settings.json");
    let Ok(text) = std::fs::read_to_string(settings) else {
        return Vec::new();
    };
    let Ok(j) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let mut games = Vec::new();
    if let Some(arr) = j.get("manualRoots").and_then(|v| v.as_array()) {
        for v in arr {
            let Some(p) = v.as_str() else { continue };
            let dir = PathBuf::from(p);
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.to_string());
            games.push(Game {
                id: format!("manual:{}", name.to_lowercase().replace(' ', "_")),
                store: "manual".into(),
                name,
                install_dir: p.to_string(),
                exe: None,
                app_id: None,
                size_bytes: None,
                library_online: dir.is_dir(),
            });
        }
    }
    games
}

pub fn scan_all() -> Vec<Game> {
    let mut games = Vec::new();
    games.extend(steam_games());
    games.extend(epic_games());
    games.extend(gog_games());
    games.extend(manual_games());
    let mut seen = HashSet::new();
    games.retain(|g| seen.insert(g.install_dir.to_lowercase()));
    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_smoke_prints_libraries() {
        let games = scan_all();
        for g in &games {
            println!("{:7} | {:38} | online={} | {}", g.store, g.name, g.library_online, g.install_dir);
        }
        println!("total: {}", games.len());
    }
}
