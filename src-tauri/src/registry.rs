use crate::http;
use crate::paths;
use crate::version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseFile {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub version: String,
    pub tag: String,
    pub title: String,
    pub published: String,
    pub page: String,
    pub beta: bool,
    pub files: Vec<ReleaseFile>,
}

impl ReleaseInfo {
    fn file(&self, needle: &str) -> Option<&ReleaseFile> {
        self.files.iter().find(|f| f.name.contains(needle))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: String,
    pub label: String,
    pub family: String,
    pub source: String,
    pub note: String,
    pub latest: Option<ReleaseInfo>,
    pub beta_latest: Option<ReleaseInfo>,
    pub pinned_url: Option<String>,
    pub error: Option<String>,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheFile {
    repos: BTreeMap<String, RepoCache>,
    #[serde(default)]
    last_check: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RepoCache {
    fetched: i64,
    releases: Vec<RawRelease>,
}

const CACHE_TTL_SECS: i64 = 6 * 3600;
const AUTO_REFRESH_SECS: i64 = 30 * 60;

pub fn last_check() -> i64 {
    load_cache().last_check
}

/// Which fork ships this exact asset name (cache only, no network). The release
/// asset names are the file names users download, so this identifies imported files.
pub fn cached_asset_fork(asset_name: &str) -> Option<String> {
    let cache = load_cache();
    for (repo, rc) in &cache.repos {
        let kind = match repo.as_str() {
            "wilsjo2/OptiScaler-DLSSNR-PreSR-Multipass" => "optiscaler-wilsjo2",
            "janblade/OptiScaler-DLSSNR-PreSR-Multipass" => "optiscaler-janblade",
            "optiscaler/OptiScaler-nightly" => "optiscaler-nightly",
            _ => continue,
        };
        for rel in &rc.releases {
            if rel.files.iter().any(|f| f.name.eq_ignore_ascii_case(asset_name)) {
                return Some(kind.to_string());
            }
        }
    }
    None
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn cache_path() -> std::path::PathBuf {
    paths::cache_dir().join("sources.json")
}

fn load_cache() -> CacheFile {
    std::fs::read_to_string(cache_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(CacheFile { repos: BTreeMap::new(), last_check: 0 })
}

fn save_cache(c: &CacheFile) {
    if let Ok(t) = serde_json::to_string(c) {
        let _ = std::fs::write(cache_path(), t);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawRelease {
    tag: String,
    title: String,
    beta: bool,
    published: String,
    page: String,
    files: Vec<ReleaseFile>,
}

fn gs(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn fetch_repo(repo: &str, pages: u32, refresh: bool, cache: &mut CacheFile) -> (Vec<RawRelease>, Option<String>) {
    let cached = cache.repos.get(repo).cloned();
    if let Some(hit) = &cached {
        if !refresh && now_secs() - hit.fetched < CACHE_TTL_SECS {
            return (hit.releases.clone(), None);
        }
    }
    let mut out: Vec<RawRelease> = Vec::new();
    let mut error: Option<String> = None;
    for page in 1..=pages {
        let url = format!(
            "https://api.github.com/repos/{}/releases?per_page=30&page={}",
            repo, page
        );
        let Some(body) = http::curl_get(&url) else {
            error = Some("network unavailable".into());
            break;
        };
        if body.contains("API rate limit exceeded") {
            error = Some("GitHub API rate limit reached (no token)".into());
            break;
        }
        let Ok(slice) = serde_json::from_str::<Vec<serde_json::Value>>(&body) else {
            error = Some("unexpected GitHub response".into());
            break;
        };
        if slice.is_empty() {
            break;
        }
        for r in slice {
            if r.get("draft").and_then(|d| d.as_bool()).unwrap_or(false) {
                continue;
            }
            let files = r
                .get("assets")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|a| ReleaseFile {
                            name: gs(a, "name"),
                            url: gs(a, "browser_download_url"),
                            size: a.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                        })
                        .collect()
                })
                .unwrap_or_default();
            out.push(RawRelease {
                tag: gs(&r, "tag_name"),
                title: gs(&r, "name"),
                beta: r.get("prerelease").and_then(|b| b.as_bool()).unwrap_or(false),
                published: gs(&r, "published_at"),
                page: gs(&r, "html_url"),
                files,
            });
        }
    }
    if !out.is_empty() {
        cache.repos.insert(
            repo.to_string(),
            RepoCache {
                fetched: now_secs(),
                releases: out.clone(),
            },
        );
        return (out, None);
    }
    if let Some(hit) = cached {
        let note = error.clone().unwrap_or_else(|| "using cached data".into());
        return (
            hit.releases,
            Some(format!("{} (showing cached release list)", note)),
        );
    }
    (out, error.or_else(|| Some("no releases found".into())))
}

fn tag_any(_: &str) -> bool {
    true
}
fn tag_dlssnr_nvidia(t: &str) -> bool {
    t.starts_with("dlssnr-") && !t.contains("SF")
}
fn tag_dlssnr_compat(t: &str) -> bool {
    t.starts_with("dlssnr-") && t.contains("SF")
}
fn tag_dlss(t: &str) -> bool {
    t.starts_with("dlss-")
}
fn tag_renodx(t: &str) -> bool {
    t.starts_with("renodx-dlss5") || t.starts_with("renodx-dlss-") || t.starts_with("renodx-dlss_")
}

struct Def {
    id: &'static str,
    label: &'static str,
    family: &'static str,
    repo: &'static str,
    pages: u32,
    asset_contains: &'static str,
    tag_exclude: &'static str,
    tag_rule: fn(&str) -> bool,
    note: &'static str,
    channel: &'static str,
}

fn defs() -> Vec<Def> {
    vec![
        Def {
            id: "optiscaler-wilsjo2",
            label: "OptiScaler NR (wilsjo2)",
            family: "optiscaler",
            repo: "wilsjo2/OptiScaler-DLSSNR-PreSR-Multipass",
            pages: 1,
            asset_contains: "OptiScaler-NR-",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "PreSR multipass NR fork. Standard + rtx40-mfg variants.",
            channel: "stable",
        },
        Def {
            id: "optiscaler-janblade",
            label: "OptiScaler NR (janblade)",
            family: "optiscaler",
            repo: "janblade/OptiScaler-DLSSNR-PreSR-Multipass",
            pages: 1,
            asset_contains: "OptiScaler-DLSSNR-",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "Tier-preset / menu build of the same NR fork.",
            channel: "stable",
        },
        Def {
            id: "optiscaler-mfg",
            label: "OptiScaler MFG Unlock (evairx)",
            family: "optiscaler",
            repo: "evairx/OptiScaler-MFG",
            pages: 1,
            asset_contains: ".7z",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "Unofficial fork: unlocks NVIDIA MFG on RTX 20/30/40 (up to 4X/6X) and keeps Intel XeFG as an explicit output.",
            channel: "stable",
        },
        Def {
            id: "optiscaler-nightly",
            label: "OptiScaler Nightly (upstream)",
            family: "optiscaler",
            repo: "optiscaler/OptiScaler-nightly",
            pages: 1,
            asset_contains: ".7z",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "Upstream nightly: newest FG/HUDfix/XeFG work, no NR pass. Packed as .7z.",
            channel: "nightly",
        },
        Def {
            id: "feeder",
            label: "DLSS5-Feeder",
            family: "feeder",
            repo: "jlrouzies-fr/DLSS5-Feeder",
            pages: 1,
            asset_contains: "DLSS5-Feeder-",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "Motion-vector/depth feed for NR. Used when the game has no upscaler.",
            channel: "stable",
        },
        Def {
            id: "renodx-rhi",
            label: "RenoDX DLSS 5 add-on",
            family: "feeder",
            repo: "RankFTW/rhi-repo",
            pages: 3,
            asset_contains: "renodx-dlss5",
            tag_exclude: "",
            tag_rule: tag_renodx,
            note: "Krish's RenoDX DLSS 5 consumer: NR through the feeder without OptiScaler.",
            channel: "stable",
        },
        Def {
            id: "optipatcher",
            label: "OptiPatcher",
            family: "tool",
            repo: "optiscaler/OptiPatcher",
            pages: 1,
            asset_contains: "OptiPatcher",
            tag_exclude: "rolling",
            tag_rule: tag_any,
            note: "Unlocks DLSS/DLSSG inputs without spoofing in supported games.",
            channel: "stable",
        },
        Def {
            id: "sm86",
            label: "DLSSG sm86 (RTX 20/30 FG)",
            family: "tool",
            repo: "sdli1995/dlssg_for_sm86",
            pages: 1,
            asset_contains: "\u{0}",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "Frame generation on Ampere/Turing. Needs a game with DLSSG.",
            channel: "stable",
        },
        Def {
            id: "streamline",
            label: "NVIDIA Streamline SDK",
            family: "tool",
            repo: "NVIDIA-RTX/Streamline",
            pages: 1,
            asset_contains: "streamline-sdk-",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "Official sl.*.dll set for OptiScaler's DLSSG output. Extracted from the SDK zip.",
            channel: "stable",
        },
        Def {
            id: "dlss-enabler",
            label: "DLSS Enabler (Artur)",
            family: "tool",
            repo: "artur-graniszewski/DLSS-Enabler",
            pages: 1,
            asset_contains: ".exe",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "dlss-enabler-headless.dll for OptiScaler's FSR FG / MFG replacement. Releases ship an installer, so use a local copy.",
            channel: "stable",
        },
        Def {
            id: "xess-sdk",
            label: "Intel XeSS SDK (XeFG DP4a + XeLL)",
            family: "tool",
            repo: "intel/xess",
            pages: 1,
            asset_contains: "XeSS_SDK_",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "libxess_fg.dll + libxell.dll from the SDK. XeSS 3.x enables XeFG on non-Intel GPUs (RTX included).",
            channel: "stable",
        },
        Def {
            id: "fakenvapi",
            label: "Fakenvapi (XeLL / Anti-Lag 2)",
            family: "tool",
            repo: "optiscaler/fakenvapi",
            pages: 1,
            asset_contains: ".7z",
            tag_exclude: "",
            tag_rule: tag_any,
            note: "Hooks Reflex and injects XeLL so XeFG gets low-latency pacing on NVIDIA GPUs.",
            channel: "stable",
        },
        Def {
            id: "runtime-nr-nvidia",
            label: "NR runtime (RTX 50)",
            family: "runtime",
            repo: "RankFTW/rhi-repo",
            pages: 3,
            asset_contains: "nvngx_dlssnr",
            tag_exclude: "",
            tag_rule: tag_dlssnr_nvidia,
            note: "NVIDIA-signed nvngx_dlssnr.dll. RTX 50.",
            channel: "stable",
        },
        Def {
            id: "runtime-nr-compat",
            label: "NR compat runtime (RTX 20/30/40)",
            family: "runtime",
            repo: "RankFTW/rhi-repo",
            pages: 3,
            asset_contains: "nvngx_dlssnr",
            tag_exclude: "",
            tag_rule: tag_dlssnr_compat,
            note: "ShortFuse cross-generation nvngx_dlssnr.dll.",
            channel: "stable",
        },
        Def {
            id: "runtime-dlss",
            label: "DLSS runtime (nvngx_dlss)",
            family: "runtime",
            repo: "RankFTW/rhi-repo",
            pages: 3,
            asset_contains: "nvngx_dlss",
            tag_exclude: "",
            tag_rule: tag_dlss,
            note: "nvngx_dlss.dll used as the upscaler input and for NR enlargement.",
            channel: "stable",
        },
    ]
}

/// Can this component be fetched without the user supplying a file? Reads the
/// cache only (no network) and stays optimistic before the first check.
pub fn cached_downloadable(id: &str) -> bool {
    if matches!(id, "lumenite" | "installer-feeder" | "reshade") {
        return true;
    }
    let cache = load_cache();
    for def in defs() {
        if def.id == id {
            return cache
                .repos
                .get(def.repo)
                .map(|r| !r.releases.is_empty())
                .unwrap_or(true);
        }
    }
    false
}

fn release_from_raw(raw: &RawRelease, def: &Def) -> Option<ReleaseInfo> {
    let files: Vec<ReleaseFile> = if def.asset_contains == "\u{0}" {
        raw.files.clone()
    } else {
        raw.files
            .iter()
            .filter(|f| f.name.contains(def.asset_contains))
            .cloned()
            .collect()
    };
    if files.is_empty() {
        return None;
    }
    Some(ReleaseInfo {
        version: raw.tag.trim_start_matches('v').to_string(),
        tag: raw.tag.clone(),
        title: raw.title.clone(),
        published: raw.published.clone(),
        page: raw.page.clone(),
        beta: raw.beta || version::is_prerelease_text(&raw.tag),
        files,
    })
}

fn pick(releases: &[RawRelease], def: &Def, allow_beta: bool) -> Option<ReleaseInfo> {
    let matching: Vec<ReleaseInfo> = releases
        .iter()
        .filter(|r| def.tag_exclude.is_empty() || !r.tag.contains(def.tag_exclude))
        .filter(|r| (def.tag_rule)(&r.tag))
        .filter_map(|r| release_from_raw(r, def))
        .collect();
    matching
        .iter()
        .filter(|r| allow_beta || !r.beta)
        .max_by(|a, b| version::extract(&a.version).cmp(&version::extract(&b.version)))
        .cloned()
}

fn sm86_releases(refresh: bool, cache: &mut CacheFile) -> (Vec<RawRelease>, Option<String>) {
    let repo = "sdli1995/dlssg_for_sm86";
    let cached = cache.repos.get(repo).cloned();
    if let Some(hit) = &cached {
        if !refresh && now_secs() - hit.fetched < CACHE_TTL_SECS {
            return (hit.releases.clone(), None);
        }
    }
    let url = format!("https://api.github.com/repos/{}/tags?per_page=30", repo);
    let mut out = Vec::new();
    let mut error = None;
    match http::curl_get(&url) {
        None => error = Some("network unavailable".into()),
        Some(body) if body.contains("API rate limit exceeded") => {
            error = Some("GitHub API rate limit reached (no token)".into())
        }
        Some(body) => {
            if let Ok(tags) = serde_json::from_str::<Vec<serde_json::Value>>(&body) {
                for t in tags {
                    let tag = gs(&t, "name");
                    if tag.is_empty() {
                        continue;
                    }
                    let base = format!("https://raw.githubusercontent.com/{}/{}/", repo, tag);
                    out.push(RawRelease {
                        tag: tag.clone(),
                        title: format!("DLSSG sm86 {}", tag),
                        beta: false,
                        published: String::new(),
                        page: format!("https://github.com/{}/tree/{}", repo, tag),
                        files: vec![
                            ReleaseFile {
                                name: "version.dll".into(),
                                url: format!("{}version.dll", base),
                                size: 28_600_000,
                            },
                            ReleaseFile {
                                name: "dlssg_sm86.ini".into(),
                                url: format!("{}dlssg_sm86.ini", base),
                                size: 3_600,
                            },
                        ],
                    });
                }
            } else {
                error = Some("unexpected GitHub response".into());
            }
        }
    }
    if !out.is_empty() {
        cache.repos.insert(
            repo.to_string(),
            RepoCache {
                fetched: now_secs(),
                releases: out.clone(),
            },
        );
        return (out, None);
    }
    if let Some(hit) = cached {
        return (hit.releases, Some("using cached data".into()));
    }
    (out, error.or_else(|| Some("no tags found".into())))
}

fn read_github_token() -> Option<String> {
    let text = std::fs::read_to_string(crate::paths::settings_path()).ok()?;
    let j: serde_json::Value = serde_json::from_str(&text).ok()?;
    j.get("githubToken").and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn reshade_release(refresh: bool, cache: &mut CacheFile) -> Option<ReleaseInfo> {
    let repo = "reshade.me";
    let fresh = cache
        .repos
        .get(repo)
        .map(|c| now_secs() - c.fetched < CACHE_TTL_SECS && !refresh)
        .unwrap_or(false);
    let mut version_text = if fresh {
        cache.repos.get(repo)?.releases.first()?.tag.clone()
    } else {
        let page = http::curl_get("https://reshade.me/")?;
        let mut found = String::new();
        let bytes = page.as_bytes();
        let needle = b"ReShade_Setup_";
        for i in 0..bytes.len().saturating_sub(needle.len()) {
            if &bytes[i..i + needle.len()] == needle {
                let rest = &page[i + needle.len()..];
                let ver: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
                let ver = ver.trim_end_matches('.').to_string();
                if ver.matches('.').count() >= 2 && !ver.is_empty() {
                    found = ver;
                    break;
                }
            }
        }
        if found.is_empty() {
            found = "6.8.0".to_string();
        }
        cache.repos.insert(
            repo.to_string(),
            RepoCache {
                fetched: now_secs(),
                releases: vec![RawRelease {
                    tag: found.clone(),
                    title: format!("ReShade {}", found),
                    beta: false,
                    published: String::new(),
                    page: "https://reshade.me/".into(),
                    files: vec![],
                }],
            },
        );
        found
    };
    if version_text.is_empty() {
        version_text = "6.8.0".into();
    }
    Some(ReleaseInfo {
        version: version_text.clone(),
        tag: version_text.clone(),
        title: format!("ReShade {}", version_text),
        published: String::new(),
        page: "https://reshade.me/".into(),
        beta: false,
        files: vec![ReleaseFile {
            name: format!("ReShade_Setup_{}_Addon.exe", version_text),
            url: format!(
                "https://reshade.me/downloads/ReShade_Setup_{}_Addon.exe",
                version_text
            ),
            size: 4_318_424,
        }],
    })
}

pub fn list(refresh: bool, allow_beta: bool) -> Vec<Component> {
    let mut cache = load_cache();
    let mut out = Vec::new();
    crate::http::set_github_token(read_github_token());

    // A normal call still re-checks GitHub when the last check is older than
    // AUTO_REFRESH_SECS, so starting the app picks up new releases on its own.
    let refresh = refresh || now_secs() - cache.last_check > AUTO_REFRESH_SECS;

    for def in defs() {
        let (releases, err) = if def.id == "sm86" {
            sm86_releases(refresh, &mut cache)
        } else {
            fetch_repo(def.repo, def.pages, refresh, &mut cache)
        };
        let latest = pick(&releases, &def, allow_beta);
        let beta_latest = pick(&releases, &def, true).filter(|b| b.beta);
        out.push(Component {
            id: def.id.into(),
            label: def.label.into(),
            family: def.family.into(),
            source: def.repo.into(),
            note: def.note.into(),
            error: err,
            latest,
            beta_latest,
            pinned_url: None,
            channel: def.channel.into(),
        });
    }

    out.push(Component {
        id: "optiscaler-imported".into(),
        label: "OptiScaler (Imported Builds)".into(),
        family: "optiscaler".into(),
        source: "your payload".into(),
        note: "Builds imported from disk that do not match a known fork release name.".into(),
        latest: None,
        beta_latest: None,
        pinned_url: None,
        error: None,
        channel: "local".into(),
    });
    let reshade = reshade_release(refresh, &mut cache);
    out.push(Component {
        id: "reshade".into(),
        label: "ReShade (add-on build)".into(),
        family: "feeder".into(),
        source: "reshade.me".into(),
        note: "Host for the feeder add-on; Vulkan layer support included.".into(),
        latest: reshade,
        beta_latest: None,
        pinned_url: None,
        error: None,
        channel: "stable".into(),
    });

    for (id, label, family, note, url) in [
        (
            "lumenite",
            "LumeniteFX (motion vectors)",
            "feeder",
            "Recommended MV provider for the feeder (kernel).",
            "https://codeload.github.com/umar-afzaal/LumeniteFX/zip/refs/heads/mainline",
        ),
        (
            "installer-feeder",
            "DLSS5-Feeder Installer Script",
            "feeder",
            "Upstream installer script (ReShade, layer registration, shader setup). Fetched from the feeder repo.",
            "https://raw.githubusercontent.com/jlrouzies-fr/DLSS5-Feeder/main/tools/Install-DLSS5Feeder.ps1",
        ),
    ] {
        out.push(Component {
            id: id.into(),
            label: label.into(),
            family: family.into(),
            source: if url.is_empty() { "local payload".into() } else { url.into() },
            note: note.into(),
            latest: None,
            beta_latest: None,
            pinned_url: if url.is_empty() { None } else { Some(url.into()) },
            error: None,
            channel: "pinned".into(),
        });
    }

    if refresh {
        cache.last_check = now_secs();
    }
    save_cache(&cache);
    out
}

pub fn release_file<'a>(rel: &'a ReleaseInfo, needle: &str) -> Option<&'a ReleaseFile> {
    rel.file(needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn live_registry_prefers_stable_releases() {
        let comps = list(true, false);
        for c in &comps {
            let v = c
                .latest
                .as_ref()
                .map(|l| format!("{} ({})", l.version, l.files.len()))
                .unwrap_or_else(|| "none".into());
            println!("{:22} latest-stable: {}", c.id, v);
        }
        let w = comps.iter().find(|c| c.id == "optiscaler-wilsjo2").unwrap();
        assert_eq!(w.latest.as_ref().map(|l| l.version.as_str()), Some("0.8.3"));
        assert!(w.beta_latest.is_some(), "0.8.4 pre-release still surfaced as beta");
        let feeder = comps.iter().find(|c| c.id == "feeder").unwrap();
        assert!(feeder.latest.is_some());
        let sm86 = comps.iter().find(|c| c.id == "sm86").unwrap();
        assert_eq!(sm86.latest.as_ref().map(|l| l.version.as_str()), Some("0.3.4"));
        let nr = comps.iter().find(|c| c.id == "runtime-nr-compat").unwrap();
        assert!(nr.latest.is_some(), "compat runtime resolvable");
    }
}
