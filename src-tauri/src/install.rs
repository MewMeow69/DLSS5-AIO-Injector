use crate::artifacts::{self, Artifact};
use crate::ini::set as set_ini;
use crate::model::Detection;
use crate::paths;
use crate::pe;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstallOptions {
    pub optiscaler_path: Option<String>,
    pub install_feeder: bool,
    pub install_sm86: bool,
    pub optipatcher: bool,
    pub streamline: bool,
    pub preset: String,
    pub nr_enabled: bool,
    pub rtx40_mfg: bool,
    pub force: bool,
    #[serde(default)]
    pub nr_provider: String,
    #[serde(default)]
    pub fg_input: String,
    #[serde(default)]
    pub fg_output: String,
    #[serde(default)]
    pub fg_replacement: String,
    #[serde(default)]
    pub xess_libs: bool,
    #[serde(default)]
    pub dlss_enabler: bool,
    #[serde(default)]
    pub runtime_dlssd: bool,
    #[serde(default)]
    pub hudfix: bool,
}

impl InstallOptions {
    pub fn provider(&self) -> &str {
        if self.nr_provider.is_empty() {
            "optiscaler"
        } else {
            &self.nr_provider
        }
    }

    pub fn uses_optiscaler(&self) -> bool {
        self.provider() == "optiscaler"
    }
}

pub fn renodx_artifact_for(provider: &str) -> Option<Artifact> {
    let variant = match provider {
        "renodx-base" => "base",
        "renodx-speedlemur" => "speedlemur",
        "renodx-performance" => "performance",
        _ => "dlss5",
    };
    artifacts::best("renodx", Some(variant))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    pub exe_dir: Option<String>,
    pub exe: Option<String>,
    pub proxy: String,
    pub steps: Vec<PlanStep>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentRecord {
    pub kind: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Journal {
    ts: String,
    added: Vec<String>,
    replaced: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    installed_at: String,
    components: Vec<ComponentRecord>,
    options: InstallOptions,
    journals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub managed: bool,
    pub installed_at: Option<String>,
    pub components: Vec<ComponentRecord>,
    pub options: Option<InstallOptions>,
    pub backups: Vec<String>,
    pub added_files: usize,
    pub installed_exe: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReport {
    pub placed: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
    pub proxy: String,
}

pub const GAME_FOLDER: &str = "_DLSS5AIO";
const LEGACY_GAME_FOLDER: &str = "_NeuroDeck";

/// Older installs wrote `_NeuroDeck`; adopt it so their journals keep working.
pub fn neuro_dir(dir: &Path) -> PathBuf {
    let current = dir.join(GAME_FOLDER);
    let legacy = dir.join(LEGACY_GAME_FOLDER);
    if !current.exists() && legacy.is_dir() {
        let _ = std::fs::rename(&legacy, &current);
    }
    current
}

/// Absolute paths inside old journals still point at `_NeuroDeck`.
fn modernize(path: &str) -> String {
    path.replace(LEGACY_GAME_FOLDER, GAME_FOLDER)
}

pub fn manifest_path(dir: &Path) -> PathBuf {
    neuro_dir(dir).join("manifest.json")
}

pub fn load_manifest(dir: &Path) -> Option<Manifest> {
    std::fs::read_to_string(manifest_path(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

fn save_manifest(dir: &Path, m: &Manifest) -> Result<(), String> {
    let p = manifest_path(dir);
    std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&p, serde_json::to_string_pretty(m).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn state(dir: &Path) -> GameState {
    let Some(m) = load_manifest(dir) else {
        return GameState::default();
    };
    let mut added_files = 0;
    for j in &m.journals {
        if let Ok(t) = std::fs::read_to_string(neuro_dir(dir).join("backups").join(j).join("journal.json")) {
            if let Ok(journal) = serde_json::from_str::<Journal>(&t) {
                added_files += journal.added.len();
            }
        }
    }
    let backups = std::fs::read_dir(neuro_dir(dir).join("backups"))
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    GameState {
        managed: true,
        installed_at: Some(m.installed_at),
        components: m.components,
        options: Some(m.options),
        backups,
        added_files,
        installed_exe: None,
    }
}

fn extract_zip(zip: &Path, dest: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let status = std::process::Command::new("tar.exe")
        .arg("-xf")
        .arg(zip)
        .arg("-C")
        .arg(dest)
        .creation_flags(0x0800_0000)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        return Ok(());
    }
    let ps = format!(
        "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
        zip.display(),
        dest.display()
    );
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .creation_flags(0x0800_0000)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("could not extract {}", zip.display()))
    }
}

fn walk_dirs(root: &Path) -> Vec<(PathBuf, u32)> {
    let mut out = vec![(root.to_path_buf(), 0)];
    let mut i = 0;
    while i < out.len() && out.len() < 500 {
        let (dir, depth) = out[i].clone();
        i += 1;
        if depth >= 3 {
            continue;
        }
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    out.push((p, depth + 1));
                }
            }
        }
    }
    out
}

fn find_file(root: &Path, name: &str, depth: u32) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let Ok(rd) = std::fs::read_dir(root) else { return None };
    let mut dirs = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            dirs.push(p);
        } else if p.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case(name)).unwrap_or(false) {
            return Some(p);
        }
    }
    for d in dirs {
        if let Some(hit) = find_file(&d, name, depth - 1) {
            return Some(hit);
        }
    }
    None
}

fn staged_zip_root(zip: &Path, tag: &str) -> Result<PathBuf, String> {
    let root = paths::staging_dir().join(tag);
    if root.is_dir() {
        let _ = std::fs::remove_dir_all(&root);
    }
    extract_zip(zip, &root)?;
    if find_file(&root, "OptiScaler.dll", 2).is_some() {
        if let Some(dir) = root
            .read_dir()
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir() && p.join("OptiScaler.dll").is_file())
        {
            return Ok(dir);
        }
        return Ok(root);
    }
    Ok(root)
}

fn materialize(art: &Artifact, file_name: &str, tag: &str) -> Result<PathBuf, String> {
    let src = PathBuf::from(&art.path);
    if !src.is_file() {
        return Err(format!("artifact missing on disk: {}", art.path));
    }
    if src.extension().map(|e| e.eq_ignore_ascii_case("zip")).unwrap_or(false) {
        let root = staged_zip_root(&src, tag)?;
        find_file(&root, file_name, 3).ok_or_else(|| format!("{} not found inside {}", file_name, src.display()))
    } else {
        Ok(src)
    }
}

struct Writer<'a> {
    dir: &'a Path,
    added: Vec<String>,
    replaced: BTreeMap<String, String>,
    backup_root: PathBuf,
}

impl<'a> Writer<'a> {
    fn new(dir: &'a Path, backup_root: PathBuf) -> Self {
        Writer {
            dir,
            added: Vec::new(),
            replaced: BTreeMap::new(),
            backup_root,
        }
    }

    fn rel(&self, path: &Path) -> String {
        path.strip_prefix(self.dir)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }

    fn place(&mut self, src: &Path, rel_dest: &str) -> Result<(), String> {
        let dest = self.dir.join(rel_dest);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if dest.is_file() {
            let key = self.rel(&dest);
            if !self.replaced.contains_key(&key) {
                let bak = self.backup_root.join("files").join(rel_dest);
                std::fs::create_dir_all(bak.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::copy(&dest, &bak).map_err(|e| e.to_string())?;
                self.replaced.insert(key, bak.to_string_lossy().to_string());
            }
        } else {
            self.added.push(self.rel(&dest));
        }
        std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Remove a pre-existing file we are replacing conceptually (e.g. a foreign
    /// OptiScaler.asi): the original is backed up and restored on rollback/uninstall.
    fn disable(&mut self, rel_dest: &str) -> Result<(), String> {
        let dest = self.dir.join(rel_dest);
        if !dest.is_file() {
            return Ok(());
        }
        let key = self.rel(&dest);
        if !self.replaced.contains_key(&key) {
            let bak = self.backup_root.join("files").join(rel_dest);
            std::fs::create_dir_all(bak.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::copy(&dest, &bak).map_err(|e| e.to_string())?;
            self.replaced.insert(key, bak.to_string_lossy().to_string());
        }
        std::fs::remove_file(&dest).map_err(|e| e.to_string())
    }

    fn write_text(&mut self, rel_dest: &str, text: &str) -> Result<(), String> {
        let dest = self.dir.join(rel_dest);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if dest.is_file() {
            let key = self.rel(&dest);
            if !self.replaced.contains_key(&key) {
                let bak = self.backup_root.join("files").join(rel_dest);
                std::fs::create_dir_all(bak.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::copy(&dest, &bak).map_err(|e| e.to_string())?;
                self.replaced.insert(key, bak.to_string_lossy().to_string());
            }
        } else {
            self.added.push(self.rel(&dest));
        }
        std::fs::write(&dest, text).map_err(|e| e.to_string())
    }
}

fn reserved_proxies(dir: &Path, opts: &InstallOptions, has_opti: Option<&str>) -> BTreeMap<String, String> {
    let mut used: BTreeMap<String, String> = BTreeMap::new();
    if let Some(p) = has_opti {
        used.insert(p.to_string(), "optiscaler".into());
    }
    if (opts.install_feeder || !opts.uses_optiscaler()) && !used.contains_key("dxgi.dll") {
        used.insert("dxgi.dll".into(), "reshade".into());
    }
    if opts.install_sm86 && !used.contains_key("version.dll") {
        used.insert("version.dll".into(), "sm86".into());
    }
    let _ = dir;
    used
}

/// Proxy preference: dxgi first when nothing else claims it (the most
/// compatible name for DX11/DX12 games), then the fallbacks. ReShade takes
/// dxgi when the feeder is installed, which pushes OptiScaler to winmm.
fn optiscaler_proxy(dir: &Path, used: &BTreeMap<String, String>) -> String {
    if let Some((name, _)) = used.iter().find(|(_, who)| who.as_str() == "optiscaler") {
        return name.clone();
    }
    for cand in ["dxgi.dll", "winmm.dll", "dbghelp.dll", "winhttp.dll", "wininet.dll", "d3d12.dll", "version.dll"] {
        if !used.contains_key(cand) && !dir.join(cand).is_file() {
            return cand.to_string();
        }
    }
    "winmm.dll".into()
}

pub fn plan(game_dir: &Path, detection: &Detection, opts: &InstallOptions) -> InstallPlan {
    let mut steps = Vec::new();
    let mut warnings = Vec::new();
    let exe_dir = detection.exe_dir.clone().map(PathBuf::from);
    let provider = opts.provider().to_string();
    let existing_opti = detection.mods.optiscaler.clone();
    let existing_reshade = detection.mods.reshade.clone();

    if opts.uses_optiscaler() {
        let opti = opts
            .optiscaler_path
            .clone()
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| artifacts::best_optiscaler().map(|a| PathBuf::from(a.path)));
        let mut detail = opti
            .as_ref()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .unwrap_or_else(|| "no artifact - download or import required".into());
        if let Some(old) = &existing_opti {
            detail.push_str(&format!(" \u{2014} updates the existing copy at {old} in place (backed up first)"));
        }
        if detection.mods.optiscaler_asi {
            detail.push_str(" \u{2014} an existing OptiScaler add-on is disabled (backed up)");
        }
        steps.push(PlanStep {
            id: "optiscaler".into(),
            title: "OptiScaler NR".into(),
            detail,
            available: opti.is_some() || crate::registry::cached_downloadable("optiscaler-wilsjo2"),
        });

        let nr_compat = artifacts::best("runtime-nr-compat", None);
        let nr_nvidia = artifacts::best("runtime-nr-nvidia", None);
        let mut nr_detail = format!(
            "compat {} / nvidia {}",
            nr_compat.as_ref().map(|a| a.version.clone()).unwrap_or_else(|| "-".into()),
            nr_nvidia.as_ref().map(|a| a.version.clone()).unwrap_or_else(|| "-".into())
        );
        if detection.mods.nr_runtime {
            nr_detail.push_str(" \u{2014} replaces the existing nvngx_dlssnr.dll (backed up)");
        }
        steps.push(PlanStep {
            id: "runtime-nr".into(),
            title: "NR runtime (nvngx_dlssnr.dll)".into(),
            detail: nr_detail,
            available: nr_compat.is_some() || nr_nvidia.is_some() || crate::registry::cached_downloadable("runtime-nr-compat"),
        });
        steps.push(PlanStep {
            id: "runtime-dlss".into(),
            title: "DLSS runtime (nvngx_dlss.dll)".into(),
            detail: artifacts::best("runtime-dlss", None)
                .map(|a| format!("{}", a.version))
                .unwrap_or_else(|| "from the Streamline package".into()),
            available: artifacts::best("runtime-dlss", None).is_some()
                || artifacts::best("streamline", None).is_some()
                || crate::registry::cached_downloadable("runtime-dlss"),
        });
        if opts.fg_output == "xefg" {
            let sdk = artifacts::best("xess-sdk", None);
            steps.push(PlanStep {
                id: "xefg".into(),
                title: "XeFG output (Intel XeSS FG)".into(),
                detail: match (&sdk, opts.xess_libs) {
                    (Some(a), true) => format!("libxess_fg + libxell from XeSS SDK {}", a.version),
                    _ => "libxess_fg + libxell from the OptiScaler package".into(),
                },
                available: !opts.xess_libs || sdk.is_some() || crate::registry::cached_downloadable("xess-sdk"),
            });
        }
        if opts.dlss_enabler {
            steps.push(PlanStep {
                id: "dlss-enabler".into(),
                title: "DLSS Enabler (Artur)".into(),
                detail: "dlss-enabler-headless.dll into OptiScaler".into(),
                available: artifacts::best("dlss-enabler", None).is_some(),
            });
        }
    } else {
        let art = renodx_artifact_for(&provider);
        steps.push(PlanStep {
            id: "renodx".into(),
            title: "RenoDX neural consumer".into(),
            detail: art
                .as_ref()
                .map(|a| {
                    format!(
                        "{} \u{2014} {}{}",
                        a.variant.clone().unwrap_or_else(|| "dlss5".into()),
                        a.path.rsplit('\\').next().unwrap_or(""),
                        a.note.as_ref().map(|n| format!(" ({})", n)).unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| "no renodx artifact imported".into()),
            available: art.is_some() || crate::registry::cached_downloadable("renodx-rhi"),
        });
        steps.push(PlanStep {
            id: "runtime-nr".into(),
            title: "NGX runtimes for the feeder".into(),
            detail: "nvngx_dlss.dll + nvngx_dlssnr.dll beside the ReShade host".into(),
            available: artifacts::best("runtime-nr-compat", None).is_some() || artifacts::best("runtime-nr-nvidia", None).is_some(),
        });
    }

    if opts.install_feeder || !opts.uses_optiscaler() {
        let mut detail = if opts.uses_optiscaler() {
            "feeder installer, consumer = OptiScaler (UAC prompt for the Vulkan layer)".to_string()
        } else {
            "feeder installer, consumer = RenoDX (UAC prompt for the Vulkan layer)".to_string()
        };
        if let Some(r) = &existing_reshade {
            detail.push_str(&format!(" \u{2014} upgrades the existing ReShade at {r}, its ini is kept and merged"));
        }
        if detection.mods.feeder {
            detail.push_str(" \u{2014} updates the existing feeder add-on");
        }
        steps.push(PlanStep {
            id: "feeder".into(),
            title: "ReShade + DLSS5-Feeder".into(),
            detail,
            available: artifacts::best("installer-feeder", None).is_some()
                || crate::registry::cached_downloadable("installer-feeder"),
        });
    }
    if opts.install_sm86 && opts.uses_optiscaler() {
        let mut detail = artifacts::best("sm86", None)
            .map(|a| format!("version.dll + dlssg_sm86.ini ({})", a.version))
            .unwrap_or_else(|| "not in payload".into());
        if detection.mods.dlssg_sm86 {
            detail.push_str(" \u{2014} updates the existing files in place");
        }
        steps.push(PlanStep {
            id: "sm86".into(),
            title: "DLSSG sm86 frame generation".into(),
            detail,
            available: artifacts::best("sm86", None).is_some() || crate::registry::cached_downloadable("sm86"),
        });
    }
    if opts.streamline && opts.uses_optiscaler() {
        steps.push(PlanStep {
            id: "streamline".into(),
            title: "Streamline set".into(),
            detail: artifacts::best("streamline", None)
                .map(|a| format!("sl.*.dll + nvngx_dlssg ({})", a.version))
                .unwrap_or_else(|| "sl.*.dll into OptiScaler\\streamline".into()),
            available: artifacts::best("streamline", None).is_some() || crate::registry::cached_downloadable("streamline"),
        });
    }
    if opts.optipatcher && opts.uses_optiscaler() {
        steps.push(PlanStep {
            id: "optipatcher".into(),
            title: "OptiPatcher".into(),
            detail: "OptiScaler\\plugins + LoadAsiPlugins".into(),
            available: artifacts::best("optipatcher", None).is_some() || crate::registry::cached_downloadable("optipatcher"),
        });
    }
    if opts.runtime_dlssd {
        steps.push(PlanStep {
            id: "runtime-dlssd".into(),
            title: "Ray Reconstruction runtime".into(),
            detail: "nvngx_dlssd.dll into the game folder".into(),
            available: artifacts::best("runtime-dlssd", None).is_some() || artifacts::best("streamline", None).is_some(),
        });
    }

    if detection.arch.as_deref() == Some("x86") {
        warnings.push("32-bit game: NR and OptiScaler are x64-only".into());
    }
    if detection.upscalers.is_empty() && !opts.install_feeder {
        warnings.push("no upscaler detected: enable the feeder or NR has no inputs".into());
    }
    if !opts.uses_optiscaler() && !opts.install_feeder {
        warnings.push("RenoDX runs through the feeder: enable it or nothing will be installed".into());
    }
    if opts.fg_output == "xefg" {
        warnings.push("XeFG only presents in Borderless Fullscreen, and non-Intel GPUs need the XeSS 3.x libraries".into());
    }
    if detection.mods.dxvk {
        warnings.push("DXVK found: OptiScaler hooks D3D/DXGI and does not attach through a Vulkan wrapper - remove DXVK or use the feeder's Vulkan path".into());
    }
    if detection.mods.dgvoodoo {
        warnings.push("dgVoodoo2 found: its wrapped output bypasses OptiScaler's hooks".into());
    }
    if detection.mods.dlss_enabler {
        warnings.push("DLSS Enabler detected (nvngx.dll): it intercepts NGX calls too and can double-hook beside NR - removing it is safer".into());
    }
    if let Some(r) = &existing_reshade {
        if !(opts.install_feeder || !opts.uses_optiscaler()) {
            warnings.push(format!("ReShade at {r} is left untouched (the feeder is off; enable it to update ReShade)"));
        }
    }
    if let Some(d) = &exe_dir {
        if !d.is_dir() {
            warnings.push("game folder missing".into());
        }
    } else {
        warnings.push("could not locate the game executable".into());
    }
    let has_opti = detection.mods.optiscaler.clone();
    let used = reserved_proxies(game_dir, opts, has_opti.as_deref());
    let proxy = optiscaler_proxy(game_dir, &used);

    InstallPlan {
        exe_dir: detection.exe_dir.clone(),
        exe: detection.exe.clone(),
        proxy,
        steps,
        warnings,
    }
}

pub type Progress<'a> = &'a dyn Fn(&str, &str, &str, bool, Option<String>);

pub fn run(progress: Progress, detection: &Detection, opts: &InstallOptions) -> Result<InstallReport, String> {
    let emit = |stage: &str, title: &str, detail: &str, done: bool, error: Option<String>| {
        progress(stage, title, detail, done, error);
    };

    let dir = detection
        .exe_dir
        .as_ref()
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .ok_or_else(|| "game exe folder not found".to_string())?;

    let ts = chrono_like_stamp();
    let backup_root = neuro_dir(&dir).join("backups").join(&ts);
    std::fs::create_dir_all(&backup_root).map_err(|e| e.to_string())?;

    let mut placed: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut components: Vec<ComponentRecord> = Vec::new();
    let mut writer = Writer::new(&dir, backup_root.clone());

    // Nothing is bundled with the app: anything missing locally is fetched from
    // its public source here, so a fresh machine only needs the Install click.
    let ensure = |kind: &str, variant: Option<&str>, component: &str, label: &str| -> Option<Artifact> {
        if let Some(a) = artifacts::best(kind, variant) {
            return Some(a);
        }
        emit("download", "Downloading", label, false, None);
        let dl = |p: artifacts::DownloadProgress| {
            if p.done {
                emit(
                    "download",
                    "Downloaded",
                    &format!("{} {}", label, p.error.unwrap_or_default()),
                    true,
                    None,
                );
            }
        };
        match crate::download::component(component, variant, "stable", &dl) {
            Ok(a) => {
                emit("download", "Downloaded", &format!("{} {}", label, a.version), true, None);
                Some(a)
            }
            Err(e) => {
                emit("download", "Download failed", &format!("{}: {}", label, e), true, Some(e));
                None
            }
        }
    };

    // 1. OptiScaler (skipped entirely on the RenoDX provider path)
    let mut proxy = String::from("(none)");
    let mut opti_root: Option<PathBuf> = None;
    let mut opti_zip: Option<PathBuf> = None;
    if opts.uses_optiscaler() {
        emit("optiscaler", "OptiScaler NR", "staging package", false, None);
        let opti_art = opts
            .optiscaler_path
            .clone()
            .and_then(|p| artifacts::list().into_iter().find(|a| a.path == p))
            .or_else(artifacts::best_optiscaler)
            .or_else(|| ensure("optiscaler-wilsjo2", None, "optiscaler-wilsjo2", "OptiScaler"))
            .ok_or("no OptiScaler build available and the download failed")?;
        let src_zip = PathBuf::from(&opti_art.path);
        if src_zip.is_file() {
            opti_zip = Some(src_zip.clone());
        }
        let stage = staged_zip_root(&src_zip, "optiscaler")?;
        let mut root = stage.clone();
        if !root.join("OptiScaler.dll").is_file() {
            if let Some(dir2) = find_file(&root, "OptiScaler.dll", 2).and_then(|p| p.parent().map(|d| d.to_path_buf())) {
                root = dir2;
            }
        }
        let has_opti = detection.mods.optiscaler.clone();
        let used = reserved_proxies(&dir, opts, has_opti.as_deref());
        proxy = optiscaler_proxy(&dir, &used);

        let opti_dll = root.join("OptiScaler.dll");
        if !opti_dll.is_file() {
            return Err(format!("OptiScaler.dll not found in {}", src_zip.display()));
        }
        writer.place(&opti_dll, &proxy)?;
        placed.push(proxy.clone());
        components.push(ComponentRecord {
            kind: "optiscaler".into(),
            version: opti_art.version.clone(),
        });

        // A hand-installed OptiScaler loaded as a ReShade add-on would run beside ours.
        for cand in ["OptiScaler.asi", "OptiScaler.dll"] {
            let p = dir.join(cand);
            if p.is_file() && !pe::scan_file_markers(&p, &["OptiScaler"], 40 * 1_048_576).is_empty() {
                writer.disable(cand)?;
                emit("optiscaler", "OptiScaler NR", &format!("existing {} disabled (backed up)", cand), true, None);
            }
        }

        let keep_ini = dir.join("OptiScaler.ini").is_file() && !opts.force;
        let mut stack = vec![(root.clone(), 0u32)];
        while let Some((d, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if p.is_dir() {
                    if depth < 4 {
                        stack.push((p, depth + 1));
                    }
                    continue;
                }
                let lname = name.to_lowercase();
                if lname == "optiscaler.dll" {
                    continue;
                }
                if lname == "optiscaler.ini" {
                    if keep_ini {
                        skipped.push(name);
                        continue;
                    }
                    writer.place(&p, "OptiScaler.ini")?;
                    continue;
                }
                if lname == "setup_windows.bat" || lname == "setup_linux.sh" {
                    continue;
                }
                let rel = p
                    .strip_prefix(&root)
                    .map(|r| r.to_string_lossy().to_string())
                    .unwrap_or(name.clone());
                writer.place(&p, &rel)?;
            }
        }
        emit("optiscaler", "OptiScaler NR", &format!("placed as {} ({})", proxy, opti_art.version), true, None);
        opti_root = Some(root.clone());
    } else {
        emit("optiscaler", "RenoDX provider", "OptiScaler not installed (RenoDX runs through the feeder)", true, None);
    }

    // 2. NR runtime
    let gpu_family = crate::gpu::gpu_info().family;
    let preferred_kind = if gpu_family == "blackwell" || opts.rtx40_mfg {
        "runtime-nr-nvidia"
    } else {
        "runtime-nr-compat"
    };
    let want_kind = match ensure(preferred_kind, None, preferred_kind, "NR runtime") {
        Some(_) => preferred_kind,
        None => {
            let other = if preferred_kind == "runtime-nr-compat" {
                "runtime-nr-nvidia"
            } else {
                "runtime-nr-compat"
            };
            if ensure(other, None, other, "NR runtime").is_some() {
                other
            } else {
                preferred_kind
            }
        }
    };
    emit("runtime-nr", "NR runtime", want_kind, false, None);
    match artifacts::best(want_kind, None) {
        Some(art) => {
            let dll = materialize(&art, "nvngx_dlssnr.dll", "nr")?;
            writer.place(&dll, "nvngx_dlssnr.dll")?;
            placed.push("nvngx_dlssnr.dll".into());
            components.push(ComponentRecord {
                kind: want_kind.into(),
                version: art.version.clone(),
            });
            emit("runtime-nr", "NR runtime", &format!("{} ({})", dll.display(), art.version), true, None);
        }
        None => {
            warnings.push("NR runtime missing: import nvngx_dlssnr.dll or download it in Components".into());
            emit("runtime-nr", "NR runtime", "missing", true, Some("not available".into()));
        }
    }

    // 2b. NR helper for builds that expect it
    if let Some(r) = &opti_root {
        if let Some(helper) = find_file(r, "nvngx.dll_dlssnr.dll", 2) {
            writer.place(&helper, "nvngx.dll_dlssnr.dll")?;
            placed.push("nvngx.dll_dlssnr.dll".into());
        }
    }

    // 3. DLSS runtime
    let _ = ensure("runtime-dlss", None, "runtime-dlss", "DLSS runtime");
    if let Some(art) = artifacts::best("runtime-dlss", None).or_else(|| artifacts::best("streamline", None)) {
        emit("runtime-dlss", "DLSS runtime", &art.version, false, None);
        if let Ok(dll) = materialize(&art, "nvngx_dlss.dll", "dlss") {
            writer.place(&dll, "nvngx_dlss.dll")?;
            placed.push("nvngx_dlss.dll".into());
            components.push(ComponentRecord {
                kind: "runtime-dlss".into(),
                version: art.version.clone(),
            });
            emit("runtime-dlss", "DLSS runtime", "placed", true, None);
        }
    } else {
        warnings.push("nvngx_dlss.dll not available (needed for the DLSS input)".into());
    }

    // 4. sm86 frame generation (OptiScaler path only)
    if opts.install_sm86 && opts.uses_optiscaler() {
        if let Some(art) = ensure("sm86", None, "sm86", "DLSSG sm86") {
            emit("sm86", "DLSSG sm86", &art.version, false, None);
            let base = PathBuf::from(&art.path);
            let base = if base.is_file() { base.parent().map(|p| p.to_path_buf()).unwrap_or(base) } else { base };
            for f in ["version.dll", "dlssg_sm86.ini"] {
                let src = base.join(f);
                if src.is_file() {
                    writer.place(&src, f)?;
                    placed.push(f.into());
                } else if let Ok(found) = materialize(&art, f, "sm86") {
                    writer.place(&found, f)?;
                    placed.push(f.into());
                }
            }
            components.push(ComponentRecord {
                kind: "sm86".into(),
                version: art.version.clone(),
            });
            emit("sm86", "DLSSG sm86", "version.dll + ini placed", true, None);
        } else {
            warnings.push("sm86 artifact not available".into());
        }
    }

    // 5. Streamline set (OptiScaler path only)
    if opts.streamline && opts.uses_optiscaler() {
        if let Some(art) = ensure("streamline", None, "streamline", "Streamline SDK") {
            let zip = PathBuf::from(&art.path);
            if zip.is_file() {
                let root = staged_zip_root(&zip, "streamline")?;
                emit("streamline", "Streamline", "copying sl.*.dll", false, None);
                let mut copied = 0;
                for (root_dir, depth) in walk_dirs(&root) {
                    let _ = depth;
                    for entry in std::fs::read_dir(&root_dir).into_iter().flatten().flatten() {
                        let p = entry.path();
                        let name = entry.file_name().to_string_lossy().to_string();
                        if !p.is_file() || !name.to_lowercase().ends_with(".dll") {
                            continue;
                        }
                        let lname = name.to_lowercase();
                        if lname.starts_with("sl.") {
                            writer.place(&p, &format!("OptiScaler\\streamline\\{}", name))?;
                            copied += 1;
                        } else if lname == "nvngx_dlssg.dll" {
                            writer.place(&p, &name)?;
                        } else if lname == "nvngx_dlss.dll" && !dir.join("nvngx_dlss.dll").is_file() {
                            writer.place(&p, &name)?;
                        } else if lname == "nvngx_dlssnr.dll" && !dir.join("nvngx_dlssnr.dll").is_file() {
                            writer.place(&p, &name)?;
                        } else if lname == "nvngx_dlssd.dll" && opts.runtime_dlssd {
                            writer.place(&p, &name)?;
                        }
                    }
                }
                placed.push(format!("OptiScaler\\streamline ({} dlls)", copied));
                components.push(ComponentRecord {
                    kind: "streamline".into(),
                    version: art.version.clone(),
                });
                emit("streamline", "Streamline", &format!("{} dlls", copied), true, None);
            }
        } else {
            warnings.push("streamline set not available".into());
        }
    }

    // 6. OptiPatcher (OptiScaler path only)
    if opts.optipatcher && opts.uses_optiscaler() {
        match ensure("optipatcher", None, "optipatcher", "OptiPatcher") {
            Some(art) => {
                writer.place(Path::new(&art.path), "OptiScaler\\plugins\\OptiPatcher.asi")?;
                placed.push("OptiScaler\\plugins\\OptiPatcher.asi".into());
                components.push(ComponentRecord {
                    kind: "optipatcher".into(),
                    version: art.version.clone(),
                });
            }
            None => warnings.push("OptiPatcher not available".into()),
        }
    }

    // 6b. XeSS frame-generation libraries (real XeFG, incl. non-Intel via XeSS 3.x DP4a)
    if opts.uses_optiscaler() && opts.fg_output == "xefg" {
        let mut sources: Vec<(String, PathBuf)> = Vec::new();
        if opts.xess_libs {
            if let Some(sdk) = ensure("xess-sdk", None, "xess-sdk", "Intel XeSS SDK") {
                emit("xefg", "XeFG libraries", &format!("XeSS SDK {}", sdk.version), false, None);
                for name in ["libxess_fg.dll", "libxell.dll"] {
                    if let Ok(p) = materialize(&sdk, name, "xess") {
                        sources.push((name.to_string(), p));
                    }
                }
            } else {
                warnings.push("XeSS SDK not available: falling back to the OptiScaler bundled XeFG libraries".into());
            }
        }
        if sources.is_empty() {
            let pkg = dir.join("OptiScaler");
            for name in ["libxess_fg.dll", "libxell.dll"] {
                let p = pkg.join(name);
                if p.is_file() {
                    sources.push((name.to_string(), p));
                }
            }
        }
        for (name, path) in &sources {
            writer.place(path, &format!("OptiScaler\\{}", name))?;
            placed.push(format!("OptiScaler\\{}", name));
        }
        components.push(ComponentRecord {
            kind: "xefg-libs".into(),
            version: if opts.xess_libs {
                artifacts::best("xess-sdk", None).map(|a| a.version).unwrap_or_else(|| "bundled".into())
            } else {
                "bundled".into()
            },
        });
        emit("xefg", "XeFG libraries", &format!("{} files", sources.len()), true, None);

        // XeLL pacing for XeFG: fakenvapi hooks Reflex and injects XeLL
        if let Some(fnapi) = ensure("fakenvapi", None, "fakenvapi", "Fakenvapi (XeLL)") {
            let mut got = 0;
            for name in ["fakenvapi.dll", "fakenvapi.ini"] {
                if let Ok(p) = materialize(&fnapi, name, "fakenvapi") {
                    writer.place(&p, name)?;
                    placed.push(name.to_string());
                    got += 1;
                }
            }
            if got > 0 {
                components.push(ComponentRecord {
                    kind: "fakenvapi".into(),
                    version: fnapi.version.clone(),
                });
                emit("xefg", "XeLL", "fakenvapi.dll placed (Reflex → XeLL)", true, None);
            }
        } else {
            warnings.push("XeFG works without XeLL, but Reflex→XeLL pacing needs fakenvapi.dll".into());
        }
    }

    // 6c. DLSS Enabler (Artur) for FSR FG / MFG replacement
    if opts.uses_optiscaler() && opts.dlss_enabler {
        let artifact = artifacts::best("dlss-enabler", None);
        let from_game = dir.join("dlss-enabler-headless.dll");
        if let Some(art) = artifact {
            writer.place(Path::new(&art.path), "OptiScaler\\dlss-enabler-headless.dll")?;
            placed.push("OptiScaler\\dlss-enabler-headless.dll".into());
            components.push(ComponentRecord {
                kind: "dlss-enabler".into(),
                version: art.version.clone(),
            });
            emit("dlss-enabler", "DLSS Enabler", "OptiScaler\\dlss-enabler-headless.dll", true, None);
        } else if from_game.is_file() {
            // Artur's installer drops the DLL in the game folder — reuse it
            writer.place(&from_game, "OptiScaler\\dlss-enabler-headless.dll")?;
            placed.push("OptiScaler\\dlss-enabler-headless.dll (from the game folder)".into());
            components.push(ComponentRecord {
                kind: "dlss-enabler".into(),
                version: "from game folder".into(),
            });
            emit("dlss-enabler", "DLSS Enabler", "picked up from the game folder", true, None);
        } else {
            warnings.push(
                "DLSS Enabler: download and run its installer (Components > DLSS Enabler), then install again — or drop dlss-enabler-headless.dll into the game folder".into(),
            );
            emit(
                "dlss-enabler",
                "DLSS Enabler",
                "installer not run yet",
                true,
                Some("needs the DLSS Enabler installer".into()),
            );
        }
    }

    // 7. ini tuning (OptiScaler only)
    if opts.uses_optiscaler() {
        let ini_path = dir.join("OptiScaler.ini");
        let mut lines: Vec<String> = if ini_path.is_file() {
            std::fs::read_to_string(&ini_path)
                .map_err(|e| e.to_string())?
                .lines()
                .map(|l| l.to_string())
                .collect()
        } else {
            Vec::new()
        };
        set_ini(&mut lines, "Upscalers", "Dx12Upscaler", "dlss");
        set_ini(&mut lines, "Upscalers", "Dx11Upscaler", "dlss_12");
        set_ini(&mut lines, "DlssNr", "Enabled", if opts.nr_enabled { "true" } else { "false" });
        set_ini(&mut lines, "DlssNr", "RunBeforeSR", "true");
        set_ini(&mut lines, "DlssNr", "FinishedPicture", "false");
        set_ini(&mut lines, "DlssNr", "DeferredDLSS", "false");
        let (scale, passes) = match opts.preset.as_str() {
            "ultra" => ("1.0", "2"),
            "quality" => ("1.0", "1"),
            "performance" => ("0.5", "1"),
            _ => ("0.67", "1"),
        };
        set_ini(&mut lines, "DlssNr", "WorkingScale", scale);
        set_ini(&mut lines, "DlssNr", "Passes", passes);
        set_ini(&mut lines, "Log", "LogToFile", "true");
        set_ini(&mut lines, "Log", "LogLevel", "2");
        set_ini(&mut lines, "Hotfix", "CheckForUpdate", "false");
        if opts.optipatcher {
            set_ini(&mut lines, "Plugins", "LoadAsiPlugins", "true");
        }
        if opts.rtx40_mfg {
            set_ini(&mut lines, "DLSSG", "AdaMfgUnlock", "true");
        }
        let fg_output = if opts.fg_output.is_empty() { "auto" } else { opts.fg_output.as_str() };
        let fg_input = if opts.fg_input.is_empty() { "auto" } else { opts.fg_input.as_str() };
        if fg_output != "auto" || fg_input != "auto" || opts.hudfix {
            set_ini(&mut lines, "FrameGen", "Enabled", if fg_output == "none" && fg_input == "none" { "false" } else { "true" });
            set_ini(&mut lines, "FrameGen", "FGInput", fg_input);
            set_ini(&mut lines, "FrameGen", "FGOutput", fg_output);
            if !opts.fg_replacement.is_empty() && opts.fg_replacement != "auto" {
                let value = match opts.fg_replacement.as_str() {
                    "none" => "None",
                    "nukems" => "Nukems",
                    "arturs" => "Arturs",
                    "ffx" => "FFX",
                    "combo" => "Combo",
                    other => other,
                };
                set_ini(&mut lines, "FrameGen", "FGNvngxReplacement", value);
            }
        }
        if opts.hudfix {
            set_ini(&mut lines, "OptiFG", "HUDFix", "true");
        }
        writer.write_text("OptiScaler.ini", &(lines.join("\r\n") + "\r\n"))?;
    }

    // 8. ReShade post-fixes
    let reshade_ini = dir.join("ReShade.ini");
    if reshade_ini.is_file() {
        if let Ok(text) = std::fs::read_to_string(&reshade_ini) {
            let mut out: Vec<String> = Vec::new();
            for line in text.lines() {
                if line.trim_start().to_lowercase().starts_with("loadfromdllmain") {
                    continue;
                }
                if line.trim_start().to_lowercase().starts_with("disabledaddons") {
                    let kept: Vec<&str> = line
                        .split('=')
                        .nth(1)
                        .unwrap_or("")
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("Generic Depth"))
                        .collect();
                    if kept.is_empty() {
                        continue;
                    }
                    out.push(format!("DisabledAddons={}", kept.join(",")));
                    continue;
                }
                out.push(line.to_string());
            }
            writer.write_text("ReShade.ini", &(out.join("\r\n") + "\r\n"))?;
        }
    }
    let preset = dir.join("ReShadePreset.ini");
    if preset.is_file() {
        if let Ok(text) = std::fs::read_to_string(&preset) {
            if text.contains("DLSS5_MV_PROVIDER")
                && !text.contains("DLSS5_MV_PROVIDER=3")
                && text.contains("Lumenite_Kernel")
                && !text.contains("Lumenite_QuantMotion")
            {
                let fixed = text.replace("DLSS5_MV_PROVIDER=4", "DLSS5_MV_PROVIDER=3");
                writer.write_text("ReShadePreset.ini", &fixed)?;
            }
        }
    }

    // 9. feeder installer (always on the RenoDX path; otherwise only when asked,
    // which is what decides whether ReShade is installed at all)
    if opts.install_feeder || !opts.uses_optiscaler() {
        let script = ensure("installer-feeder", None, "installer-feeder", "feeder installer script")
            .map(|a| PathBuf::from(a.path))
            .filter(|p| p.is_file());
        match script {
            Some(script) => {
                for (kind, component, label) in [
                    ("feeder", "feeder", "DLSS5-Feeder"),
                    ("reshade", "reshade", "ReShade"),
                    ("lumenite", "lumenite", "LumeniteFX"),
                ] {
                    let _ = ensure(kind, None, component, label);
                }
                let deps = build_feeder_deps()?;
                let nr = dir.join("nvngx_dlssnr.dll");
                emit("feeder", "ReShade + feeder", "running installer (UAC may prompt)", false, None);
                let mut cmd = std::process::Command::new("powershell.exe");
                let consumer = if opts.uses_optiscaler() { "OptiScaler" } else { "RenoDX" };
                cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
                    .arg(&script)
                    .arg(detection.exe.clone().unwrap_or_else(|| dir.to_string_lossy().to_string()))
                    .args(["-Consumer", consumer, "-Yes", "-NoPause", "-NoVerify"])
                    .arg("-LocalFiles")
                    .arg(&deps);
                if nr.is_file() {
                    cmd.arg("-DlssNrDll").arg(&nr);
                }
                if opts.uses_optiscaler() {
                    // Feed the script the build the user picked; otherwise it downloads
                    // Dagherbou/OptiScaler_DLSSNR (~130 MB) and silently replaces it.
                    if let Some(zip) = opti_zip.as_ref() {
                        cmd.arg("-OptiScalerZip").arg(zip);
                    }
                }
                if !opts.uses_optiscaler() {
                    let renodx = renodx_artifact_for(opts.provider())
                        .or_else(|| ensure("renodx", None, "renodx-rhi", "RenoDX DLSS 5"));
                    match renodx {
                        Some(art) => {
                            cmd.arg("-RenoDxAddon").arg(&art.path);
                            components.push(ComponentRecord {
                                kind: "renodx".into(),
                                version: art.variant.clone().unwrap_or_else(|| art.version.clone()),
                            });
                        }
                        None => warnings.push(
                            "RenoDX add-on file not found: import renodx-dlss5*.addon64 in Components, or pick the OptiScaler provider".into(),
                        ),
                    }
                }
                if opts.force {
                    cmd.arg("-Force");
                }
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
                let out = cmd.output().map_err(|e| e.to_string())?;
                let log = String::from_utf8_lossy(&out.stdout).to_string();
                for line in log.lines() {
                    progress("feeder", "ReShade + feeder", line, false, None);
                }
                if !out.status.success() {
                    warnings.push("feeder installer reported a failure - see the log".into());
                    emit("feeder", "ReShade + feeder", "installer failed", true, Some(log.chars().take(400).collect()));
                } else {
                    placed.push("ReShade + DLSS5 feeder".into());
                    components.push(ComponentRecord {
                        kind: "feeder".into(),
                        version: artifacts::best("feeder", None).map(|a| a.version).unwrap_or_else(|| "local".into()),
                    });
                    emit("feeder", "ReShade + feeder", "installed", true, None);
                }
            }
            None => {
                warnings.push("feeder installer script not in payload (import Test\\OptiScaler-DLSS5-Integrated)".into());
                skipped.push("feeder".into());
            }
        }
    }

    // 10. journal + manifest
    let journal = Journal {
        ts: ts.clone(),
        added: writer.added.clone(),
        replaced: writer.replaced.clone(),
    };
    std::fs::write(
        backup_root.join("journal.json"),
        serde_json::to_string_pretty(&journal).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let mut manifest = load_manifest(&dir).unwrap_or_default();
    manifest.installed_at = ts.clone();
    manifest.options = opts.clone();
    manifest.components = components;
    if !manifest.journals.contains(&ts) {
        manifest.journals.push(ts.clone());
    }
    save_manifest(&dir, &manifest)?;

    Ok(InstallReport {
        placed,
        skipped,
        warnings,
        proxy,
    })
}

fn build_feeder_deps() -> Result<PathBuf, String> {
    let dir = paths::staging_dir().join("feederdeps");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for kind in ["feeder", "reshade", "lumenite", "runtime-dlss", "runtime-nr-compat", "runtime-nr-nvidia"] {
        if let Some(art) = artifacts::best(kind, None) {
            let src = PathBuf::from(&art.path);
            if !src.is_file() {
                continue;
            }
            let dest = dir.join(src.file_name().unwrap());
            if dest.is_file() {
                continue;
            }
            if std::fs::hard_link(&src, &dest).is_err() {
                std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(dir)
}

fn chrono_like_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs = now % 86_400;
    let days = now / 86_400;
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    format!("{:05}-{:02}{:02}{:02}", days, h, m, s)
}

/// Writes one file inside the game folder through the same journal/backup path
/// the installer uses, so "Roll back" also reverts settings edits.
pub fn write_journaled(dir: &Path, rel: &str, content: &str) -> Result<(), String> {
    let ts = format!("{}-edit", chrono_like_stamp());
    let backup_root = neuro_dir(dir).join("backups").join(&ts);
    std::fs::create_dir_all(&backup_root).map_err(|e| e.to_string())?;
    let mut writer = Writer::new(dir, backup_root.clone());
    writer.write_text(rel, content)?;
    let journal = Journal {
        ts: ts.clone(),
        added: writer.added.clone(),
        replaced: writer.replaced.clone(),
    };
    std::fs::write(
        backup_root.join("journal.json"),
        serde_json::to_string_pretty(&journal).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut manifest = load_manifest(dir).unwrap_or_default();
    if !manifest.journals.contains(&ts) {
        manifest.journals.push(ts.clone());
    }
    save_manifest(dir, &manifest)
}

pub fn rollback(dir: &Path) -> Result<String, String> {
    let mut manifest = load_manifest(dir).ok_or("no DLSS5 AIO install found in this folder")?;
    let mut journals = manifest.journals.clone();
    let ts = journals.pop().ok_or("no backup snapshot to roll back to")?;
    let root = neuro_dir(dir).join("backups").join(&ts);
    let text = std::fs::read_to_string(root.join("journal.json")).map_err(|e| e.to_string())?;
    let journal: Journal = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    for rel in &journal.added {
        let p = dir.join(rel);
        if p.is_file() {
            let _ = std::fs::remove_file(&p);
        }
    }
    for (rel, bak) in &journal.replaced {
        let bak = modernize(bak);
        if Path::new(&bak).is_file() {
            let dest = dir.join(rel);
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::copy(&bak, &dest).map_err(|e| e.to_string())?;
        }
    }
    manifest.journals = journals;
    save_manifest(dir, &manifest)?;
    let _ = std::fs::rename(&root, neuro_dir(dir).join("backups").join(format!("{}-rolledback", ts)));
    Ok(format!("rolled back {}", ts))
}

pub fn uninstall(dir: &Path) -> Result<String, String> {
    let manifest = load_manifest(dir).ok_or("no DLSS5 AIO install found in this folder")?;
    let mut removed = 0;
    for ts in manifest.journals.iter().rev() {
        let root = neuro_dir(dir).join("backups").join(format!("{}-rolledback", ts));
        let root = if root.is_dir() { root } else { neuro_dir(dir).join("backups").join(ts) };
        let Ok(text) = std::fs::read_to_string(root.join("journal.json")) else { continue };
        let Ok(journal) = serde_json::from_str::<Journal>(&text) else { continue };
        for rel in &journal.added {
            let p = dir.join(rel);
            if p.is_file() {
                let _ = std::fs::remove_file(&p);
                removed += 1;
            }
        }
        for (rel, bak) in &journal.replaced {
            let bak = modernize(bak);
            if Path::new(&bak).is_file() {
                let dest = dir.join(rel);
                let _ = std::fs::copy(&bak, &dest);
            }
        }
    }
    let _ = std::fs::remove_dir_all(neuro_dir(dir));
    Ok(format!("removed {} files", removed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect;

    #[test]
    fn renodx_provider_plan_skips_optiscaler() {
        let base = std::env::temp_dir().join("neurodeck-plan-test");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("Game.exe"), vec![0u8; 1024]).unwrap();
        let det = Detection {
            exe: Some(game.join("Game.exe").to_string_lossy().to_string()),
            exe_dir: Some(game.to_string_lossy().to_string()),
            arch: Some("x64".into()),
            ..Default::default()
        };
        let opts = InstallOptions {
            nr_provider: "renodx-performance".into(),
            install_feeder: true,
            fg_output: "dlssg".into(),
            ..Default::default()
        };
        let renodx_plan = plan(&game, &det, &opts);
        let ids: Vec<&str> = renodx_plan.steps.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"renodx"), "renodx step present: {:?}", ids);
        assert!(ids.contains(&"feeder"));
        assert!(!ids.contains(&"optiscaler"), "no OptiScaler step on the RenoDX path");
        assert!(!ids.contains(&"sm86"));

        let opti_opts = InstallOptions {
            nr_provider: "optiscaler".into(),
            install_feeder: true,
            install_sm86: true,
            fg_output: "xefg".into(),
            ..Default::default()
        };
        let opti_plan = plan(&game, &det, &opti_opts);
        let opti_ids: Vec<&str> = opti_plan.steps.iter().map(|s| s.id.as_str()).collect();
        assert!(opti_ids.contains(&"optiscaler"));
        assert!(opti_ids.contains(&"xefg"));
        assert!(opti_ids.contains(&"sm86"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn reshade_only_comes_with_the_feeder() {
        let base = std::env::temp_dir().join("neurodeck-feeder-gate-test");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("Game.exe"), vec![0u8; 1024]).unwrap();
        let mut det = Detection {
            exe: Some(game.join("Game.exe").to_string_lossy().to_string()),
            exe_dir: Some(game.to_string_lossy().to_string()),
            arch: Some("x64".into()),
            ..Default::default()
        };
        det.upscalers = vec!["dlss".into()];

        // upscaler present + OptiScaler provider + feeder off → no ReShade/feeder step
        let opts = InstallOptions {
            nr_provider: "optiscaler".into(),
            install_feeder: false,
            ..Default::default()
        };
        let ids: Vec<String> = plan(&game, &det, &opts).steps.iter().map(|s| s.id.clone()).collect();
        assert!(!ids.contains(&"feeder".to_string()), "no feeder for a game with an upscaler: {:?}", ids);

        // RenoDX provider → feeder is forced even with the toggle off
        let reno = InstallOptions {
            nr_provider: "renodx-performance".into(),
            install_feeder: false,
            ..Default::default()
        };
        let ids: Vec<String> = plan(&game, &det, &reno).steps.iter().map(|s| s.id.clone()).collect();
        assert!(ids.contains(&"feeder".to_string()), "RenoDX needs the feeder: {:?}", ids);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[ignore]
    fn live_fresh_machine_installs_everything_by_download() {
        use crate::paths;
        use std::collections::HashSet;
        let store = paths::app_dir().join("artifacts.json");
        let backup = std::fs::read_to_string(&store).ok();
        let _ = std::fs::remove_file(&store);

        let base = std::env::temp_dir().join("neurodeck-fresh-install");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("Game.exe"), vec![0u8; 1024]).unwrap();
        let det = Detection {
            exe: Some(game.join("Game.exe").to_string_lossy().to_string()),
            exe_dir: Some(game.to_string_lossy().to_string()),
            arch: Some("x64".into()),
            ..Default::default()
        };
        let opts = InstallOptions {
            nr_provider: "optiscaler".into(),
            streamline: true,
            preset: "balanced".into(),
            ..Default::default()
        };
        let log = |stage: &str, title: &str, detail: &str, done: bool, _e: Option<String>| {
            if done || stage == "download" {
                println!("[{}] {}: {}", stage, title, detail);
            }
        };
        let report = run(&log, &det, &opts).expect("fresh install runs");
        println!("placed: {:?}", report.placed);
        println!("warnings: {:?}", report.warnings);
        assert!(game.join("OptiScaler").is_dir(), "OptiScaler runtime folder");
        assert!(game.join("nvngx_dlssnr.dll").is_file(), "NR runtime downloaded");
        assert!(game.join("nvngx_dlss.dll").is_file(), "DLSS runtime downloaded");
        assert!(game.join("OptiScaler\\streamline").is_dir(), "streamline set downloaded");
        assert!(game.join("OptiScaler.ini").is_file(), "ini written");

        if let Some(prev) = backup {
            let now = std::fs::read_to_string(&store).unwrap_or_default();
            let mut cur: Vec<serde_json::Value> = serde_json::from_str(&now).unwrap_or_default();
            let old: Vec<serde_json::Value> = serde_json::from_str(&prev).unwrap_or_default();
            let seen: HashSet<String> = cur
                .iter()
                .filter_map(|v| v["path"].as_str().map(|s| s.to_lowercase()))
                .collect();
            for o in old {
                if let Some(p) = o["path"].as_str() {
                    if !seen.contains(&p.to_lowercase()) {
                        cur.push(o);
                    }
                }
            }
            let _ = std::fs::write(&store, serde_json::to_string_pretty(&cur).unwrap());
        }
        rollback(&game).expect("rollback");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[ignore]
    fn live_xefg_on_nvidia_installs_libs_and_fakenvapi() {
        if artifacts::list().is_empty() {
            artifacts::auto_import();
        }
        let base = std::env::temp_dir().join("neurodeck-xefg-test");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("Game.exe"), vec![0u8; 1024]).unwrap();
        let mut det = Detection {
            exe: Some(game.join("Game.exe").to_string_lossy().to_string()),
            exe_dir: Some(game.to_string_lossy().to_string()),
            arch: Some("x64".into()),
            ..Default::default()
        };
        det.upscalers = vec!["dlss".into()];

        let opts = InstallOptions {
            nr_provider: "optiscaler".into(),
            install_feeder: false, // game has an upscaler → ReShade stays out
            fg_output: "xefg".into(),
            xess_libs: true,
            hudfix: true,
            ..Default::default()
        };
        let log = |stage: &str, title: &str, detail: &str, done: bool, _e: Option<String>| {
            if done {
                println!("[{}] {}: {}", stage, title, detail);
            }
        };
        let report = run(&log, &det, &opts).expect("xefg install runs");
        println!("placed: {:?}", report.placed);
        println!("warnings: {:?}", report.warnings);
        assert!(game.join("dxgi.dll").is_file(), "no ReShade → OptiScaler takes dxgi.dll");
        assert!(game.join("OptiScaler\\libxess_fg.dll").is_file(), "XeFG runtime from the XeSS SDK");
        assert!(game.join("OptiScaler\\libxell.dll").is_file(), "XeLL runtime");
        assert!(game.join("fakenvapi.dll").is_file(), "fakenvapi for Reflex→XeLL");
        let ini = std::fs::read_to_string(game.join("OptiScaler.ini")).unwrap();
        assert!(ini.contains("FGOutput = xefg"), "ini: {}", ini);
        assert!(!game.join("ReShade.ini").is_file(), "ReShade must not be installed");
        rollback(&game).expect("rollback");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[ignore]
    fn live_install_renodx_provider() {
        if artifacts::list().is_empty() {
            artifacts::auto_import();
        }
        if renodx_artifact_for("renodx-performance").is_none() {
            eprintln!("no renodx artifact - skipping");
            return;
        }
        let base = std::env::temp_dir().join("neurodeck-renodx-install");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("Game.exe"), vec![0u8; 1024]).unwrap();
        let det = Detection {
            exe: Some(game.join("Game.exe").to_string_lossy().to_string()),
            exe_dir: Some(game.to_string_lossy().to_string()),
            arch: Some("x64".into()),
            ..Default::default()
        };
        let opts = InstallOptions {
            nr_provider: "renodx-performance".into(),
            install_feeder: false,
            ..Default::default()
        };
        let noop = |_: &str, _: &str, _: &str, _: bool, _: Option<String>| {};
        let report = run(&noop, &det, &opts).expect("renodx install runs");
        println!("placed: {:?}", report.placed);
        assert!(game.join("nvngx_dlssnr.dll").is_file(), "NR runtime placed");
        assert!(game.join("nvngx_dlss.dll").is_file(), "DLSS runtime placed");
        assert!(!game.join("winmm.dll").is_file(), "no OptiScaler proxy on the RenoDX path");
        rollback(&game).expect("rollback");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[ignore]
    fn live_install_into_fake_game_and_rollback() {
        if artifacts::list().is_empty() {
            artifacts::auto_import();
        }
        if artifacts::best("optiscaler", None).is_none() {
            eprintln!("no local OptiScaler artifact registered - skipping");
            return;
        }
        let base = std::env::temp_dir().join("neurodeck-live-install");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("FakeGame.exe"), vec![0u8; 2048]).unwrap();
        std::fs::write(game.join("OptiScaler.ini"), "[DlssNr]\nEnabled = false\nUserKey = keepme\n").unwrap();
        std::fs::write(game.join("dxgi.dll"), b"reshade-ish placeholder").unwrap();
        std::fs::write(game.join("OptiScaler.asi"), "OptiScaler add-on payload").unwrap();

        let detection = detect::detect(&game, None);
        assert!(detection.exe.is_some(), "stub exe found");
        let opts = InstallOptions {
            preset: "balanced".into(),
            install_sm86: artifacts::best("sm86", None).is_some(),
            ..Default::default()
        };
        let noop = |_: &str, _: &str, _: &str, _: bool, _: Option<String>| {};
        let report = run(&noop, &detection, &opts).expect("install runs");
        println!("placed: {:?}", report.placed);
        println!("warnings: {:?}", report.warnings);

        let proxy = game.join(&report.proxy);
        assert!(proxy.is_file(), "proxy dll placed");
        let ini = std::fs::read_to_string(game.join("OptiScaler.ini")).unwrap();
        assert!(ini.contains("WorkingScale = 0.67"), "preset applied: {}", ini);
        assert!(ini.contains("UserKey = keepme"), "existing keys preserved");
        assert!(ini.contains("Dx12Upscaler = dlss"));
        assert!(game.join("OptiScaler").is_dir(), "OptiScaler runtime folder copied");
        assert!(!game.join("OptiScaler.asi").is_file(), "foreign OptiScaler.asi disabled");

        rollback(&game).expect("rollback");
        let ini_after = std::fs::read_to_string(game.join("OptiScaler.ini")).unwrap();
        assert!(!ini_after.contains("WorkingScale"), "rollback restored ini: {}", ini_after);
        assert!(!game.join(&report.proxy).is_file(), "proxy removed");
        assert_eq!(
            std::fs::read_to_string(game.join("OptiScaler.asi")).unwrap(),
            "OptiScaler add-on payload",
            "rollback restores the hand-installed add-on"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn ini_merge_roundtrip() {
        let mut lines: Vec<String> = vec![
            "[Upscalers]".into(),
            "Dx12Upscaler = auto".into(),
            "[DlssNr]".into(),
            "Enabled = false".into(),
        ];
        set_ini(&mut lines, "Upscalers", "Dx12Upscaler", "dlss");
        set_ini(&mut lines, "DlssNr", "Enabled", "true");
        set_ini(&mut lines, "DlssNr", "WorkingScale", "0.67");
        set_ini(&mut lines, "Log", "LogToFile", "true");
        let text = lines.join("\n");
        assert!(text.contains("Dx12Upscaler = dlss"), "existing key replaced in place: {}", text);
        assert!(!text.contains("Dx12Upscaler = auto"));
        assert!(text.contains("Enabled = true"));
        assert_eq!(text.matches("Enabled").count(), 1, "no duplicate key");
        assert!(text.contains("WorkingScale = 0.67"));
        assert!(text.contains("[Log]"));
        assert!(text.contains("LogToFile = true"));
    }

    #[test]
    fn journal_rollback_restores_replaced_files() {
        let base = std::env::temp_dir().join("neurodeck-rollback-test");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("original.ini"), "vanilla").unwrap();
        let backup = neuro_dir(&game).join("backups").join("test");
        std::fs::create_dir_all(&backup).unwrap();

        let mut w = Writer::new(&game, backup.clone());
        w.place(&game.join("original.ini"), "copy_of_original.dll").unwrap();
        w.write_text("original.ini", "modded").unwrap();
        w.write_text("added.dll", "new file").unwrap();
        std::fs::write(
            backup.join("journal.json"),
            serde_json::to_string(&Journal {
                ts: "test".into(),
                added: w.added.clone(),
                replaced: w.replaced.clone(),
            })
            .unwrap(),
        )
        .unwrap();
        save_manifest(
            &game,
            &Manifest {
                installed_at: "test".into(),
                components: vec![],
                options: InstallOptions::default(),
                journals: vec!["test".into()],
            },
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(game.join("original.ini")).unwrap(), "modded");

        rollback(&game).unwrap();
        assert_eq!(std::fs::read_to_string(game.join("original.ini")).unwrap(), "vanilla");
        assert!(!game.join("added.dll").is_file());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn disabled_foreign_file_comes_back_on_rollback() {
        let base = std::env::temp_dir().join("neurodeck-disable-test");
        let _ = std::fs::remove_dir_all(&base);
        let game = base.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("OptiScaler.asi"), "hand-installed").unwrap();
        let backup = neuro_dir(&game).join("backups").join("test");
        std::fs::create_dir_all(&backup).unwrap();

        let mut w = Writer::new(&game, backup.clone());
        w.disable("OptiScaler.asi").unwrap();
        assert!(!game.join("OptiScaler.asi").is_file());
        std::fs::write(
            backup.join("journal.json"),
            serde_json::to_string(&Journal { ts: "test".into(), added: w.added.clone(), replaced: w.replaced.clone() }).unwrap(),
        )
        .unwrap();
        save_manifest(
            &game,
            &Manifest {
                installed_at: "test".into(),
                components: vec![],
                options: InstallOptions::default(),
                journals: vec!["test".into()],
            },
        )
        .unwrap();

        rollback(&game).unwrap();
        assert_eq!(std::fs::read_to_string(game.join("OptiScaler.asi")).unwrap(), "hand-installed");
        let _ = std::fs::remove_dir_all(&base);
    }
}

