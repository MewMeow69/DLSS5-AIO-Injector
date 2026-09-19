import { useCallback, useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { open } from "@tauri-apps/plugin-dialog";
import {
  appPaths,
  cacheStats,
  checkAppUpdate,
  clearDownloads,
  getAppSettings,
  importArtifacts,
  removeManualRoot,
  resetAppData,
  setAppSettings,
} from "../lib/api";
import { setAllowBeta } from "../lib/prefs";
import type { AppPaths, AppSettings, AppUpdate, CacheStats } from "../lib/types";
import { Bolt, Check, Folder, Refresh, Trash } from "./icons";
import { Field, Popup, SectionLabel, Select, Toggle } from "./ui";

function fmtBytes(n: number) {
  if (!n) return "0 MB";
  return n >= 1024 ** 3 ? `${(n / 1024 ** 3).toFixed(2)} GB` : `${(n / 1024 ** 2).toFixed(1)} MB`;
}

export function SettingsPage({
  onGamesChanged,
  onArtifactsChanged,
}: {
  onGamesChanged: (games: import("../lib/types").Game[]) => void;
  onArtifactsChanged: () => void;
}) {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [paths, setPaths] = useState<AppPaths | null>(null);
  const [stats, setStats] = useState<CacheStats | null>(null);
  const [update, setUpdate] = useState<AppUpdate | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);

  const load = useCallback(async () => {
    const [s, p, c] = await Promise.all([getAppSettings(), appPaths(), cacheStats()]);
    setSettings(s);
    setPaths(p);
    setStats(c);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const patch = async (p: Partial<AppSettings>) => {
    if (!settings) return;
    const next = { ...settings, ...p };
    setSettings(next);
    try {
      await setAppSettings(next);
      setStatus("Saved");
      setTimeout(() => setStatus(null), 1500);
    } catch (e) {
      setStatus(String(e));
    }
  };

  const doCheckUpdate = async () => {
    setBusy(true);
    setUpdate(await checkAppUpdate());
    setBusy(false);
  };

  const doImport = async () => {
    try {
      const picked = await open({ directory: true, multiple: false, title: "Folder With Mod Files" });
      if (typeof picked !== "string") return;
      setBusy(true);
      const added = await importArtifacts(picked);
      setStatus(`Imported ${added.length} file(s)`);
      onArtifactsChanged();
      await load();
    } catch (e) {
      setStatus(String(e));
    }
    setBusy(false);
  };

  const doClear = async () => {
    setBusy(true);
    try {
      const [removed, freed] = await clearDownloads();
      setStatus(`Cleared ${fmtBytes(freed)} and ${removed} downloaded artifact(s)`);
      onArtifactsChanged();
      await load();
    } catch (e) {
      setStatus(String(e));
    }
    setBusy(false);
  };

  const doReset = async () => {
    setConfirmReset(false);
    setBusy(true);
    try {
      await resetAppData();
      setStatus("All app data removed — restart the app to start fresh");
      await load();
    } catch (e) {
      setStatus(String(e));
    }
    setBusy(false);
  };

  if (!settings) {
    return <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5 text-[12px] text-deck-muted">Loading settings…</div>;
  }

  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-[20px] font-bold tracking-normal">Settings</h1>
          <div className="mt-1 text-[12px] font-medium text-deck-muted">
            {status ?? "App preferences, storage and updates"}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button className="btn btn-primary" onClick={() => void doCheckUpdate()} disabled={busy}>
            <Refresh />
            Check for App Updates
          </button>
        </div>
      </div>

      {update && (
        <div
          className="pop-in mt-3.5 rounded-2xl px-3.5 py-2.5 text-[12px]"
          style={{
            background: update.newer ? "rgba(142,241,79,0.10)" : "rgba(255,255,255,0.03)",
            boxShadow: update.newer
              ? "inset 0 0 0 1px rgba(142,241,79,0.25)"
              : "inset 0 0 0 1px rgba(255,255,255,0.07)",
          }}
        >
          {update.error ? (
            <span className="text-deck-amber">Update check failed: {update.error}</span>
          ) : update.newer ? (
            <div className="flex items-center justify-between gap-3">
              <span>
                <b className="text-deck-green">Version {update.latest} is available</b>{" "}
                <span className="text-deck-muted">(you have {update.current})</span>
              </span>
              <button className="btn btn-success btn-sm" onClick={() => openPath(update.url).catch(() => {})}>
                Open Release Page
              </button>
            </div>
          ) : (
            <span className="text-deck-muted">You are on the newest release ({update.current}).</span>
          )}
        </div>
      )}

      <section className="mt-6">
        <SectionLabel>Install Behaviour</SectionLabel>
        <div className="tile px-4 py-2">
          <Field label="Default NR Preset" hint="Used when a game has no saved choice">
            <Select
              value={settings.defaultPreset}
              options={[
                { value: "auto", label: "Auto (by GPU)" },
                { value: "ultra", label: "Ultra" },
                { value: "quality", label: "Quality" },
                { value: "balanced", label: "Balanced" },
                { value: "performance", label: "Performance" },
              ]}
              onChange={(v) => void patch({ defaultPreset: v })}
            />
          </Field>
          <Field label="Leave NR disabled after install" hint="Enable it yourself in the OptiScaler overlay (Insert)">
            <Toggle on={settings.keepNrDisabled} onChange={(v) => void patch({ keepNrDisabled: v })} />
          </Field>
          <Field label="Allow pre-releases and betas" hint="Also affects the Components page channel picker">
            <Toggle
              on={settings.allowBeta}
              onChange={(v) => {
                setAllowBeta(v);
                void patch({ allowBeta: v });
              }}
            />
          </Field>
          <Field label="Check for component updates on start" hint="Checks GitHub at most every 30 minutes">
            <Toggle on={settings.checkUpdatesOnStart} onChange={(v) => void patch({ checkUpdatesOnStart: v })} />
          </Field>
        </div>
      </section>

      <section className="mt-6">
        <SectionLabel>GitHub Access</SectionLabel>
        <div className="tile px-4 py-3">
          <div className="text-[11.5px] text-deck-muted">
            Optional. The unauthenticated GitHub API allows 60 requests per hour; a token raises that to 5000 and
            removes the occasional "rate limit" warning. A classic token with <b>public_repo</b> read access is enough.
          </div>
          <div className="mt-2 flex items-center gap-2">
            <input
              type="password"
              value={settings.githubToken}
              placeholder="ghp_…"
              onChange={(e) => setSettings({ ...settings, githubToken: e.target.value })}
              className="inset flex-1 px-3 py-2 text-[12.5px] outline-none"
            />
            <button className="btn" onClick={() => void patch({ githubToken: settings.githubToken })}>
              <Check />
              Save Token
            </button>
          </div>
        </div>
      </section>

      <section className="mt-6">
        <SectionLabel>Storage</SectionLabel>
        <div className="tile px-4 py-2">
          <Field label="Downloaded files" hint={paths?.downloadsDir}>
            <span className="text-[12px] text-deck-muted">{fmtBytes(stats?.downloadsBytes ?? 0)}</span>
          </Field>
          <Field label="Extraction scratch space" hint={paths?.stagingDir}>
            <span className="text-[12px] text-deck-muted">{fmtBytes(stats?.stagingBytes ?? 0)}</span>
          </Field>
          <Field label="Registered artifacts">
            <span className="text-[12px] text-deck-muted">
              {stats?.artifacts ?? 0} ({stats?.imported ?? 0} imported · {stats?.downloaded ?? 0} downloaded)
            </span>
          </Field>
          <Field label="Data folder" hint={paths?.appDir}>
            <div className="flex gap-2">
              <button className="btn btn-sm" onClick={() => void doImport()} disabled={busy}>
                <Folder />
                Import Folder
              </button>
              <button className="btn btn-sm" onClick={() => openPath(paths?.appDir ?? "").catch(() => {})}>
                Open
              </button>
              <button className="btn btn-amber btn-sm" onClick={() => void doClear()} disabled={busy}>
                <Trash />
                Clear Downloads
              </button>
            </div>
          </Field>
        </div>
        <div className="mt-2 text-[11px] text-deck-muted">
          Clearing downloads only removes files the app fetched — imported payload files are untouched, and anything
          missing is downloaded again on the next install.
        </div>
      </section>

      {settings.manualRoots.length > 0 && (
        <section className="mt-6">
          <SectionLabel>Manually Added Game Folders</SectionLabel>
          <div className="tile px-4 py-3">
            {settings.manualRoots.map((r) => (
              <div key={r} className="flex items-center justify-between gap-3 border-b border-white/[0.045] py-1.5 last:border-0">
                <span className="min-w-0 flex-1 truncate text-[12px]">{r}</span>
                <button
                  className="btn btn-sm"
                  onClick={async () => {
                    const games = await removeManualRoot(r);
                    onGamesChanged(games);
                    await load();
                  }}
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
        </section>
      )}

      <section className="mt-6">
        <SectionLabel>About</SectionLabel>
        <div className="tile px-4 py-3 text-[12px] leading-relaxed">
          <div className="flex items-center gap-2">
            <span className="font-bold">DLSS5 AIO Injector</span>
            <span className="rounded-md bg-white/[0.06] px-1.5 py-[3px] text-[10px] font-bold text-deck-muted">
              v{paths?.version ?? "?"}
            </span>
          </div>
          <div className="mt-1 text-deck-muted">
            One-click Neural Rendering and Frame Generation setup. Nothing is bundled: every component is downloaded
            from its official source during install. GPL-3.0.
          </div>
          <div className="mt-2 flex flex-wrap gap-2">
            <button className="btn btn-sm" onClick={() => openPath("https://github.com/MewMeow69/DLSS5-AIO-Injector").catch(() => {})}>
              <Bolt />
              Project Page
            </button>
            <button
              className="btn btn-sm"
              onClick={() => openPath("https://github.com/MewMeow69/DLSS5-AIO-Injector/releases").catch(() => {})}
            >
              Releases
            </button>
            <button className="btn btn-sm" onClick={() => openPath(paths?.settingsFile ?? "").catch(() => {})}>
              Settings File
            </button>
            <button className="btn btn-rose btn-sm" onClick={() => setConfirmReset(true)} disabled={busy}>
              <Trash />
              Reset App Data
            </button>
          </div>
        </div>
      </section>

      <Popup open={confirmReset} title="Remove all app data?" onClose={() => setConfirmReset(false)}>
        <p>
          This deletes the download cache, the artifact registry and your preferences. Installed games are not touched —
          their files and rollback snapshots live in the game folders.
        </p>
        <div className="mt-3 flex gap-2">
          <button className="btn btn-rose" onClick={() => void doReset()}>
            Yes, Remove Everything
          </button>
          <button className="btn" onClick={() => setConfirmReset(false)}>
            Cancel
          </button>
        </div>
      </Popup>
    </div>
  );
}
