use crate::install;
use crate::model::Detection;
use crate::pe;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub label: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReport {
    pub checks: Vec<Check>,
    pub hints: Vec<String>,
    pub managed: bool,
}

fn file_check(dir: &Path, rel: &str, label: &str, checks: &mut Vec<Check>) {
    let p = dir.join(rel);
    checks.push(Check {
        label: label.into(),
        ok: p.is_file(),
        detail: rel.into(),
    });
}

fn tail(path: &Path, bytes: usize) -> String {
    let Ok(meta) = std::fs::metadata(path) else { return String::new() };
    let len = meta.len() as usize;
    let start = len.saturating_sub(bytes);
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else { return String::new() };
    if f.seek(SeekFrom::Start(start as u64)).is_err() {
        return String::new();
    }
    let mut raw = Vec::new();
    let _ = f.take(bytes as u64).read_to_end(&mut raw);
    String::from_utf8_lossy(&raw).to_string()
}

pub fn verify(dir: &Path, detection: Option<&Detection>) -> VerifyReport {
    let mut report = VerifyReport::default();
    let state = install::state(dir);
    report.managed = state.managed;
    let opts = state.options.clone().unwrap_or_default();
    let mut checks = Vec::new();

    // proxy: whichever candidate carries the OptiScaler marker
    let proxies = ["dxgi.dll", "winmm.dll", "version.dll", "dbghelp.dll", "winhttp.dll", "wininet.dll", "d3d12.dll"];
    let mut proxy_found: Option<String> = None;
    for p in proxies {
        let path = dir.join(p);
        if path.is_file() && !pe::scan_file_markers(&path, &["OptiScaler"], 40 * 1_048_576).is_empty() {
            proxy_found = Some(p.to_string());
            break;
        }
    }
    checks.push(Check {
        label: "OptiScaler proxy DLL".into(),
        ok: proxy_found.is_some(),
        detail: proxy_found.clone().unwrap_or_else(|| "none of dxgi/winmm/version/dbghelp found".into()),
    });

    if opts.uses_optiscaler() {
        file_check(dir, "OptiScaler.ini", "OptiScaler.ini", &mut checks);
        checks.push(Check {
            label: "OptiScaler runtime folder".into(),
            ok: dir.join("OptiScaler").is_dir(),
            detail: "OptiScaler\\".into(),
        });
        if opts.fg_output == "xefg" {
            file_check(dir, "OptiScaler\\libxess_fg.dll", "XeFG runtime (libxess_fg)", &mut checks);
            file_check(dir, "OptiScaler\\libxell.dll", "XeLL runtime (libxell)", &mut checks);
            file_check(dir, "fakenvapi.dll", "fakenvapi (Reflex → XeLL)", &mut checks);
        }
        if opts.streamline {
            checks.push(Check {
                label: "Streamline set".into(),
                ok: dir.join("OptiScaler").join("streamline").is_dir(),
                detail: "OptiScaler\\streamline\\".into(),
            });
        }
        if opts.optipatcher {
            file_check(dir, "OptiScaler\\plugins\\OptiPatcher.asi", "OptiPatcher", &mut checks);
        }
    }
    file_check(dir, "nvngx_dlssnr.dll", "NR runtime", &mut checks);
    file_check(dir, "nvngx_dlss.dll", "DLSS runtime", &mut checks);
    if opts.runtime_dlssd {
        file_check(dir, "nvngx_dlssd.dll", "DLSSD runtime", &mut checks);
    }
    if opts.install_sm86 && opts.uses_optiscaler() {
        file_check(dir, "dlssg_sm86.ini", "DLSSG sm86 config", &mut checks);
        // The sm86 mod ships as version.dll; OptiScaler also names dlssg_sm86 in its
        // strings, so a marker scan of arbitrary proxies reports false positives.
        let v = dir.join("version.dll");
        let markers = v.is_file().then(|| pe::scan_file_markers(&v, &["dlssg_sm86", "OptiScaler"], 32 * 1_048_576));
        let sm86_ok = markers.as_ref().is_some_and(|m| m.iter().any(|s| s == "dlssg_sm86") && !m.iter().any(|s| s == "OptiScaler"));
        checks.push(Check {
            label: "DLSSG sm86 proxy".into(),
            ok: sm86_ok,
            detail: if sm86_ok { "version.dll".into() } else { "version.dll missing or not the sm86 build".into() },
        });
    }
    if opts.install_feeder || !opts.uses_optiscaler() {
        let addon = dir.join("dlss5-feed.addon64").is_file() || dir.join("dlss5-feed.addon32").is_file();
        checks.push(Check {
            label: "DLSS5 feeder add-on".into(),
            ok: addon,
            detail: "dlss5-feed.addon64".into(),
        });
        checks.push(Check {
            label: "ReShade host".into(),
            ok: dir.join("ReShade.ini").is_file(),
            detail: "ReShade.ini".into(),
        });
    }
    checks.push(Check {
        label: "Rollback snapshot".into(),
        ok: !state.backups.is_empty(),
        detail: format!("{} snapshot(s)", state.backups.len()),
    });

    // runtime health from the logs
    let opti_log = dir.join("OptiScaler.log");
    if opti_log.is_file() {
        let t = tail(&opti_log, 24_000).to_lowercase();
        if t.contains("waiting for the upscaler") {
            report
                .hints
                .push("OptiScaler is waiting for an upscaler call: enable the feeder for this game, or use the RenoDX provider.".into());
        }
        if t.contains("model initialization failed") || t.contains("model wouldn't initialize") {
            report
                .hints
                .push("The NR model failed to initialise — check the NR runtime hash and your driver version.".into());
        }
        if !t.contains("dlssnr") && !t.contains("neural") {
            report
                .hints
                .push("The log does not mention NR yet: it only appears after you enable Neural Rendering in the overlay.".into());
        }
    } else {
        report.hints.push("No OptiScaler.log yet — launch the game once with logging enabled.".into());
    }
    let feed_log = dir.join("dlss5-feed.log");
    if feed_log.is_file() {
        let text = tail(&feed_log, 24_000);
        let first = text.lines().next().unwrap_or("").to_string();
        if !first.is_empty() {
            report.hints.push(format!("Feeder log: {}", first.trim()));
        }
        if text.contains("frame 0 delivered") || !text.contains("delivered") {
            report
                .hints
                .push("The feeder has not delivered frames yet — it only runs once the game presents with the add-on loaded.".into());
        }
    }
    if let Some(d) = detection {
        if d.engine.as_deref() == Some("Unity") {
            report
                .hints
                .push("Unity title: launch with -force-vulkan if the feeder crashes on Direct3D 11.".into());
        }
    }

    report.checks = checks;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn live_verify_reports_for_real_installs() {
        for dir in [r"D:\Games\GTAVEnhanced", r"D:\Games\Easy Red 2"] {
            let path = std::path::Path::new(dir);
            if !path.is_dir() {
                continue;
            }
            let detection = crate::detect::detect(path, None);
            let m = &detection.mods;
            println!(
                "\n=== {dir} mods: opti={:?} asi={} reshade={:?} feeder={} nr={} sm86={} dxvk={} dgvoodoo={} enabler={} fakenvapi={} ===",
                m.optiscaler, m.optiscaler_asi, m.reshade, m.feeder, m.nr_runtime, m.dlssg_sm86,
                m.dxvk, m.dgvoodoo, m.dlss_enabler, m.fakenvapi
            );
            let report = verify(path, Some(&detection));
            println!("=== {dir} (managed={}) ===", report.managed);
            for c in &report.checks {
                println!(" {} {:<28} {}", if c.ok { "OK  " } else { "MISS" }, c.label, c.detail);
            }
            for h in &report.hints {
                println!(" !  {h}");
            }
        }
    }
}
