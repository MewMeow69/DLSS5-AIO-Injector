import { useCallback, useEffect, useState } from "react";
import { readGameSettings, rollbackGame, writeGameSettings } from "../lib/api";
import type { Detection, Game, GameSettings } from "../lib/types";
import { Bolt, Check, Refresh } from "./icons";
import { Field, Pill, Range, SectionLabel, Select, Toggle } from "./ui";

const FG_INPUTS = [
  { value: "auto", label: "Auto" },
  { value: "nofg", label: "No FG" },
  { value: "dlssg", label: "DLSSG (game's own)" },
  { value: "nvngxfg", label: "Nvngx FG (replacement)" },
  { value: "fsrfg", label: "FSR FG" },
  { value: "upscaler", label: "Upscaler (OptiFG)" },
];

const FG_OUTPUTS = [
  { value: "auto", label: "Auto" },
  { value: "nofg", label: "Off" },
  { value: "dlssg", label: "DLSSG (Streamline)" },
  { value: "fsrfg", label: "FSR FG (AMD runtime)" },
  { value: "xefg", label: "XeFG (Intel runtime)" },
];

const FG_REPLACEMENTS = [
  { value: "auto", label: "Auto" },
  { value: "none", label: "None" },
  { value: "nukems", label: "Nukem's (FSR3)" },
  { value: "arturs", label: "Artur's (FSR MFG)" },
  { value: "ffx", label: "FFX (FSR 3/4 FG)" },
  { value: "combo", label: "Combo (FFX + Enabler)" },
];

const PROVIDER_LABEL: Record<string, string> = {
  optiscaler: "OptiScaler NR",
  "renodx-base": "RenoDX (base add-on)",
  "renodx-speedlemur": "RenoDX DLSS5 (Speedlemur, HDR Transfer = 0 fix)",
  "renodx-performance": "RenoDX DLSS5 (performance fix)",
  "renodx-dlss5": "RenoDX DLSS5",
};

export function SettingsPanel({ game }: { game: Game; detection?: Detection }) {
  const [settings, setSettings] = useState<GameSettings | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      setSettings(await readGameSettings(game.installDir));
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [game.installDir]);

  useEffect(() => {
    void load();
  }, [load]);

  const patch = (fn: (s: GameSettings) => GameSettings) => setSettings((prev) => (prev ? fn(structuredClone(prev)) : prev));

  const save = async () => {
    if (!settings) return;
    setBusy(true);
    setError(null);
    try {
      const files = await writeGameSettings(game.installDir, settings);
      setStatus(files.length ? `Saved ${files.join(", ")}` : "Nothing to write for this install");
      setTimeout(() => setStatus(null), 2500);
    } catch (e) {
      setError(String(e));
    }
    setBusy(false);
  };

  const rollback = async () => {
    setBusy(true);
    try {
      setStatus(await rollbackGame(game.installDir));
      await load();
      setTimeout(() => setStatus(null), 3000);
    } catch (e) {
      setError(String(e));
    }
    setBusy(false);
  };

  if (!settings) {
    return (
      <div className="tile mt-4 p-4 text-[12px] text-deck-muted">
        {error ? error : "Reading settings…"}
      </div>
    );
  }

  const showNr = settings.optiscalerInstalled || settings.managed;
  const showFg = settings.optiscalerInstalled || settings.managed;
  const showReshade = settings.reshadeInstalled || settings.feederInstalled;

  return (
    <div className="mt-4 space-y-3">
      <div className="tile p-4">
        <div className="flex items-center justify-between">
          <SectionLabel>Neural Rendering Provider</SectionLabel>
          <div className="flex gap-2">
            <button className="btn btn-sm" onClick={() => void load()} disabled={busy}>
              <Refresh />
              Reload
            </button>
            <button className="btn btn-primary btn-sm" onClick={() => void save()} disabled={busy}>
              <Check />
              {busy ? "Saving…" : "Save Settings"}
            </button>
          </div>
        </div>
        <div className="mt-1 flex items-center gap-2">
          <Pill tone={settings.provider === "optiscaler" ? "accent" : "violet"}>
            {PROVIDER_LABEL[settings.provider] ?? settings.provider}
          </Pill>
          {settings.feederInstalled && <Pill tone="green">Feeder</Pill>}
          {settings.reshadeInstalled && <Pill tone="cyan">ReShade</Pill>}
          {!settings.managed && <Pill tone="amber">Not Installed by DLSS5 AIO</Pill>}
        </div>
        {status && <div className="mt-2 text-[11.5px] text-deck-green">{status}</div>}
        {error && <div className="mt-2 text-[11.5px] text-deck-rose">{error}</div>}
      </div>

      {showNr && (
        <div className="tile p-4">
          <SectionLabel>Neural Rendering</SectionLabel>
          <Field label="Enable NR" hint="Off until you turn it on; applies on next launch">
            <Toggle on={settings.nr.enabled} onChange={(v) => patch((s) => ((s.nr.enabled = v), s))} />
          </Field>
          <Field label="Generate Before Upscale" hint="Cheaper: NR runs on the pre-upscale image">
            <Toggle on={settings.nr.runBeforeSr} onChange={(v) => patch((s) => ((s.nr.runBeforeSr = v), s))} />
          </Field>
          <Field label="Model Resolution" hint="WorkingScale - lower is faster, higher is sharper">
            <Range
              value={settings.nr.workingScale}
              min={0.25}
              max={2}
              step={0.01}
              onChange={(v) => patch((s) => ((s.nr.workingScale = v), s))}
            />
          </Field>
          <Field label="Model Passes" hint="2-3 layers cost 2-3× model time">
            <Range
              value={settings.nr.passes}
              min={1}
              max={3}
              step={1}
              onChange={(v) => patch((s) => ((s.nr.passes = Math.round(v)), s))}
            />
          </Field>
          <Field label="Detail Strength">
            <Range
              value={settings.nr.intensity}
              min={0}
              max={2}
              step={0.05}
              onChange={(v) => patch((s) => ((s.nr.intensity = v), s))}
            />
          </Field>
          <Field label="Colour Strength" hint="0 keeps the game's colour exactly">
            <Range
              value={settings.nr.colourStrength}
              min={0}
              max={1}
              step={0.05}
              onChange={(v) => patch((s) => ((s.nr.colourStrength = v), s))}
            />
          </Field>
          <Field label="Local Structure">
            <Range
              value={settings.nr.localStructure}
              min={0}
              max={2}
              step={0.05}
              onChange={(v) => patch((s) => ((s.nr.localStructure = v), s))}
            />
          </Field>
          <Field label="Style" hint="0 standard · 1 natural · 2 cinematic">
            <Select
              value={String(settings.nr.style)}
              options={[
                { value: "0", label: "Standard" },
                { value: "1", label: "Natural" },
                { value: "2", label: "Cinematic" },
              ]}
              onChange={(v) => patch((s) => ((s.nr.style = parseInt(v, 10)), s))}
            />
          </Field>
          <Field label="HDR Transfer" hint="Experimental brightness transfer">
            <Toggle on={settings.nr.hdrTransfer} onChange={(v) => patch((s) => ((s.nr.hdrTransfer = v), s))} />
          </Field>
          <Field label="Scan Exposure">
            <Toggle on={settings.nr.scanExposure} onChange={(v) => patch((s) => ((s.nr.scanExposure = v), s))} />
          </Field>
          <Field label="Auto Mask">
            <Toggle on={settings.nr.autoMask} onChange={(v) => patch((s) => ((s.nr.autoMask = v), s))} />
          </Field>
          <Field label="Apply To Finished Picture" hint="DX12 only; early generation + late apply">
            <Toggle
              on={settings.nr.finishedPicture}
              onChange={(v) => patch((s) => ((s.nr.finishedPicture = v), s))}
            />
          </Field>
        </div>
      )}

      {showNr && (
        <div className="tile p-4">
          <SectionLabel>DLSS Super Resolution</SectionLabel>
          <Field label="Render Preset Override" hint="Force one DLSS preset for every quality mode">
            <Toggle
              on={settings.dlss.presetOverride}
              onChange={(v) => patch((s) => ((s.dlss.presetOverride = v), s))}
            />
          </Field>
          {settings.dlss.presetOverride && (
            <Field label="Preset" hint="Newer letters ship newer DLSS transformers (E/F are common picks)">
              <Select
                value={String(settings.dlss.preset)}
                options={[
                  { value: "0", label: "Default (game)" },
                  { value: "12", label: "K" },
                  { value: "11", label: "J" },
                  { value: "10", label: "I" },
                  { value: "9", label: "H" },
                  { value: "8", label: "G" },
                  { value: "7", label: "F" },
                  { value: "5", label: "E" },
                  { value: "4", label: "D" },
                  { value: "3", label: "C" },
                ]}
                onChange={(v) => patch((s) => ((s.dlss.preset = parseInt(v, 10)), s))}
              />
            </Field>
          )}
          <Field label="Generic NGX App ID" hint="Fixes preset overrides in games where they do not apply">
            <Toggle
              on={settings.dlss.useGenericAppid}
              onChange={(v) => patch((s) => ((s.dlss.useGenericAppid = v), s))}
            />
          </Field>
        </div>
      )}

      {showFg && (
        <div className="tile p-4">
          <SectionLabel>Frame Generation</SectionLabel>
          <Field label="Enable FG">
            <Toggle on={settings.fg.enabled} onChange={(v) => patch((s) => ((s.fg.enabled = v), s))} />
          </Field>
          <Field label="FG Input" hint="Where the frames come from">
            <Select
              value={settings.fg.input}
              options={FG_INPUTS}
              onChange={(v) => patch((s) => ((s.fg.input = v), s))}
            />
          </Field>
          <Field label="FG Output" hint="FSR FG works on any GPU; XeFG needs Borderless Fullscreen">
            <Select
              value={settings.fg.output}
              options={FG_OUTPUTS}
              onChange={(v) => patch((s) => ((s.fg.output = v), s))}
            />
          </Field>
          <Field label="Nvngx Replacement" hint="Nukem's / Artur's / FFX runtime">
            <Select
              value={settings.fg.replacement}
              options={FG_REPLACEMENTS}
              onChange={(v) => patch((s) => ((s.fg.replacement = v), s))}
            />
          </Field>
          <Field label="Generated Frames" hint="1 = 2× · 2 = 3× · 3 = 4× · 5 = 6× (NVIDIA MFG)">
            <Range
              value={settings.fg.interpolationCount}
              min={1}
              max={5}
              step={1}
              onChange={(v) => patch((s) => ((s.fg.interpolationCount = Math.round(v)), s))}
            />
          </Field>
          <Field label="HUDFix" hint="FSR/FG HUD ghosting fix (OptiFG)">
            <Toggle on={settings.fg.hudfix} onChange={(v) => patch((s) => ((s.fg.hudfix = v), s))} />
          </Field>
          <Field label="Depth Copy" hint="Keeps FG stable, small VRAM cost">
            <Toggle on={settings.fg.makeDepthCopy} onChange={(v) => patch((s) => ((s.fg.makeDepthCopy = v), s))} />
          </Field>
          <Field label="Motion Vector Copy">
            <Toggle on={settings.fg.makeMvCopy} onChange={(v) => patch((s) => ((s.fg.makeMvCopy = v), s))} />
          </Field>
          <Field label="Ignore Hudless Image">
            <Toggle
              on={settings.fg.disableHudless}
              onChange={(v) => patch((s) => ((s.fg.disableHudless = v), s))}
            />
          </Field>
        </div>
      )}

      {showReshade && (
        <div className="tile p-4">
          <SectionLabel>ReShade Feed (motion vectors + depth)</SectionLabel>
          <Field label="Motion-Vector Provider" hint="Must match the enabled LumeniteFX technique">
            <Select
              value={String(settings.reshade.mvProvider)}
              options={[
                { value: "3", label: "LumeniteFX Kernel (3)" },
                { value: "4", label: "QuantMotion (4)" },
              ]}
              onChange={(v) => patch((s) => ((s.reshade.mvProvider = parseInt(v, 10)), s))}
            />
          </Field>
          <Field label="Validate Motion Vectors">
            <Toggle on={settings.reshade.mvValidate} onChange={(v) => patch((s) => ((s.reshade.mvValidate = v), s))} />
          </Field>
          <Field label="Mask Strength">
            <Range
              value={settings.reshade.maskStrength}
              min={0}
              max={1}
              step={0.05}
              onChange={(v) => patch((s) => ((s.reshade.maskStrength = v), s))}
            />
          </Field>
          <Field label="Geometry Motion Mask" hint="Helps with foliage/disocclusion">
            <Toggle on={settings.reshade.geomEnable} onChange={(v) => patch((s) => ((s.reshade.geomEnable = v), s))} />
          </Field>
          {Object.entries(settings.reshade.techniques).map(([name, on]) => (
            <Field key={name} label={name}>
              <Toggle
                on={on}
                onChange={(v) => patch((s) => ((s.reshade.techniques[name] = v), s))}
              />
            </Field>
          ))}
        </div>
      )}

      {!showNr && !showFg && !showReshade && (
        <div className="tile p-4 text-[12px] text-deck-muted">
          <Bolt /> Install the mod first - settings appear once OptiScaler.ini or ReShadePreset.ini exists.
        </div>
      )}

      <div className="text-[11px] text-deck-muted">
        Settings writes are journalled:{" "}
        <button className="underline" onClick={() => void rollback()} disabled={busy}>
          Roll Back
        </button>{" "}
        restores the previous files.
      </div>
    </div>
  );
}
