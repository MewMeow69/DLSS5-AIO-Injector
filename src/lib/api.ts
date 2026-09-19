import { invoke } from "@tauri-apps/api/core";
import type {
  AppPaths,
  AppSettings,
  AppUpdate,
  Artifact,
  CacheStats,
  Component,
  Detection,
  Game,
  GameSettings,
  GameState,
  GpuInfo,
  InstallOptions,
  InstallPlan,
  InstallReport,
} from "./types";

export const scanLibrary = () => invoke<Game[]>("scan_library");

export const detectGame = (installDir: string, exeHint?: string | null) =>
  invoke<Detection>("detect_game", { installDir, exeHint: exeHint ?? null });

export const systemInfo = () => invoke<GpuInfo>("system_info");

export const steamSearchAppId = (term: string) =>
  invoke<string | null>("steam_search_appid", { term });

export const addManualRoot = (path: string) => invoke<Game[]>("add_manual_root", { path });

export const listArtifacts = () => invoke<Artifact[]>("list_artifacts");

export const setPreferredArtifact = (kind: string, path: string | null) =>
  invoke<Artifact[]>("set_preferred_artifact", { kind, path });

export const preferredArtifacts = () => invoke<Record<string, string>>("preferred_artifacts");

export const importArtifacts = (folder?: string) =>
  invoke<Artifact[]>("import_artifacts", { folder: folder ?? null });

export const listComponents = (refresh: boolean, allowBeta: boolean) =>
  invoke<Component[]>("list_components", { refresh, allowBeta });

export const componentsCheckedAt = () => invoke<number>("components_checked_at");

export const runInstallerFile = (path: string) => invoke<void>("run_installer_file", { path });

export const getAppSettings = () => invoke<AppSettings>("get_app_settings");
export const setAppSettings = (settings: AppSettings) => invoke<void>("set_app_settings", { settings });
export const appPaths = () => invoke<AppPaths>("app_paths");
export const cacheStats = () => invoke<CacheStats>("cache_stats");
export const clearDownloads = () => invoke<[number, number]>("clear_downloads");
export const resetAppData = () => invoke<void>("reset_app_data");
export const checkAppUpdate = () => invoke<AppUpdate>("check_app_update");
export const removeManualRoot = (path: string) => invoke<Game[]>("remove_manual_root", { path });

export const readGameSettings = (installDir: string) =>
  invoke<GameSettings>("read_game_settings", { installDir });

export const writeGameSettings = (installDir: string, settings: GameSettings) =>
  invoke<string[]>("write_game_settings", { installDir, settings });

export const downloadComponent = (id: string, variant?: string | null, channel: "stable" | "beta" = "stable") =>
  invoke<Artifact>("download_component", { id, variant: variant ?? null, channel });

export const planInstall = (installDir: string, exeHint: string | null, options: InstallOptions) =>
  invoke<InstallPlan>("plan_install", { installDir, exeHint, options });

export const installGame = (installDir: string, exeHint: string | null, options: InstallOptions) =>
  invoke<InstallReport>("install_game", { installDir, exeHint, options });

export const gameState = (installDir: string) => invoke<GameState>("game_state", { installDir });

export const rollbackGame = (installDir: string) => invoke<string>("rollback_game", { installDir });

export const uninstallGame = (installDir: string) => invoke<string>("uninstall_game", { installDir });

export const COMPONENT_KIND_LABEL: Record<string, string> = {
  optiscaler: "OptiScaler",
  feeder: "Feeder",
  reshade: "ReShade",
  optipatcher: "OptiPatcher",
  sm86: "DLSSG sm86",
  "runtime-nr-nvidia": "NR runtime (50)",
  "runtime-nr-compat": "NR runtime (20/30/40)",
  "runtime-dlss": "DLSS runtime",
  lumenite: "LumeniteFX",
  streamline: "Streamline",
};

export const STORE_LABEL: Record<string, string> = {
  steam: "Steam",
  epic: "Epic",
  gog: "GOG",
  manual: "Manual",
};

export const API_LABEL: Record<string, string> = {
  dx12: "DX12",
  dx11: "DX11",
  vulkan: "Vulkan",
  opengl: "OpenGL",
  d3d9: "D3D9",
};

export const TECH_LABEL: Record<string, string> = {
  dlss: "DLSS",
  "dlss-rr": "DLSS RR",
  fsr: "FSR",
  xess: "XeSS",
  nr: "NR",
  dlssg: "DLSSG",
  fsrfg: "FSR FG",
  xefg: "Xe FG",
  "streamline-fg": "SL FG",
};

export const RUNTIME_LABEL: Record<string, string> = {
  "nvidia-310.8": "NVIDIA 310.8 (RTX 50)",
  "shortfuse-sf-v2": "ShortFuse SF-v2 (RTX 20/30/40)",
  none: "unsupported",
};

export function fmtBytes(n: number | null | undefined): string {
  if (!n) return "—";
  const gb = n / 1024 ** 3;
  return gb >= 1 ? `${gb.toFixed(1)} GB` : `${(n / 1024 ** 2).toFixed(0)} MB`;
}
