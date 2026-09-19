use crate::ini as inifile;
use crate::install;
use crate::model::Detection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NrSettings {
    pub enabled: bool,
    pub run_before_sr: bool,
    pub working_scale: f64,
    pub passes: u32,
    pub intensity: f64,
    pub style: u32,
    pub preset: u32,
    pub local_structure: f64,
    pub transfer_strength: f64,
    pub colour_strength: f64,
    pub auto_mask: bool,
    pub scan_exposure: bool,
    pub hdr_transfer: bool,
    pub finished_picture: bool,
    pub compare: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FgSettings {
    pub enabled: bool,
    pub input: String,
    pub output: String,
    pub replacement: String,
    pub interpolation_count: u32,
    pub hudfix: bool,
    pub make_depth_copy: bool,
    pub make_mv_copy: bool,
    pub disable_hudless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DlssSettings {
    pub preset_override: bool,
    pub preset: u32,
    pub use_generic_appid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReShadeSettings {
    pub installed: bool,
    pub mv_provider: u32,
    pub mv_validate: bool,
    pub mask_strength: f64,
    pub geom_enable: bool,
    pub debug_view: u32,
    pub techniques: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    pub provider: String,
    pub optiscaler_installed: bool,
    pub reshade_installed: bool,
    pub feeder_installed: bool,
    pub nr: NrSettings,
    pub fg: FgSettings,
    pub reshade: ReShadeSettings,
    pub dlss: DlssSettings,
    pub managed: bool,
}

fn bool_str(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

fn pick_str(v: &str, fallback: &str) -> String {
    if v.trim().is_empty() {
        fallback.to_string()
    } else {
        v.trim().to_string()
    }
}

const TECHNIQUES: &[(&str, &str)] = &[
    ("Lumenite_Kernel", "lumenite_Kernel.fx"),
    ("Lumenite_QuantMotion", "lumenite_QuantMotion.fx"),
    ("DLSS5_Feed", "DLSS5_Feed.fx"),
    ("UIMask_Top", "UIMask.fx"),
    ("UIMask_Bottom", "UIMask.fx"),
];

fn parse_techniques(lines: &[String]) -> BTreeMap<String, bool> {
    let raw = inifile::get(lines, "", "")
        .or_else(|| {
            lines.iter().find_map(|l| {
                let t = l.trim();
                t.strip_prefix("Techniques=").map(|v| v.to_string())
            })
        })
        .unwrap_or_default();
    let active: Vec<String> = raw
        .split(',')
        .filter_map(|e| e.split('@').next().map(|n| n.trim().to_string()))
        .filter(|n| !n.is_empty())
        .collect();
    TECHNIQUES
        .iter()
        .map(|(name, _)| (name.to_string(), active.iter().any(|a| a == name)))
        .collect()
}

fn set_technique(lines: &mut Vec<String>, name: &str, file: &str, enabled: bool) {
    let idx = lines.iter().position(|l| l.trim().starts_with("Techniques="));
    let mut list: Vec<String> = match idx {
        Some(i) => lines[i]
            .trim()
            .trim_start_matches("Techniques=")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => Vec::new(),
    };
    let entry = format!("{}@{}", name, file);
    list.retain(|e| e.split('@').next().map(|n| n != name).unwrap_or(true));
    if enabled {
        list.push(entry);
    }
    let joined = format!("Techniques={}", list.join(","));
    match idx {
        Some(i) => lines[i] = joined,
        None => lines.insert(0, joined),
    }
}

fn read_preset(dir: &Path) -> (Vec<String>, bool) {
    let path = dir.join("ReShadePreset.ini");
    if !path.is_file() {
        return (Vec::new(), false);
    }
    (inifile::read(&path), true)
}

/// Cheap presence check — the full detector walks the whole game folder and
/// must not run when the Settings tab opens.
fn quick_mod_state(dir: &Path) -> (bool, bool, bool) {
    let proxies = [
        "winmm.dll", "dxgi.dll", "version.dll", "dbghelp.dll", "winhttp.dll", "wininet.dll", "d3d12.dll",
    ];
    let opti = dir.join("OptiScaler.ini").is_file()
        || proxies.iter().any(|p| dir.join(p).is_file())
        || dir.join("OptiScaler").is_dir();
    let reshade = dir.join("ReShade.ini").is_file() || dir.join("ReShadePreset.ini").is_file();
    let feeder = dir.join("dlss5-feed.addon64").is_file() || dir.join("dlss5-feed.cfg").is_file();
    (opti, reshade, feeder)
}

pub fn read(dir: &Path, detection: Option<&Detection>) -> GameSettings {
    let mut out = GameSettings::default();
    let state = install::state(dir);
    out.managed = state.managed;
    out.provider = state
        .options
        .as_ref()
        .map(|o| o.provider().to_string())
        .unwrap_or_else(|| "optiscaler".into());
    let (opti_present, reshade_present, feeder_present) = match detection {
        Some(d) => (
            d.mods.optiscaler.is_some(),
            d.mods.reshade.is_some(),
            d.mods.feeder,
        ),
        None => quick_mod_state(dir),
    };
    out.optiscaler_installed = opti_present;
    out.reshade_installed = reshade_present;
    out.feeder_installed = feeder_present;

    let ini = dir.join("OptiScaler.ini");
    if ini.is_file() {
        let lines = inifile::read(&ini);
        out.nr = NrSettings {
            enabled: inifile::get_bool(&lines, "DlssNr", "Enabled").unwrap_or(false),
            run_before_sr: inifile::get_bool(&lines, "DlssNr", "RunBeforeSR").unwrap_or(true),
            working_scale: inifile::get_f64(&lines, "DlssNr", "WorkingScale").unwrap_or(1.0),
            passes: inifile::get_u32(&lines, "DlssNr", "Passes").unwrap_or(1),
            intensity: inifile::get_f64(&lines, "DlssNr", "Intensity").unwrap_or(1.0),
            style: inifile::get_u32(&lines, "DlssNr", "Style").unwrap_or(0),
            preset: inifile::get_u32(&lines, "DlssNr", "Preset").unwrap_or(0),
            local_structure: inifile::get_f64(&lines, "DlssNr", "LocalStructure").unwrap_or(1.0),
            transfer_strength: inifile::get_f64(&lines, "DlssNr", "TransferStrength").unwrap_or(1.0),
            colour_strength: inifile::get_f64(&lines, "DlssNr", "ColourStrength").unwrap_or(1.0),
            auto_mask: inifile::get_bool(&lines, "DlssNr", "AutoMask").unwrap_or(true),
            scan_exposure: inifile::get_bool(&lines, "DlssNr", "ScanExposure").unwrap_or(true),
            hdr_transfer: inifile::get_bool(&lines, "DlssNr", "HdrTransfer").unwrap_or(false),
            finished_picture: inifile::get_bool(&lines, "DlssNr", "FinishedPicture").unwrap_or(false),
            compare: inifile::get_bool(&lines, "DlssNr", "Compare").unwrap_or(false),
        };
        out.fg = FgSettings {
            enabled: inifile::get_bool(&lines, "FrameGen", "Enabled").unwrap_or(false),
            input: pick_str(&inifile::get(&lines, "FrameGen", "FGInput").unwrap_or_default(), "auto"),
            output: pick_str(&inifile::get(&lines, "FrameGen", "FGOutput").unwrap_or_default(), "auto"),
            replacement: pick_str(
                &inifile::get(&lines, "FrameGen", "FGNvngxReplacement").unwrap_or_default(),
                "auto",
            ),
            interpolation_count: inifile::get_u32(&lines, "DLSSG", "InterpolationCount").unwrap_or(1),
            hudfix: inifile::get_bool(&lines, "OptiFG", "HUDFix").unwrap_or(false),
            make_depth_copy: inifile::get_bool(&lines, "OptiFG", "MakeDepthCopy").unwrap_or(true),
            make_mv_copy: inifile::get_bool(&lines, "OptiFG", "MakeMVCopy").unwrap_or(true),
            disable_hudless: inifile::get_bool(&lines, "FrameGen", "DisableHudless").unwrap_or(false),
        };
    }

    if ini.is_file() {
        let lines = inifile::read(&ini);
        out.dlss = DlssSettings {
            preset_override: inifile::get_bool(&lines, "DLSS", "RenderPresetOverride").unwrap_or(false),
            preset: inifile::get_u32(&lines, "DLSS", "RenderPresetForAll").unwrap_or(0),
            use_generic_appid: inifile::get_bool(&lines, "DLSS", "UseGenericAppIdWithDlss").unwrap_or(false),
        };
    }

    let (preset, has_preset) = read_preset(dir);
    out.reshade = ReShadeSettings {
        installed: has_preset || out.reshade_installed,
        mv_provider: inifile::get(&preset, "DLSS5_Feed.fx", "PreprocessorDefinitions")
            .and_then(|d| {
                d.split(',')
                    .find_map(|p| p.trim().strip_prefix("DLSS5_MV_PROVIDER=").map(|v| v.trim().to_string()))
            })
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3),
        mv_validate: inifile::get_u32(&preset, "DLSS5_Feed.fx", "MV_VALIDATE").unwrap_or(1) != 0,
        mask_strength: inifile::get_f64(&preset, "DLSS5_Feed.fx", "MASK_STRENGTH").unwrap_or(0.85),
        geom_enable: inifile::get_u32(&preset, "DLSS5_Feed.fx", "GEOM_ENABLE").unwrap_or(0) != 0,
        debug_view: inifile::get_u32(&preset, "DLSS5_Feed.fx", "DEBUG_VIEW").unwrap_or(0),
        techniques: parse_techniques(&preset),
    };
    out
}

pub fn write(dir: &Path, patch: &GameSettings) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    let ini_path = dir.join("OptiScaler.ini");
    let mut lines = if ini_path.is_file() {
        inifile::read(&ini_path)
    } else {
        Vec::new()
    };
    if !lines.is_empty() {
        inifile::set(&mut lines, "DlssNr", "Enabled", bool_str(patch.nr.enabled));
        inifile::set(&mut lines, "DlssNr", "RunBeforeSR", bool_str(patch.nr.run_before_sr));
        inifile::set(&mut lines, "DlssNr", "WorkingScale", &format!("{:.3}", patch.nr.working_scale));
        inifile::set(&mut lines, "DlssNr", "Passes", &patch.nr.passes.clamp(1, 3).to_string());
        inifile::set(&mut lines, "DlssNr", "Intensity", &format!("{:.2}", patch.nr.intensity));
        inifile::set(&mut lines, "DlssNr", "Style", &patch.nr.style.min(2).to_string());
        inifile::set(&mut lines, "DlssNr", "Preset", &patch.nr.preset.min(3).to_string());
        inifile::set(&mut lines, "DlssNr", "LocalStructure", &format!("{:.2}", patch.nr.local_structure));
        inifile::set(&mut lines, "DlssNr", "TransferStrength", &format!("{:.2}", patch.nr.transfer_strength));
        inifile::set(&mut lines, "DlssNr", "ColourStrength", &format!("{:.2}", patch.nr.colour_strength));
        inifile::set(&mut lines, "DlssNr", "AutoMask", bool_str(patch.nr.auto_mask));
        inifile::set(&mut lines, "DlssNr", "ScanExposure", bool_str(patch.nr.scan_exposure));
        inifile::set(&mut lines, "DlssNr", "HdrTransfer", bool_str(patch.nr.hdr_transfer));
        inifile::set(&mut lines, "DlssNr", "FinishedPicture", bool_str(patch.nr.finished_picture));
        inifile::set(&mut lines, "DlssNr", "Compare", bool_str(patch.nr.compare));
        inifile::set(&mut lines, "FrameGen", "Enabled", bool_str(patch.fg.enabled));
        inifile::set(&mut lines, "FrameGen", "FGInput", &patch.fg.input);
        inifile::set(&mut lines, "FrameGen", "FGOutput", &patch.fg.output);
        let replacement = match patch.fg.replacement.as_str() {
            "none" => "None",
            "nukems" => "Nukems",
            "arturs" => "Arturs",
            "ffx" => "FFX",
            "combo" => "Combo",
            other => other,
        };
        inifile::set(&mut lines, "FrameGen", "FGNvngxReplacement", replacement);
        inifile::set(&mut lines, "FrameGen", "DisableHudless", bool_str(patch.fg.disable_hudless));
        inifile::set(&mut lines, "DLSSG", "InterpolationCount", &patch.fg.interpolation_count.clamp(1, 5).to_string());
        inifile::set(&mut lines, "OptiFG", "HUDFix", bool_str(patch.fg.hudfix));
        inifile::set(&mut lines, "OptiFG", "MakeDepthCopy", bool_str(patch.fg.make_depth_copy));
        inifile::set(&mut lines, "OptiFG", "MakeMVCopy", bool_str(patch.fg.make_mv_copy));
        inifile::set(&mut lines, "DLSS", "RenderPresetOverride", bool_str(patch.dlss.preset_override));
        inifile::set(&mut lines, "DLSS", "RenderPresetForAll", &patch.dlss.preset.min(15).to_string());
        inifile::set(&mut lines, "DLSS", "UseGenericAppIdWithDlss", bool_str(patch.dlss.use_generic_appid));
        install::write_journaled(dir, "OptiScaler.ini", &(lines.join("\r\n") + "\r\n"))?;
        written.push("OptiScaler.ini".into());
    }

    let preset_path = dir.join("ReShadePreset.ini");
    if preset_path.is_file() {
        let mut preset = inifile::read(&preset_path);
        let providers = format!("DLSS5_MV_PROVIDER={}", patch.reshade.mv_provider.clamp(3, 4));
        inifile::set(&mut preset, "DLSS5_Feed.fx", "PreprocessorDefinitions", &providers);
        inifile::set(&mut preset, "DLSS5_Feed.fx", "MV_VALIDATE", if patch.reshade.mv_validate { "1" } else { "0" });
        inifile::set(
            &mut preset,
            "DLSS5_Feed.fx",
            "MASK_STRENGTH",
            &format!("{:.6}", patch.reshade.mask_strength),
        );
        inifile::set(
            &mut preset,
            "DLSS5_Feed.fx",
            "GEOM_ENABLE",
            if patch.reshade.geom_enable { "1" } else { "0" },
        );
        inifile::set(&mut preset, "DLSS5_Feed.fx", "DEBUG_VIEW", &patch.reshade.debug_view.to_string());
        for (name, file) in TECHNIQUES {
            if let Some(on) = patch.reshade.techniques.get(*name) {
                set_technique(&mut preset, name, file, *on);
            }
        }
        install::write_journaled(dir, "ReShadePreset.ini", &(preset.join("\r\n") + "\r\n"))?;
        written.push("ReShadePreset.ini".into());
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_settings() {
        let base = std::env::temp_dir().join("neurodeck-settings-test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            base.join("OptiScaler.ini"),
            "[DlssNr]\nEnabled = false\nWorkingScale = 0.67\n[FrameGen]\nFGOutput = dlssg\n",
        )
        .unwrap();
        std::fs::write(
            base.join("ReShadePreset.ini"),
            "Techniques=Lumenite_Kernel@lumenite_Kernel.fx,DLSS5_Feed@DLSS5_Feed.fx\n\n[DLSS5_Feed.fx]\nPreprocessorDefinitions=DLSS5_MV_PROVIDER=3\nMASK_STRENGTH=0.85\n",
        )
        .unwrap();

        let mut s = read(&base, None);
        assert_eq!(s.nr.working_scale, 0.67);
        assert_eq!(s.fg.output, "dlssg");
        assert_eq!(s.reshade.mv_provider, 3);
        assert_eq!(s.reshade.techniques.get("Lumenite_Kernel"), Some(&true));
        assert_eq!(s.reshade.techniques.get("Lumenite_QuantMotion"), Some(&false));

        s.nr.enabled = true;
        s.nr.passes = 2;
        s.fg.output = "xefg".into();
        s.reshade.mv_provider = 4;
        s.dlss.preset_override = true;
        s.dlss.preset = 7;
        s.reshade.techniques.insert("Lumenite_QuantMotion".into(), true);
        s.reshade.techniques.insert("Lumenite_Kernel".into(), false);
        write(&base, &s).unwrap();

        let after = read(&base, None);
        assert!(after.nr.enabled);
        assert_eq!(after.nr.passes, 2);
        assert_eq!(after.fg.output, "xefg");
        assert_eq!(after.reshade.mv_provider, 4);
        assert!(after.dlss.preset_override);
        assert_eq!(after.dlss.preset, 7);
        assert_eq!(after.reshade.techniques.get("Lumenite_QuantMotion"), Some(&true));
        assert_eq!(after.reshade.techniques.get("Lumenite_Kernel"), Some(&false));
        let preset = std::fs::read_to_string(base.join("ReShadePreset.ini")).unwrap();
        assert!(preset.contains("DLSS5_MV_PROVIDER=4"));
        assert!(preset.contains("Lumenite_QuantMotion@lumenite_QuantMotion.fx"));
        assert!(!preset.contains("Lumenite_Kernel@lumenite_Kernel.fx"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
