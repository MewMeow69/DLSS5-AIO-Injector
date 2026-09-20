use crate::model::{Detection, ModState};
use crate::pe;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const EXE_NOISE: &[&str] = &[
    "unins",
    "unitycrashhandler",
    "crashhandler",
    "crashreport",
    "launcher",
    "setup",
    "install",
    "redist",
    "vcredist",
    "dxsetup",
    "dxwebsetup",
    "dotnetfx",
    "ue4prereqsetup",
    "ueprereqsetup",
    "easyanticheat",
    "battleye",
    "be_service",
    "beservice",
    "steamerrorreporter",
    "steamlaunch",
    "activation",
    "touchup",
    "helper",
    "report",
    "cefsubprocess",
    "quicksfv",
];

fn is_noise_exe(name: &str) -> bool {
    let n = name.to_lowercase();
    EXE_NOISE.iter().any(|p| n.contains(p))
}

// ponytail: bounded walk, depth 3, file cap. Deep enough for UE/Unity layouts, cheap enough
// to run per game in parallel. Widen if a real game gets missed.
pub fn walk_files(root: &Path, max_depth: u32, cap: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0u32)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            if out.len() >= cap {
                return out;
            }
            let p = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if depth < max_depth {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name == "logs" || name == "screenshots" || name == "crashdumps" {
                        continue;
                    }
                    stack.push((p, depth + 1));
                }
            } else {
                out.push(p);
            }
        }
    }
    out
}

fn is_x64(path: &Path) -> bool {
    pe::pe_read(path).map(|p| p.is_x64()).unwrap_or(false)
}

/// Biggest 64-bit exe under one of the usual binary subfolders, shallow scan.
/// The generic walk caps at a few thousand files, which a big game never gets
/// past, so Bin64\x64 has to be looked up by name.
fn x64_subfolder_exe(dir: &Path) -> Option<(u64, PathBuf)> {
    let mut best: Option<(u64, PathBuf)> = None;
    for rel in ["Bin64", "x64", "Win64", "binaries\\win64", "bin\\x64"] {
        let start = dir.join(rel);
        if !start.is_dir() {
            continue;
        }
        let mut stack = vec![(start, 0u32)];
        while let Some((d, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if depth < 2 {
                        stack.push((p, depth + 1));
                    }
                    continue;
                }
                let n = e.file_name().to_string_lossy().to_string();
                if !n.to_lowercase().ends_with(".exe") || is_noise_exe(&n) || !is_x64(&p) {
                    continue;
                }
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
                    best = Some((size, p));
                }
            }
        }
    }
    best
}

/// A game exe sitting directly in this folder (not somewhere below it).
pub fn has_direct_exe(dir: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else { return false };
    rd.flatten().any(|e| {
        let n = e.file_name().to_string_lossy().to_string();
        e.path().is_file() && n.to_lowercase().ends_with(".exe") && !is_noise_exe(&n)
    })
}

pub fn find_main_exe(dir: &Path, hint: Option<&str>) -> Option<PathBuf> {
    if let Some(h) = hint {
        let p = dir.join(h);
        if p.is_file() {
            return Some(p);
        }
        let hp = PathBuf::from(h);
        if hp.is_file() {
            return Some(hp);
        }
    }

    let unity = dir.join("UnityPlayer.dll").is_file();
    let mut best: Option<(u64, PathBuf)> = None;

    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if !name.to_lowercase().ends_with(".exe") || is_noise_exe(&name) {
                continue;
            }
            let stem = name.trim_end_matches(".exe").trim_end_matches(".EXE");
            let data = dir.join(format!("{}_Data", stem));
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            let score = if unity && data.is_dir() {
                1_000_000_000
            } else {
                size
            };
            if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                best = Some((score, p));
            }
        }
    }
    let all = walk_files(dir, 4, 4000);
    let mut shipping: Vec<(u64, PathBuf)> = Vec::new();
    let mut others: Vec<(u64, PathBuf)> = Vec::new();
    for p in all {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !name.to_lowercase().ends_with(".exe") || is_noise_exe(&name) {
            continue;
        }
        let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        if name.to_lowercase().contains("-win64-shipping") || name.to_lowercase().contains("-wingdk-shipping") {
            shipping.push((size, p));
        } else {
            others.push((size, p));
        }
    }
    let ship_pick = shipping.into_iter().max_by_key(|(s, _)| *s);

    // A 32-bit launcher at the root (BeamNG.drive.exe) hides the real game in a
    // subfolder (Bin64\BeamNG.drive.x64.exe) and makes a 64-bit game look 32-bit.
    let root_is_x64 = best.as_ref().map(|(_, p)| is_x64(p)).unwrap_or(false);
    if !root_is_x64 {
        if let Some((_, p)) = x64_subfolder_exe(dir) {
            return Some(p);
        }
        let root_size = best.as_ref().map(|(s, _)| *s).unwrap_or(0);
        let tree_x64 = others
            .iter()
            .chain(ship_pick.iter())
            .filter(|(_, p)| is_x64(p))
            .max_by_key(|(s, _)| *s);
        if let Some((size, p)) = tree_x64 {
            if *size > root_size {
                return Some(p.clone());
            }
        }
    }

    if let Some((score, path)) = best {
        // Unity: the exe with a matching _Data folder is the game.
        if score >= 1_000_000_000 {
            return Some(path);
        }
        // A UE game ships tools at its root (crash handlers, redist helpers); the
        // nested *-Win64-Shipping.exe is the game, not the biggest root exe.
        let root_is_shipping = path
            .file_name()
            .map(|n| {
                let n = n.to_string_lossy().to_lowercase();
                n.contains("-win64-shipping") || n.contains("-wingdk-shipping")
            })
            .unwrap_or(false);
        if !root_is_shipping {
            if let Some((_, ship)) = ship_pick {
                return Some(ship);
            }
        }
        return Some(path);
    }

    ship_pick
        .map(|(_, p)| p)
        .or_else(|| others.into_iter().max_by_key(|(s, _)| *s).map(|(_, p)| p))
}

struct NameSignals {
    upscalers: Vec<String>,
    framegen: Vec<String>,
}

fn name_signals(names: &HashSet<String>) -> NameSignals {
    let mut up = Vec::new();
    let mut fg = Vec::new();
    let has = |n: &str| names.contains(n);
    let has_prefix = |prefix: &str| names.iter().any(|n| n.starts_with(prefix));

    if has("nvngx_dlss.dll") || has_prefix("nvngx_dlss_") {
        up.push("dlss".into());
    }
    if has("nvngx_dlssd.dll") {
        up.push("dlss-rr".into());
    }
    if has("libxess.dll") || has("libxess_dx11.dll") {
        up.push("xess".into());
    }
    if has("amd_fidelityfx_dx12.dll")
        || has("amd_fidelityfx_loader_dx12.dll")
        || has("amd_fidelityfx_upscaler_dx12.dll")
        || has("amd_fidelityfx_vk.dll")
        || has_prefix("ffx_fsr3upscaler")
        || has_prefix("ffx_fsr2")
    {
        up.push("fsr".into());
    }
    if has("nvngx_dlssg.dll") {
        fg.push("dlssg".into());
    }
    if has("amd_fidelityfx_framegeneration_dx12.dll") || has_prefix("dlssg_to_fsr3") {
        fg.push("fsrfg".into());
    }
    if has("libxess_fg.dll") {
        fg.push("xefg".into());
    }
    NameSignals { upscalers: up, framegen: fg }
}

pub fn detect(dir: &Path, exe_hint: Option<&str>) -> Detection {
    let mut d = Detection {
        exe_dir: Some(dir.to_string_lossy().to_string()),
        ..Default::default()
    };
    let Some(exe) = find_main_exe(dir, exe_hint) else {
        d.error = Some("no executable found".into());
        return d;
    };
    d.exe = Some(exe.to_string_lossy().to_string());

    let files = walk_files(dir, 3, 5000);
    d.scanned_files = files.len() as u32;
    let mut names: HashSet<String> = HashSet::new();
    for p in &files {
        if let Some(n) = p.file_name() {
            names.insert(n.to_string_lossy().to_lowercase());
        }
    }

    let sig = name_signals(&names);
    d.upscalers = sig.upscalers;
    d.framegen = sig.framegen;

    let mut api: Vec<String> = Vec::new();
    if let Some(p) = pe::pe_read(&exe) {
        d.arch = Some(if p.is_x64() { "x64".into() } else { "x86".into() });
        if p.has_import("d3d12.dll") {
            api.push("dx12".into());
        }
        if p.has_import("d3d11.dll") {
            api.push("dx11".into());
        }
        if p.has_import("dxgi.dll") && api.is_empty() {
            api.push("dx11".into());
        }
        if p.has_import("vulkan-1.dll") {
            api.push("vulkan".into());
        }
        if p.has_import("opengl32.dll") {
            api.push("opengl".into());
        }
        if p.has_import("d3d9.dll") {
            api.push("d3d9".into());
        }
    }

    let unity = dir.join("UnityPlayer.dll").is_file();
    if unity {
        d.engine = Some("Unity".into());
    } else {
        let ue = dir.join("Engine").is_dir()
            || names.iter().any(|n| n.contains("-win64-shipping.exe"))
            || names.iter().any(|n| n.ends_with(".pak"));
        if ue {
            d.engine = Some("Unreal".into());
        }
    }
    if api.is_empty() {
        for probe in ["UnityPlayer.dll", "GameAssembly.dll", "d3d11.dll", "d3d12.dll"] {
            let p = dir.join(probe);
            if !p.is_file() || std::fs::metadata(&p).map(|m| m.len() > 200_000_000).unwrap_or(true) {
                continue;
            }
            if let Some(pe) = pe::pe_read(&p) {
                if pe.has_import("d3d12.dll") {
                    api.push("dx12".into());
                }
                if pe.has_import("d3d11.dll") {
                    api.push("dx11".into());
                }
                if pe.has_import("vulkan-1.dll") {
                    api.push("vulkan".into());
                }
                if !api.is_empty() {
                    break;
                }
            }
        }
    }
    d.api = api;

    let tech_markers: &[(&str, &str, &str)] = &[
        ("nvngx_dlssnr", "nr", "upscaler"),
        ("nvngx_dlssg", "dlssg", "fg"),
        ("nvngx_dlssd", "dlss-rr", "upscaler"),
        ("nvngx_dlss", "dlss", "upscaler"),
        ("libxess_fg", "xefg", "fg"),
        ("libxess", "xess", "upscaler"),
        ("amd_fidelityfx_framegeneration", "fsrfg", "fg"),
        ("amd_fidelityfx", "fsr", "upscaler"),
        ("ffx_fsr3", "fsr", "upscaler"),
        ("dlssg_to_fsr3", "fsrfg", "fg"),
        ("sl.dlss_g", "streamline-fg", "fg"),
    ];
    let marker_names: Vec<&str> = tech_markers.iter().map(|m| m.0).collect();
    // Reading a game exe costs seconds on a slow drive. Only do it when the
    // shipped file names gave no answer (the usual case is covered by those).
    let mut exe_markers = Vec::new();
    if d.upscalers.is_empty() && d.framegen.is_empty() {
        exe_markers = pe::scan_file_markers(&exe, &marker_names, 16 * 1_048_576);
        if unity {
            let up = dir.join("UnityPlayer.dll");
            if up.is_file() {
                for m in pe::scan_file_markers(&up, &marker_names, 16 * 1_048_576) {
                    if !exe_markers.contains(&m) {
                        exe_markers.push(m);
                    }
                }
            }
        }
    }
    for m in tech_markers {
        if exe_markers.iter().any(|x| x == m.0) {
            match m.2 {
                "upscaler" => {
                    if !d.upscalers.iter().any(|u| u == m.1) {
                        d.upscalers.push(m.1.into())
                    }
                }
                "fg" => {
                    if !d.framegen.iter().any(|u| u == m.1) {
                        d.framegen.push(m.1.into())
                    }
                }
                _ => {}
            }
        }
    }

    let mut mods = ModState::default();
    // Every name a ReShade or OptiScaler install can take; anything a game loads as
    // an interposer has to be one of these (or an .asi loaded by ReShade).
    let proxies = [
        "dxgi.dll", "winmm.dll", "version.dll", "dbghelp.dll", "winhttp.dll", "wininet.dll",
        "d3d12.dll", "d3d11.dll", "d3d10.dll", "d3d9.dll", "d3d8.dll", "opengl32.dll",
        "ddraw.dll", "dinput8.dll", "xinput1_3.dll", "nvngx.dll",
    ];
    // Marker scans read up to 32 MB per file; interposer DLLs sit on slow game
    // drives, so scan the present ones in parallel.
    const MOD_MARKERS: &[&str] = &["OptiScaler", "ReShade", "dlssg_sm86", "DXVK", "dgVoodoo"];
    let present: Vec<&str> = proxies.iter().copied().filter(|p| dir.join(p).is_file()).collect();
    let scanned: Vec<(&str, Vec<String>)> = std::thread::scope(|s| {
        let mut handles: Vec<_> = present
            .iter()
            .map(|p| {
                let path = dir.join(p);
                s.spawn(move || (*p, pe::scan_file_markers(&path, MOD_MARKERS, 32 * 1_048_576)))
            })
            .collect();
        handles.drain(..).map(|h| h.join().unwrap_or(("", Vec::new()))).collect()
    });
    for (p, found) in scanned {
        // An OptiScaler binary mentions ReShade, dlssg_sm86, DXVK and dgVoodoo in its
        // strings (interop and detection); do not count those as separate installs.
        let is_opti = found.iter().any(|m| m == "OptiScaler");
        for marker in found {
            match marker.as_str() {
                "OptiScaler" if mods.optiscaler.is_none() => mods.optiscaler = Some(p.into()),
                "ReShade" if !is_opti && mods.reshade.is_none() => mods.reshade = Some(p.into()),
                "dlssg_sm86" if !is_opti => mods.dlssg_sm86 = true,
                "DXVK" if !is_opti => mods.dxvk = true,
                "dgVoodoo" if !is_opti => mods.dgvoodoo = true,
                _ => {}
            }
        }
    }
    mods.optiscaler_asi = dir.join("OptiScaler.asi").is_file() || dir.join("OptiScaler.dll").is_file();
    if names.contains("dlss5-feed.addon64") || names.contains("dlss5-feed.addon32") {
        mods.feeder = true;
    }
    // Only the root-level NGX wrapper conflicts with our runtimes; dlss-enabler-headless.dll
    // inside OptiScaler\ is a payload we install ourselves.
    mods.dlss_enabler = dir.join("nvngx.dll").is_file()
        || std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten().any(|e| {
                    let n = e.file_name().to_string_lossy().to_lowercase();
                    n.starts_with("dlss-enabler") && n.ends_with(".dll")
                })
            })
            .unwrap_or(false);
    mods.fakenvapi = names.contains("fakenvapi.dll");
    mods.dlssg_sm86 = mods.dlssg_sm86
        || names.contains("dlssg_sm86.ini")
        || names.contains("dlssg_sm86.dll")
        || dir.join("dlssg_sm86").is_dir();
    if names.contains("nvngx_dlssnr.dll") {
        mods.nr_runtime = true;
        let p = dir.join("nvngx_dlssnr.dll");
        if let Some(h) = std::fs::metadata(&p).ok().map(|m| m.len()) {
            mods.nr_runtime_kind = Some(if h > 200_000_000 {
                "nvidia-310.8 (165 MB+)".into()
            } else if h > 100_000_000 {
                "shortfuse-sf (cached size)".into()
            } else {
                format!("{} bytes", h)
            });
        }
    }
    mods.optipatcher = dir.join("OptiScaler").join("plugins").is_dir()
        && std::fs::read_dir(dir.join("OptiScaler").join("plugins"))
            .map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().to_lowercase().contains("optipatcher")))
            .unwrap_or(false);
    mods.streamline = names.contains("sl.interposer.dll") || dir.join("OptiScaler").join("streamline").is_dir();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with("_DLSS5AIO") || n.starts_with("_NeuroDeck") || n.starts_with("_dlss5_backup") {
                mods.backup_dir = Some(n);
                break;
            }
        }
    }
    d.mods = mods;

    if d.arch.as_deref() == Some("x86") {
        d.warnings.push("32-bit game: NR requires a 64-bit process".into());
    }
    let ac = ["EasyAntiCheat.exe", "EasyAntiCheat_EOS.exe", "BEService_x64.exe", "BEService.exe", "start_protected_game.exe"];
    if ac.iter().any(|f| dir.join(f).is_file()) {
        d.warnings.push("anti-cheat present: do not mod online games".into());
    }
    if d.upscalers.is_empty() && d.framegen.is_empty() {
        d.warnings.push("no upscaler detected: feeder path required".into());
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_game() -> Option<PathBuf> {
        let p = PathBuf::from(r"D:\Games\Easy Red 2");
        if p.join("Easy Red 2.exe").is_file() {
            Some(p)
        } else {
            None
        }
    }

    #[test]
    fn detects_foreign_mods_without_false_reshade() {
        let dir = std::env::temp_dir().join("neurodeck-detect-foreign");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Game.exe"), vec![0u8; 1024]).unwrap();
        // OptiScaler's own strings mention ReShade and dlssg_sm86 - must not count as those.
        std::fs::write(dir.join("winmm.dll"), b"MZ ... OptiScaler ... ReShade ... dlssg_sm86").unwrap();
        std::fs::write(dir.join("dxgi.dll"), b"MZ ... DXVK ... linker: DXVK").unwrap();
        std::fs::write(dir.join("OptiScaler.asi"), b"asi").unwrap();
        std::fs::write(dir.join("dlss-enabler.dll"), b"x").unwrap();
        std::fs::write(dir.join("fakenvapi.dll"), b"x").unwrap();
        std::fs::write(dir.join("nvngx_dlssnr.dll"), vec![7u8; 4096]).unwrap();

        let d = detect(&dir, None);
        assert_eq!(d.mods.optiscaler.as_deref(), Some("winmm.dll"));
        assert!(d.mods.reshade.is_none(), "OptiScaler must not be reported as ReShade");
        assert!(!d.mods.dlssg_sm86, "OptiScaler must not be reported as sm86");
        assert!(d.mods.dxvk);
        assert!(d.mods.optiscaler_asi);
        assert!(d.mods.dlss_enabler);
        assert!(d.mods.fakenvapi);
        assert!(d.mods.nr_runtime);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefers_the_real_64bit_exe_over_a_32bit_launcher() {
        let sys = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let x86 = Path::new(&sys).join("SysWOW64").join("notepad.exe");
        let x64 = Path::new(&sys).join("System32").join("notepad.exe");
        if !x86.is_file() || !x64.is_file() {
            return;
        }
        let dir = std::env::temp_dir().join("neurodeck-arch-pick");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Bin64")).unwrap();
        std::fs::copy(&x86, dir.join("Launcher.exe")).unwrap();
        let mut real = std::fs::read(&x64).unwrap();
        real.extend_from_slice(&vec![0u8; 2 * 1024 * 1024]);
        std::fs::write(dir.join("Bin64").join("Game.x64.exe"), real).unwrap();

        let pick = find_main_exe(&dir, None).expect("main exe");
        assert_eq!(pick.file_name().unwrap().to_string_lossy(), "Game.x64.exe");
        assert!(is_x64(&pick));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_unity_main_exe() {
        let Some(dir) = reference_game() else { return };
        let exe = find_main_exe(&dir, None).expect("main exe");
        assert_eq!(exe.file_name().unwrap().to_string_lossy(), "Easy Red 2.exe");
    }

    #[test]
    fn detects_reference_install() {
        let Some(dir) = reference_game() else { return };
        let d = detect(&dir, None);
        assert_eq!(d.engine.as_deref(), Some("Unity"));
        assert_eq!(d.arch.as_deref(), Some("x64"));
        assert!(d.mods.feeder, "feeder add-on detected");
        assert!(d.mods.nr_runtime, "nvngx_dlssnr.dll detected");
        assert!(d.mods.optiscaler.is_some(), "OptiScaler proxy detected");
    }
}


