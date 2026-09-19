use crate::artifacts::{self, Artifact};
use crate::registry::{self, ReleaseFile, ReleaseInfo};
use std::path::PathBuf;

fn choose_file<'a>(rel: &'a ReleaseInfo, needle: &str, exclude: &str) -> Option<&'a ReleaseFile> {
    rel.files
        .iter()
        .filter(|f| f.name.contains(needle) && !(exclude.is_empty() == false && f.name.contains(exclude)))
        .max_by_key(|f| f.size)
}

pub fn component_kind(id: &str) -> &str {
    match id {
        "renodx-rhi" => "renodx",
        // the enabler ships an Inno Setup installer, not a bare DLL
        "dlss-enabler" => "dlss-enabler-installer",
        "fakenvapi" => "fakenvapi",
        other => other,
    }
}

fn asset_needle(id: &str, variant: Option<&str>) -> (&'static str, &'static str) {
    match id {
        "optiscaler-nightly" => (".7z", ""),
        "optiscaler-mfg" => (".7z", ""),
        "fakenvapi" => (".7z", ""),
        "xess-sdk" => ("XeSS_SDK_", ""),
        "dlss-enabler" => (".exe", ""),
        "renodx-rhi" => ("renodx-dlss5", ""),
        "optipatcher" => ("OptiPatcher", ""),
        "reshade" => ("ReShade_Setup", ""),
        "lumenite" => ("Lumenite", ""),
        _ if id.starts_with("optiscaler") => {
            if variant == Some("rtx40-mfg") {
                ("rtx40-mfg", "")
            } else {
                (".zip", "rtx40-mfg")
            }
        }
        _ => (".zip", ""),
    }
}

pub fn component(id: &str, variant: Option<&str>, channel: &str, emit: artifacts::Emit) -> Result<Artifact, String> {
    let components = registry::list(false, channel == "beta");
    let comp = components
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("unknown component {}", id))?;

    if let Some(url) = &comp.pinned_url {
        let raw_name = url.rsplit('/').next().unwrap_or("download.zip").to_string();
        let name = if raw_name.contains("mainline") {
            "LumeniteFX-mainline.zip".to_string()
        } else {
            raw_name
        };
        let rel = ReleaseInfo {
            version: if id == "installer-feeder" { "upstream".into() } else { "mainline".into() },
            tag: "pinned".into(),
            title: name.clone(),
            published: String::new(),
            page: url.clone(),
            beta: false,
            files: vec![ReleaseFile {
                name: name.clone(),
                url: url.clone(),
                size: 0,
            }],
        };
        let dest = artifacts::fetch_release(&rel, component_kind(id), &name, emit)?;
        return artifacts::register(component_kind(id), &rel.version, None, &dest, "download")
            .ok_or_else(|| "could not register artifact".to_string());
    }

    let rel = if channel == "beta" {
        comp.beta_latest.clone().or_else(|| comp.latest.clone())
    } else {
        comp.latest.clone()
    }
    .ok_or_else(|| format!("{} has no downloadable release", comp.label))?;

    if id == "sm86" {
        let mut dll: Option<PathBuf> = None;
        for f in &rel.files {
            let rel_one = ReleaseInfo {
                version: rel.version.clone(),
                tag: rel.tag.clone(),
                title: rel.title.clone(),
                published: rel.published.clone(),
                page: rel.page.clone(),
                beta: rel.beta,
                files: vec![f.clone()],
            };
            let path = artifacts::fetch_release(&rel_one, "sm86", &f.name, emit)?;
            if f.name.eq_ignore_ascii_case("version.dll") {
                dll = Some(path);
            }
        }
        let dll = dll.ok_or("sm86 download incomplete")?;
        return artifacts::register("sm86", &rel.version, None, &dll, "download")
            .ok_or_else(|| "could not register sm86 artifact".to_string());
    }

    let (needle, exclude) = asset_needle(id, variant);
    let file = choose_file(&rel, needle, exclude).ok_or_else(|| format!("no matching asset in {}", rel.tag))?;

    let variant_opt = if id == "optiscaler-nightly" {
        Some("nightly")
    } else if id.starts_with("optiscaler") {
        Some(if file.name.contains("rtx40-mfg") { "rtx40-mfg" } else { "standard" })
    } else {
        None
    };
    let kind = component_kind(id);
    let dest = artifacts::fetch_release(&rel, kind, &file.name, emit)?;
    artifacts::register(kind, &rel.version, variant_opt, &dest, "download")
        .ok_or_else(|| "could not register artifact".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn live_download_small_components() {
        let emit = |p: crate::artifacts::DownloadProgress| {
            if p.done {
                println!("  done {} {}", p.file, p.error.clone().unwrap_or_default());
            }
        };
        for id in ["optipatcher", "feeder"] {
            match component(id, None, "stable", &emit) {
                Ok(a) => println!("{} -> {} {} ({} bytes, {})", id, a.version, a.kind, a.size, a.origin),
                Err(e) => println!("{} -> ERROR {}", id, e),
            }
        }
    }
}
