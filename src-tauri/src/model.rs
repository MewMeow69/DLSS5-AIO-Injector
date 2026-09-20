use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub store: String,
    pub name: String,
    pub install_dir: String,
    pub exe: Option<String>,
    pub app_id: Option<String>,
    pub size_bytes: Option<u64>,
    pub library_online: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModState {
    pub optiscaler: Option<String>,
    pub optiscaler_version: Option<String>,
    pub optiscaler_asi: bool,
    pub reshade: Option<String>,
    pub feeder: bool,
    pub nr_runtime: bool,
    pub nr_runtime_kind: Option<String>,
    pub dlssg_sm86: bool,
    pub optipatcher: bool,
    pub streamline: bool,
    pub dxvk: bool,
    pub dgvoodoo: bool,
    pub dlss_enabler: bool,
    pub fakenvapi: bool,
    pub backup_dir: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub exe: Option<String>,
    pub exe_dir: Option<String>,
    pub arch: Option<String>,
    pub api: Vec<String>,
    pub engine: Option<String>,
    pub upscalers: Vec<String>,
    pub framegen: Vec<String>,
    pub mods: ModState,
    pub warnings: Vec<String>,
    pub scanned_files: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub family: String,
    pub driver: String,
    pub vram_bytes: Option<u64>,
    pub nr_runtime: String,
    pub nr_supported: bool,
}

impl Default for GpuInfo {
    fn default() -> Self {
        GpuInfo {
            name: "No GPU detected".into(),
            vendor: "unknown".into(),
            family: "unknown".into(),
            driver: String::new(),
            vram_bytes: None,
            nr_runtime: "none".into(),
            nr_supported: false,
        }
    }
}
