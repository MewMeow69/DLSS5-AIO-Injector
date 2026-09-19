import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { runInstallerFile, getAppSettings } from "../lib/api";
import {
  downloadComponent,
  gameState,
  installGame,
  listArtifacts,
  planInstall,
  rollbackGame,
  uninstallGame,
} from "../lib/api";
import type {
  Artifact,
  Detection,
  DownloadProgress,
  Game,
  GameState,
  GpuInfo,
  InstallOptions,
  InstallPlan,
  InstallProgress,
} from "../lib/types";
import { Bolt, Download, List, Play, Trash, Undo } from "./icons";
import { FORK_LABEL } from "../lib/kinds";
import { Field, Pill, Popup, Progress, SectionLabel, Select, Toggle } from "./ui";

const PRESETS = [
  { id: "ultra", label: "Ultra", hint: "Model resolution 100% · 2 passes" },
  { id: "quality", label: "Quality", hint: "Model resolution 100% · 1 pass" },
  { id: "balanced", label: "Balanced", hint: "Model resolution 67% · 1 pass" },
  { id: "performance", label: "Performance", hint: "Model resolution 50% · 1 pass" },
];

const STAGE_TITLES: Record<string, string> = {
  optiscaler: "OptiScaler NR",
  "runtime-nr": "NR Runtime",
  "runtime-dlss": "DLSS Runtime",
  feeder: "ReShade + Feeder",
  sm86: "DLSSG SM86 Frame Generation",
  streamline: "Streamline Set",
  optipatcher: "OptiPatcher",
};

function defaultOptions(detection: Detection | undefined, gpu: GpuInfo | null): InstallOptions {
  const noUpscaler = !detection || (detection.upscalers.length === 0 && detection.framegen.length === 0);
  const ampereOrTuring = gpu?.family === "ampere" || gpu?.family === "turing";
  const preset = ampereOrTuring ? "balanced" : "quality";
  return {
    optiscalerPath: null,
    installFeeder: noUpscaler,
    installSm86: ampereOrTuring && (detection?.framegen.includes("dlssg") || noUpscaler) === true,
    optipatcher: false,
    streamline: ampereOrTuring,
    preset,
    nrEnabled: false,
    rtx40Mfg: false,
    force: false,
    nrProvider: "optiscaler",
    fgInput: "auto",
    fgOutput: "auto",
    fgReplacement: "auto",
    xessLibs: false,
    dlssEnabler: false,
    runtimeDlssd: false,
    hudfix: false,
  };
}

function Chip({
  on,
  onClick,
  children,
  tone = "green",
  title,
}: {
  on: boolean;
  onClick: () => void;
  children: React.ReactNode;
  tone?: "green" | "cyan" | "violet" | "amber" | "accent";
  title?: string;
}) {
  return (
    <button
      title={title}
      onClick={onClick}
      className={`chip ${on ? `chip-on chip-on-${tone}` : ""}`}
    >
      {children}
    </button>
  );
}

export function InstallPanel({
  game,
  detection,
  gpu,
}: {
  game: Game;
  detection?: Detection;
  gpu: GpuInfo | null;
}) {
  const storeKey = `neurodeck.opts.${game.id}`;
  const [options, setOptions] = useState<InstallOptions>(() => {
    try {
      const raw = localStorage.getItem(storeKey);
      if (raw) return { ...defaultOptions(detection, gpu), ...(JSON.parse(raw) as InstallOptions) };
    } catch {
      /* ignore */
    }
    return defaultOptions(detection, gpu);
  });
  const [state, setState] = useState<GameState | null>(null);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [download, setDownload] = useState<DownloadProgress | null>(null);
  const [help, setHelp] = useState(false);
  const [plan, setPlan] = useState<InstallPlan | null>(null);
  const [progress, setProgress] = useState<InstallProgress[]>([]);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showLog, setShowLog] = useState(false);

  useEffect(() => {
    setOptions((prev) => {
      const fresh = defaultOptions(detection, gpu);
      return { ...fresh, ...prev, preset: prev.preset || fresh.preset };
    });
  }, [detection, gpu]);

  useEffect(() => {
    localStorage.setItem(storeKey, JSON.stringify(options));
  }, [options, storeKey]);

  useEffect(() => {
    void listArtifacts().then(setArtifacts).catch(() => {});
    void getAppSettings()
      .then((s) => {
        if (s.defaultPreset && s.defaultPreset !== "auto") {
          setOptions((o) => ({ ...o, preset: s.defaultPreset }));
        }
      })
      .catch(() => {});
    const un = listen<DownloadProgress>("download:progress", (e) => {
      setDownload(e.payload);
      if (e.payload.done) {
        setTimeout(() => setDownload(null), 1200);
        void listArtifacts().then(setArtifacts).catch(() => {});
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const reloadState = () => {
    void gameState(game.installDir)
      .then(setState)
      .catch(() => setState(null));
  };
  useEffect(reloadState, [game.installDir]);

  useEffect(() => {
    const un = listen<InstallProgress>("install:progress", (e) => {
      const p = { ...e.payload, title: STAGE_TITLES[e.payload.stage] ?? e.payload.title };
      setProgress((prev) => {
        const idx = prev.findIndex((x) => x.stage === p.stage && x.title === p.title);
        if (idx === -1) return [...prev, p];
        if (p.detail && (!prev[idx].done || p.done)) {
          const copy = [...prev];
          copy[idx] = p;
          return copy;
        }
        return prev;
      });
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  const set = (patch: Partial<InstallOptions>) => setOptions((o) => ({ ...o, ...patch }));

  const preview = async () => {
    setError(null);
    try {
      setPlan(await planInstall(game.installDir, detection?.exe ?? null, options));
    } catch (e) {
      setError(String(e));
    }
  };

  const doInstall = async () => {
    setRunning(true);
    setProgress([]);
    setResult(null);
    setError(null);
    try {
      const report = await installGame(game.installDir, detection?.exe ?? null, options);
      const parts = [`${report.placed.length} items placed as ${report.proxy}`];
      if (report.warnings.length) parts.push(`${report.warnings.length} warning(s)`);
      setResult(parts.join(" · "));
      reloadState();
    } catch (e) {
      setError(String(e));
    }
    setRunning(false);
  };

  const doRollback = async () => {
    setError(null);
    try {
      setResult(await rollbackGame(game.installDir));
      reloadState();
    } catch (e) {
      setError(String(e));
    }
  };

  const doUninstall = async () => {
    try {
      setResult(await uninstallGame(game.installDir));
      reloadState();
    } catch (e) {
      setError(String(e));
    }
  };

  const installed = state?.managed ?? false;
  const hasDlssEnabler = useMemo(() => artifacts.some((a) => a.kind === "dlss-enabler"), [artifacts]);
  const gameHasUpscaler = !!detection && (detection.upscalers.length > 0 || detection.framegen.length > 0);
  const feederRequired = options.nrProvider !== "optiscaler" || !gameHasUpscaler;
  const enablerInstallerPath = useMemo(
    () => artifacts.find((a) => a.kind === "dlss-enabler-installer")?.path ?? null,
    [artifacts],
  );
  const hasEnablerInstaller = enablerInstallerPath !== null;
  const buildOptions = useMemo(() => {
    const opts = [{ value: "", label: "Auto — newest build across forks" }];
    const pools = ["optiscaler-wilsjo2", "optiscaler-janblade", "optiscaler-mfg", "optiscaler-nightly", "optiscaler"];
    for (const kind of pools) {
      for (const a of artifacts.filter((x) => x.kind === kind)) {
        const parts = [FORK_LABEL[kind] ?? kind, a.version];
        if (a.variant && a.variant !== "standard") parts.push(a.variant);
        if (a.origin === "import") parts.push("imported");
        opts.push({ value: a.path, label: parts.join(" · ") });
      }
    }
    return opts;
  }, [artifacts]);
  const installedLabel = useMemo(() => {
    if (!state?.components?.length) return null;
    return state.components.map((c) => `${c.kind} ${c.version}`).join(" · ");
  }, [state]);

  return (
    <div className="tile mt-4 p-4">
      <div className="flex items-center justify-between">
        <SectionLabel>{installed ? "Installed" : "Install"}</SectionLabel>
        {installed ? <Pill tone="green">Managed by DLSS5 AIO</Pill> : null}
      </div>

      {installed && installedLabel ? (
        <div className="-mt-0.5 text-[11.5px] text-deck-muted">
          {installedLabel}
          {state?.backups.length ? ` · ${state.backups.length} backup snapshot(s)` : ""}
        </div>
      ) : null}

      <div className="mt-3 flex flex-wrap gap-1.5">
        {PRESETS.map((p) => (
          <Chip key={p.id} on={options.preset === p.id} onClick={() => set({ preset: p.id })} title={p.hint} tone="accent">
            {p.label}
          </Chip>
        ))}
      </div>

      <div className="mt-3">
        <Field label="Neural Rendering Provider" hint="OptiScaler NR, or RenoDX through the ReShade feeder">
          <Select
            value={options.nrProvider}
            options={[
              { value: "optiscaler", label: "OptiScaler NR" },
              { value: "renodx-performance", label: "RenoDX DLSS5 (performance)" },
              { value: "renodx-speedlemur", label: "RenoDX DLSS5 (Speedlemur)" },
              { value: "renodx-base", label: "RenoDX (base add-on)" },
            ]}
            onChange={(v) =>
              set({
                nrProvider: v,
                installFeeder: v === "optiscaler" ? options.installFeeder : true,
                streamline: v === "optiscaler" ? options.streamline : false,
              })
            }
          />
        </Field>
        {options.nrProvider !== "optiscaler" && (
          <div className="mt-1.5 flex flex-wrap items-center gap-2 text-[11px] text-deck-muted">
            <Pill tone="violet">RenoDX path</Pill>
            OptiScaler is skipped; ReShade + the feeder do NR. The add-on file comes from your imported payload.
          </div>
        )}
      </div>

      {options.nrProvider === "optiscaler" && buildOptions.length > 1 && (
        <div className="mt-2">
          <Field label="OptiScaler Build" hint="NR fork builds, rtx40-mfg variants and the upstream nightly">
            <Select
              value={options.optiscalerPath ?? ""}
              options={buildOptions}
              onChange={(v) => set({ optiscalerPath: v || null })}
            />
          </Field>
        </div>
      )}

      {options.nrProvider === "optiscaler" && (
        <div className="mt-2">
          <Field label="Frame-Generation Output" hint="FSR FG and XeFG run on any GPU (XeFG: Borderless Fullscreen, XeSS 3.x libs + fakenvapi are installed automatically)">
            <Select
              value={options.fgOutput}
              options={[
                { value: "auto", label: "Auto (leave to the game)" },
                { value: "dlssg", label: "DLSSG (Streamline)" },
                { value: "fsrfg", label: "FSR FG" },
                { value: "xefg", label: "XeFG (Intel XeSS FG)" },
              ]}
              onChange={(v) => set({ fgOutput: v })}
            />
          </Field>
          <Field label="Nvngx Replacement" hint="Nukem's / Artur's / FFX framegen runtime">
            <Select
              value={options.fgReplacement}
              options={[
                { value: "auto", label: "Auto" },
                { value: "none", label: "None" },
                { value: "nukems", label: "Nukem's (FSR3)" },
                { value: "arturs", label: "Artur's (FSR MFG)" },
                { value: "ffx", label: "FFX (FSR 3/4 FG)" },
                { value: "combo", label: "Combo (FFX + Enabler)" },
              ]}
              onChange={(v) => set({ fgReplacement: v })}
            />
          </Field>
          {options.fgOutput === "xefg" && (
            <Field label="Use XeSS 3.x libraries" hint="libxess_fg + libxell from the Intel SDK: real XeFG on RTX / AMD too">
              <Toggle on={options.xessLibs} onChange={(v) => set({ xessLibs: v })} />
            </Field>
          )}
          {(options.fgOutput === "fsrfg" || options.fgReplacement === "arturs" || options.fgReplacement === "combo") && (
            <>
              <Field label="Install DLSS Enabler" hint="dlss-enabler-headless.dll into the OptiScaler folder (Artur's FSR MFG)">
                <Toggle on={options.dlssEnabler} onChange={(v) => set({ dlssEnabler: v })} />
              </Field>
              {options.dlssEnabler && !hasDlssEnabler && (
                <div className="mt-2 rounded-xl border border-deck-amber/25 bg-deck-amber/5 p-2.5">
                  <div className="text-[11.5px] text-deck-amber">
                    Its author ships an installer, so this one piece is manual — everything else downloads itself.
                  </div>
                  <div className="mt-2 flex flex-wrap gap-2">
                    <button
                      className="btn btn-primary btn-sm"
                      onClick={() => void downloadComponent("dlss-enabler")}
                      disabled={!!download}
                    >
                      <Download />
                      Download Installer
                    </button>
                    {hasEnablerInstaller && (
                      <button
                        className="btn btn-violet btn-sm"
                        onClick={() => void runInstallerFile(enablerInstallerPath!)}
                      >
                        <Bolt />
                        Run Installer
                      </button>
                    )}
                    <button className="btn btn-sm" onClick={() => setHelp(true)}>
                      Instructions
                    </button>
                  </div>
                  {download && (
                    <div className="mt-2">
                      <Progress value={download.received} max={download.total} />
                    </div>
                  )}
                </div>
              )}
            </>
          )}
          {(options.fgOutput === "fsrfg" || options.fgOutput === "xefg") && (
            <Field label="HUDfix" hint="Removes HUD ghosting on generated frames">
              <Toggle on={options.hudfix} onChange={(v) => set({ hudfix: v })} />
            </Field>
          )}
          <Field label="Ray Reconstruction runtime" hint="Places nvngx_dlssd.dll (DLSS RR games)">
            <Toggle on={options.runtimeDlssd} onChange={(v) => set({ runtimeDlssd: v })} />
          </Field>
        </div>
      )}

      <div className="mt-2.5 flex flex-wrap gap-1.5">
        <Chip
          on={feederRequired ? true : options.installFeeder}
          onClick={() => !feederRequired && set({ installFeeder: !options.installFeeder })}
          tone={feederRequired ? "amber" : "green"}
          title={
            feederRequired
              ? options.nrProvider === "optiscaler"
                ? "Required: this game has no upscaler, so NR needs the feeder's motion vectors and depth"
                : "Required: RenoDX runs through the ReShade feeder"
              : "Optional: this game already hands motion vectors to its upscaler, so ReShade is not installed"
          }
        >
          <Bolt />
          {feederRequired ? "Feeder · Required" : "Feeder"}
        </Chip>
        {options.nrProvider === "optiscaler" && (
          <>
            <Chip
              on={options.installSm86}
              onClick={() => set({ installSm86: !options.installSm86 })}
              title="DLSSG frame generation on RTX 20/30"
            >
              FG SM86
            </Chip>
            <Chip
              on={options.streamline}
              onClick={() => set({ streamline: !options.streamline })}
              title="sl.*.dll + nvngx_dlssg for OptiScaler's DLSSG output"
            >
              Streamline
            </Chip>
            <Chip
              on={options.optipatcher}
              onClick={() => set({ optipatcher: !options.optipatcher })}
              title="Unlock DLSS/DLSSG inputs without spoofing"
            >
              OptiPatcher
            </Chip>
            {(gpu?.family === "ada" || gpu?.family === "blackwell") && (
              <Chip
                on={options.rtx40Mfg}
                onClick={() => set({ rtx40Mfg: !options.rtx40Mfg })}
                tone="violet"
                title="Use the rtx40-mfg build + Ada MFG unlock (restart required)"
              >
                RTX 40 MFG
              </Chip>
            )}
            <Chip
              on={options.force}
              onClick={() => set({ force: !options.force })}
              tone="amber"
              title="Overwrite OptiScaler.ini instead of merging"
            >
              Force INI
            </Chip>
            <Chip
              on={options.nrEnabled}
              onClick={() => set({ nrEnabled: !options.nrEnabled })}
              tone="amber"
              title="Leave off and enable NR yourself in the OptiScaler overlay"
            >
              Enable NR Now
            </Chip>
          </>
        )}
      </div>

      <div className="mt-3.5 flex flex-wrap items-center gap-2">
        <button onClick={() => void doInstall()} disabled={running || !game.libraryOnline} className="btn btn-primary">
          <Play />
          {running ? "Installing…" : installed ? "Update / Repair" : "Install"}
        </button>
        <button onClick={() => void preview()} className="btn">
          <List />
          Preview Plan
        </button>
        {installed && (
          <>
            <button
              onClick={() => void doRollback()}
              disabled={!state?.backups.length}
              className="btn btn-amber btn-sm"
              title="Restore the files as they were before the last install"
            >
              <Undo />
              Roll Back
            </button>
            <button onClick={() => void doUninstall()} className="btn btn-rose btn-sm">
              <Trash />
              Uninstall
            </button>
          </>
        )}
      </div>

      {plan && (
        <div className="fade-in tile-flush mt-3 p-3">
          <SectionLabel>Plan</SectionLabel>
          <div className="mt-1 space-y-1">
            {plan.steps.map((s) => (
              <div key={s.id} className="flex items-baseline gap-2 text-[11.5px]">
                <span className={s.available ? "text-deck-green" : "text-deck-rose"}>{s.available ? "●" : "!"}</span>
                <span className="font-bold">{s.title}</span>
                <span className="truncate text-deck-muted">{s.detail}</span>
              </div>
            ))}
          </div>
          {plan.warnings.length > 0 && (
            <div className="mt-2 text-[11.5px] text-deck-amber">{plan.warnings.join(" · ")}</div>
          )}
        </div>
      )}

      {progress.length > 0 && (
        <div className="fade-in mt-3 space-y-1.5">
          {progress.map((p, i) => (
            <div key={`${p.stage}-${p.title}-${i}`} className="flex items-baseline gap-2 text-[11.5px]">
              <span className={p.error ? "text-deck-rose" : p.done ? "text-deck-green" : "text-deck-cyan"}>
                {p.error ? "✕" : p.done ? "✓" : "●"}
              </span>
              <span className="font-bold">{p.title}</span>
              <span className="truncate text-deck-muted">{p.detail}</span>
            </div>
          ))}
          <button className="text-[11px] font-semibold text-deck-muted underline" onClick={() => setShowLog((v) => !v)}>
            {showLog ? "Hide Log" : "Show Log"}
          </button>
          {showLog && (
            <pre className="inset max-h-40 overflow-y-auto p-2.5 text-[10.5px] leading-relaxed text-deck-muted">
              {progress.map((p) => `${p.title}: ${p.detail}`).join("\n")}
            </pre>
          )}
        </div>
      )}

      {result && <div className="mt-2 text-[11.5px] font-semibold text-deck-green">{result}</div>}
      {error && <div className="mt-2 text-[11.5px] font-semibold text-deck-rose">{error}</div>}

      <Popup open={help} title="DLSS Enabler — how to use it" onClose={() => setHelp(false)}>
        <p>
          Artur's DLSS Enabler unlocks FSR frame generation / MFG inside OptiScaler. Its author ships an installer, so
          this one step is manual — everything else in this app installs itself.
        </p>
        <ol className="list-decimal space-y-1 pl-5">
          <li>Press <b>Download Installer</b> here (or in Components).</li>
          <li>Press <b>Run Installer</b> and point it at this game folder.</li>
          <li>
            Then press <b>Update / Repair</b>: the app finds <code>dlss-enabler-headless.dll</code> in the game folder
            and places it into <code>OptiScaler\</code>.
          </li>
        </ol>
      </Popup>

      <div className="mt-2.5 text-[11px] leading-relaxed text-deck-muted">
        Everything missing is downloaded during install — no manual file drop needed. NR stays off afterwards: launch
        the game, press <b className="text-deck-text">Insert</b>, enable Neural Rendering, then press{" "}
        <b className="text-deck-text">Home</b> to confirm the feeder add-on loaded.
      </div>
    </div>
  );
}
