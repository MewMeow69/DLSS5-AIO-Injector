export type Store = "steam" | "epic" | "gog" | "manual";

export interface Game {
  id: string;
  store: Store;
  name: string;
  installDir: string;
  exe: string | null;
  appId: string | null;
  sizeBytes: number | null;
  libraryOnline: boolean;
}

export interface ModState {
  optiscaler: string | null;
  optiscalerVersion: string | null;
  reshade: string | null;
  feeder: boolean;
  nrRuntime: boolean;
  nrRuntimeKind: string | null;
  dlssgSm86: boolean;
  optipatcher: boolean;
  streamline: boolean;
  backupDir: string | null;
}

export interface Detection {
  exe: string | null;
  exeDir: string | null;
  arch: string | null;
  api: string[];
  engine: string | null;
  upscalers: string[];
  framegen: string[];
  mods: ModState;
  warnings: string[];
  scannedFiles: number;
  error: string | null;
}

export interface GpuInfo {
  name: string;
  vendor: string;
  family: string;
  driver: string;
  vramBytes: number | null;
  nrRuntime: string;
  nrSupported: boolean;
}

export interface ReleaseFile {
  name: string;
  url: string;
  size: number;
}

export interface ReleaseInfo {
  version: string;
  tag: string;
  title: string;
  published: string;
  page: string;
  beta: boolean;
  files: ReleaseFile[];
}

export interface Component {
  id: string;
  label: string;
  family: string;
  source: string;
  note: string;
  latest: ReleaseInfo | null;
  betaLatest: ReleaseInfo | null;
  pinnedUrl: string | null;
  error: string | null;
  channel: string;
}

export interface NrSettings {
  enabled: boolean;
  runBeforeSr: boolean;
  workingScale: number;
  passes: number;
  intensity: number;
  style: number;
  preset: number;
  localStructure: number;
  transferStrength: number;
  colourStrength: number;
  autoMask: boolean;
  scanExposure: boolean;
  hdrTransfer: boolean;
  finishedPicture: boolean;
  compare: boolean;
}

export interface FgSettings {
  enabled: boolean;
  input: string;
  output: string;
  replacement: string;
  interpolationCount: number;
  hudfix: boolean;
  makeDepthCopy: boolean;
  makeMvCopy: boolean;
  disableHudless: boolean;
}

export interface ReShadeSettings {
  installed: boolean;
  mvProvider: number;
  mvValidate: boolean;
  maskStrength: number;
  geomEnable: boolean;
  debugView: number;
  techniques: Record<string, boolean>;
}

export interface DlssSettings {
  presetOverride: boolean;
  preset: number;
  useGenericAppid: boolean;
}

export interface AppSettings {
  manualRoots: string[];
  githubToken: string;
  checkUpdatesOnStart: boolean;
  keepNrDisabled: boolean;
  defaultPreset: string;
  allowBeta: boolean;
}

export interface AppPaths {
  appDir: string;
  downloadsDir: string;
  cacheDir: string;
  stagingDir: string;
  settingsFile: string;
  version: string;
}

export interface CacheStats {
  downloadsBytes: number;
  stagingBytes: number;
  cacheBytes: number;
  artifacts: number;
  imported: number;
  downloaded: number;
}

export interface VerifyCheck {
  label: string;
  ok: boolean;
  detail: string;
}

export interface VerifyReport {
  checks: VerifyCheck[];
  hints: string[];
  managed: boolean;
}

export interface AppUpdate {
  current: string;
  latest: string;
  url: string;
  newer: boolean;
  error: string | null;
}

export interface GameSettings {
  provider: string;
  optiscalerInstalled: boolean;
  reshadeInstalled: boolean;
  feederInstalled: boolean;
  nr: NrSettings;
  fg: FgSettings;
  reshade: ReShadeSettings;
  dlss: DlssSettings;
  managed: boolean;
}

export interface Artifact {
  kind: string;
  version: string;
  variant: string | null;
  path: string;
  sha256: string;
  size: number;
  origin: string;
  added: number;
  mtime: number;
  note: string | null;
}

export interface InstallOptions {
  optiscalerPath: string | null;
  installFeeder: boolean;
  installSm86: boolean;
  optipatcher: boolean;
  streamline: boolean;
  preset: string;
  nrEnabled: boolean;
  rtx40Mfg: boolean;
  force: boolean;
  nrProvider: string;
  fgInput: string;
  fgOutput: string;
  fgReplacement: string;
  xessLibs: boolean;
  dlssEnabler: boolean;
  runtimeDlssd: boolean;
  hudfix: boolean;
}

export interface PlanStep {
  id: string;
  title: string;
  detail: string;
  available: boolean;
}

export interface InstallPlan {
  exeDir: string | null;
  exe: string | null;
  proxy: string;
  steps: PlanStep[];
  warnings: string[];
}

export interface InstallReport {
  placed: string[];
  skipped: string[];
  warnings: string[];
  proxy: string;
}

export interface ComponentRecord {
  kind: string;
  version: string;
}

export interface GameState {
  managed: boolean;
  installedAt: string | null;
  components: ComponentRecord[];
  options: InstallOptions | null;
  backups: string[];
  addedFiles: number;
}

export interface DownloadProgress {
  kind: string;
  file: string;
  received: number;
  total: number;
  done: boolean;
  error: string | null;
}

export interface InstallProgress {
  stage: string;
  title: string;
  detail: string;
  done: boolean;
  error: string | null;
}
