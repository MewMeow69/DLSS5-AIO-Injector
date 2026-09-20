use crate::model::{Detection, Game};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// Small scan cache: which games sit under a manual root (keyed by the root's
// mtime) and detection results (keyed by the game folder's mtime).
// ponytail: one JSON file, size is a few hundred KB at most, pruned by age.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RootEntry {
    mtime: u64,
    games: Vec<Game>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DetectEntry {
    mtime: u64,
    exe_mtime: u64,
    t: u64,
    d: Detection,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Store {
    version: u32,
    #[serde(default)]
    roots: BTreeMap<String, RootEntry>,
    #[serde(default)]
    detect: BTreeMap<String, DetectEntry>,
}

const VERSION: u32 = 2;
const MAX_DETECT: usize = 400;
const MAX_AGE_DAYS: u64 = 30;

fn path() -> PathBuf {
    crate::paths::cache_dir().join("scan-cache.json")
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn mtime(path: &std::path::Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
        .unwrap_or(0)
}

fn load() -> Store {
    let Ok(text) = std::fs::read_to_string(path()) else { return Store::default() };
    let mut s: Store = serde_json::from_str(&text).unwrap_or_default();
    if s.version != VERSION {
        return Store::default();
    }
    let cutoff = now().saturating_sub(MAX_AGE_DAYS * 86_400);
    s.detect.retain(|_, e| e.t >= cutoff);
    s
}

fn save(s: &Store) {
    let mut s = s.clone();
    s.version = VERSION;
    let cutoff = now().saturating_sub(MAX_AGE_DAYS * 86_400);
    s.detect.retain(|_, e| e.t >= cutoff);
    if s.detect.len() > MAX_DETECT {
        let mut ages: Vec<(u64, String)> = s.detect.iter().map(|(k, e)| (e.t, k.clone())).collect();
        ages.sort();
        for (_, k) in ages.into_iter().take(s.detect.len() - MAX_DETECT) {
            s.detect.remove(&k);
        }
    }
    let tmp = path().with_extension("tmp");
    if serde_json::to_string(&s).map(|t| std::fs::write(&tmp, t)).is_ok() {
        let _ = std::fs::rename(&tmp, path());
    }
}

pub fn root_games(root: &std::path::Path) -> Option<Vec<Game>> {
    let s = load();
    let key = root.to_string_lossy().to_lowercase();
    let e = s.roots.get(&key)?;
    if e.mtime != mtime(root) {
        return None;
    }
    Some(e.games.clone())
}

pub fn store_root_games(root: &std::path::Path, games: &[Game]) {
    let mut s = load();
    s.roots.insert(
        root.to_string_lossy().to_lowercase(),
        RootEntry { mtime: mtime(root), games: games.to_vec() },
    );
    s.roots.retain(|k, _| std::path::Path::new(k).is_dir());
    save(&s);
}

fn exe_mtime(d: &Detection) -> u64 {
    d.exe.as_deref().map(|e| mtime(std::path::Path::new(e))).unwrap_or(0)
}

pub fn detection(dir: &std::path::Path) -> Option<Detection> {
    let s = load();
    let e = s.detect.get(&dir.to_string_lossy().to_lowercase())?;
    if e.mtime != mtime(dir) {
        return None;
    }
    // A patched game replaces the exe without touching the folder mtime.
    if exe_mtime(&e.d) != e.exe_mtime {
        return None;
    }
    Some(e.d.clone())
}

pub fn store_detection(dir: &std::path::Path, d: &Detection) {
    let mut s = load();
    s.detect.insert(
        dir.to_string_lossy().to_lowercase(),
        DetectEntry { mtime: mtime(dir), exe_mtime: exe_mtime(d), t: now(), d: d.clone() },
    );
    save(&s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_roundtrip_and_miss_on_unknown_root() {
        let dir = std::env::temp_dir().join("neurodeck-cache-test");
        let other = std::env::temp_dir().join("neurodeck-cache-test-other");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&other);
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(path());

        assert!(detection(&dir).is_none(), "empty cache");
        let mut d = Detection::default();
        d.engine = Some("Unreal".into());
        store_detection(&dir, &d);
        assert_eq!(detection(&dir).and_then(|d| d.engine).as_deref(), Some("Unreal"));

        let games = vec![Game {
            id: "manual:x".into(),
            store: "manual".into(),
            name: "X".into(),
            install_dir: dir.to_string_lossy().to_string(),
            exe: None,
            app_id: None,
            size_bytes: None,
            library_online: true,
        }];
        store_root_games(&dir, &games);
        assert_eq!(root_games(&dir).map(|g| g.len()), Some(1));
        assert!(root_games(&other).is_none(), "unknown root is a miss");

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(path());
    }
}
