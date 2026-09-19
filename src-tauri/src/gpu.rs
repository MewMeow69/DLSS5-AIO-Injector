use crate::model::GpuInfo;
use winreg::enums::*;
use winreg::RegKey;

const GPU_CLASS: &str = r"SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}";

fn is_virtual(name: &str) -> bool {
    let n = name.to_lowercase();
    [
        "virtual", "parsec", "sunshine", "meta ", "oculus", "basic display", "basic render", "remote",
        "idd", "displaylink", "spacedesk", "duet",
    ]
    .iter()
    .any(|k| n.contains(k))
}

fn classify(name: &str) -> (String, String, String, bool) {
    let n = name.to_uppercase();
    let vendor = if n.contains("NVIDIA") {
        "nvidia"
    } else if n.contains("RADEON") || n.contains("AMD") {
        "amd"
    } else if n.contains("INTEL") || n.contains("ARC") {
        "intel"
    } else {
        "unknown"
    };
    let (family, runtime, supported) = if n.contains("RTX 50") || n.contains("RTX PRO 6") {
        ("blackwell", "nvidia-310.8", true)
    } else if n.contains("RTX 40") {
        ("ada", "shortfuse-sf-v2", true)
    } else if n.contains("RTX 30") {
        ("ampere", "shortfuse-sf-v2", true)
    } else if n.contains("RTX 20") {
        ("turing", "shortfuse-sf-v2", true)
    } else if vendor == "nvidia" {
        ("nvidia-legacy", "none", false)
    } else {
        ("non-nvidia", "none", false)
    };
    (vendor.into(), family.into(), runtime.into(), supported)
}

pub fn gpu_info() -> GpuInfo {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(class) = hklm.open_subkey(GPU_CLASS) else {
        return GpuInfo::default();
    };
    let mut best: Option<GpuInfo> = None;
    for sub in class.enum_keys().flatten() {
        let Ok(k) = class.open_subkey(&sub) else { continue };
        let name = k.get_value::<String, _>("DriverDesc").unwrap_or_default();
        if name.is_empty() || is_virtual(&name) {
            continue;
        }
        let driver = k.get_value::<String, _>("DriverVersion").unwrap_or_default();
        let vram = k
            .get_raw_value("HardwareInformation.qwMemorySize")
            .ok()
            .and_then(|b| {
                if b.bytes.len() >= 8 {
                    Some(u64::from_le_bytes(b.bytes[0..8].try_into().ok()?))
                } else {
                    None
                }
            });
        let (vendor, family, runtime, supported) = classify(&name);
        let info = GpuInfo {
            name,
            vendor,
            family,
            driver,
            vram_bytes: vram,
            nr_runtime: runtime,
            nr_supported: supported,
        };
        let better = match &best {
            None => true,
            Some(b) => (!b.nr_supported && info.nr_supported) || (b.vram_bytes.unwrap_or(0) < info.vram_bytes.unwrap_or(0)),
        };
        if better {
            best = Some(info);
        }
    }
    best.unwrap_or_default()
}
