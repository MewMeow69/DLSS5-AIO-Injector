import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  componentsCheckedAt,
  downloadComponent,
  importArtifacts,
  listArtifacts,
  listComponents,
  preferredArtifacts,
  runInstallerFile,
  setPreferredArtifact,
} from "../lib/api";
import type { Artifact, Component, DownloadProgress } from "../lib/types";
import { Bolt, Check, Download, FolderPlus, Pin, Refresh } from "./icons";
import { artifactKind, artifactsFor } from "../lib/kinds";
import { Pill, Popup, Progress, SectionLabel } from "./ui";

function fmtSize(n: number) {
  if (!n) return "";
  return n > 1024 ** 2 ? `${(n / 1024 ** 2).toFixed(1)} MB` : `${(n / 1024).toFixed(0)} KB`;
}

function fmtDate(epoch: number) {
  if (!epoch) return "";
  const d = new Date(epoch * 1000);
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

function ago(epoch: number) {
  if (!epoch) return "never";
  const secs = Math.max(0, Math.floor(Date.now() / 1000) - epoch);
  if (secs < 90) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} h ago`;
  return `${Math.floor(hours / 24)} d ago`;
}

function cmpVersion(a: string, b: string) {
  const pa = a.split(/[.-]/).map((x) => parseInt(x, 10) || 0);
  const pb = b.split(/[.-]/).map((x) => parseInt(x, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

type Filter = "all" | "updates" | "local" | "nightly";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "updates", label: "Updates" },
  { id: "local", label: "Local Files" },
  { id: "nightly", label: "Nightly" },
];

const FAMILIES: { id: string; title: string }[] = [
  { id: "optiscaler", title: "OptiScaler Builds" },
  { id: "runtime", title: "NVIDIA Runtimes" },
  { id: "feeder", title: "ReShade + Feeder Stack" },
  { id: "tool", title: "Tools, Frame Generation & XeSS" },
];

export function ComponentsPage({ onArtifactsChanged }: { onArtifactsChanged: (a: Artifact[]) => void }) {
  const [components, setComponents] = useState<Component[]>([]);
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [preferred, setPreferred] = useState<Record<string, string>>({});
  const [allowBeta, setAllowBeta] = useState(() => localStorage.getItem("neurodeck.beta") === "1");
  const [busy, setBusy] = useState(true);
  const [status, setStatus] = useState("");
  const [download, setDownload] = useState<DownloadProgress | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [checkedAt, setCheckedAt] = useState(0);
  const [showEnablerHelp, setShowEnablerHelp] = useState(false);

  const refresh = useCallback(
    async (force: boolean) => {
      setBusy(true);
      setStatus(force ? "Checking GitHub…" : "Loading Component List…");
      try {
        const [comps, arts, pref, at] = await Promise.all([
          listComponents(force, allowBeta),
          listArtifacts(),
          preferredArtifacts(),
          componentsCheckedAt(),
        ]);
        setComponents(comps);
        setArtifacts(arts);
        setPreferred(pref);
        setCheckedAt(at);
        onArtifactsChanged(arts);
        setStatus("");
      } catch (e) {
        setStatus(String(e));
      }
      setBusy(false);
    },
    [allowBeta, onArtifactsChanged],
  );

  useEffect(() => {
    void refresh(false);
  }, [refresh]);

  useEffect(() => {
    const un = listen<DownloadProgress>("download:progress", (e) => {
      setDownload(e.payload);
      if (e.payload.done) {
        setTimeout(() => setDownload(null), 1200);
        void listArtifacts().then((a) => {
          setArtifacts(a);
          onArtifactsChanged(a);
        });
      }
    });
    return () => {
      void un.then((f) => f());
    };
  }, [onArtifactsChanged]);

  const byKind = useMemo(() => {
    const map: Record<string, Artifact[]> = {};
    for (const a of artifacts) {
      (map[a.kind] ??= []).push(a);
    }
    for (const k of Object.keys(map)) {
      map[k].sort((x, y) => cmpVersion(y.version, x.version));
    }
    return map;
  }, [artifacts]);

  const doDownload = async (id: string, variant?: string, channel: "stable" | "beta" = "stable") => {
    setBusy(true);
    setStatus(`Downloading ${id}${channel === "beta" ? " (Pre-Release)" : ""}…`);
    try {
      const art = await downloadComponent(id, variant ?? null, channel);
      setStatus(`Downloaded ${art.version}`);
      const arts = await listArtifacts();
      setArtifacts(arts);
      onArtifactsChanged(arts);
    } catch (e) {
      setStatus(String(e));
    }
    setBusy(false);
  };

  const doImport = async () => {
    try {
      const picked = await open({ directory: true, multiple: false, title: "Folder With Mod Files" });
      if (typeof picked !== "string") return;
      setBusy(true);
      setStatus("Importing…");
      const arts = await importArtifacts(picked);
      setStatus(`Imported ${arts.length} File(s)`);
      const all = await listArtifacts();
      setArtifacts(all);
      onArtifactsChanged(all);
    } catch (e) {
      setStatus(String(e));
    }
    setBusy(false);
  };

  const choose = async (kind: string, path: string | null) => {
    const arts = await setPreferredArtifact(kind, path);
    setArtifacts(arts);
    onArtifactsChanged(arts);
    setPreferred(await preferredArtifacts());
  };

  const toggleBeta = () => {
    const next = !allowBeta;
    setAllowBeta(next);
    localStorage.setItem("neurodeck.beta", next ? "1" : "0");
  };

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return components.filter((c) => {
      if (q && !c.label.toLowerCase().includes(q) && !c.id.toLowerCase().includes(q) && !c.source.toLowerCase().includes(q)) {
        return false;
      }
      const kind = artifactKind(c.id);
      const local = artifactsFor(c.id, byKind);
      switch (filter) {
        case "nightly":
          return c.channel === "nightly";
        case "local":
          return local.length > 0;
        case "updates": {
          const rel = allowBeta ? c.betaLatest ?? c.latest : c.latest;
          if (!rel) return false;
          const best = local.reduce<string | null>(
            (acc, a) => (acc === null || cmpVersion(a.version, acc) > 0 ? a.version : acc),
            null,
          );
          if (c.channel === "nightly" || kind === "renodx" || kind === "streamline") return false;
          return !best || cmpVersion(rel.version, best) > 0;
        }
        default:
          return true;
      }
    });
  }, [components, byKind, filter, query, allowBeta]);

  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-[20px] font-bold tracking-normal">Components</h1>
          <div className="mt-1 text-[12px] font-medium text-deck-muted">
            {busy ? status || "Working…" : status || `Checked ${ago(checkedAt)} · cached 6 h · auto-checks on start`}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={toggleBeta} className={`chip ${allowBeta ? "chip-on chip-on-amber" : ""}`}>
            Pre-Releases: {allowBeta ? "On" : "Off"}
          </button>
          <button onClick={() => void doImport()} className="btn">
            <FolderPlus />
            Import Folder
          </button>
          <button onClick={() => void refresh(true)} disabled={busy} className="btn btn-primary">
            <Refresh />
            Check for Updates
          </button>
        </div>
      </div>

      <div className="mt-3.5 flex flex-wrap items-center gap-2">
        <div className="inset flex h-[34px] items-center px-3">
          <svg viewBox="0 0 24 24" className="mr-2 h-3.5 w-3.5 text-deck-muted" fill="none" stroke="currentColor" strokeWidth="2.4">
            <circle cx="11" cy="11" r="7" />
            <path d="M20 20l-3.5-3.5" />
          </svg>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search Components…"
            className="w-52 bg-transparent text-[12.5px] outline-none placeholder:text-deck-muted/70"
          />
        </div>
        {FILTERS.map((f) => (
          <button
            key={f.id}
            onClick={() => setFilter(f.id)}
            className={`chip ${filter === f.id ? "chip-on chip-on-accent" : ""}`}
          >
            {f.label}
          </button>
        ))}
        <span className="ml-auto text-[11.5px] text-deck-muted">
          {visible.length} of {components.length} Components
        </span>
      </div>

      {download && (
        <div className="tile-flush pop-in mt-3.5 px-3.5 py-2.5 text-[12px]">
          <div className="flex items-baseline justify-between gap-3">
            <div className="min-w-0 truncate">
              <span className="font-bold text-deck-cyan">{download.kind}</span> · {download.file}
            </div>
            <div className="shrink-0 text-deck-muted">
              {download.done ? (download.error ? `Failed: ${download.error}` : "Done") : ""}
            </div>
          </div>
          {!download.done && (
            <div className="mt-2">
              <Progress value={download.received} max={download.total} />
            </div>
          )}
        </div>
      )}

      <Popup
        open={showEnablerHelp}
        title="DLSS Enabler — how to use it"
        onClose={() => setShowEnablerHelp(false)}
      >
        <p>
          Artur's DLSS Enabler unlocks FSR frame generation / MFG inside OptiScaler. Its author ships an installer, so
          this one step is manual — everything else in this app installs itself.
        </p>
        <ol className="list-decimal space-y-1 pl-5">
          <li>Click <b>Download</b> above to fetch the official installer.</li>
          <li>Click <b>Run Installer</b> and point it at your game folder (the one with the game .exe).</li>
          <li>
            Back in the game page, turn on <b>Install DLSS Enabler</b> and press <b>Update / Repair</b>: the app picks
            up <code>dlss-enabler-headless.dll</code> from the game folder and places it in <code>OptiScaler\</code>.
          </li>
        </ol>
        <p className="text-deck-muted">
          Already have the DLL? Drop it into the game folder — the installer step will find it there.
        </p>
      </Popup>

      {FAMILIES.map((fam) => {
        const list = visible.filter((c) => c.family === fam.id);
        if (list.length === 0) return null;
        return (
          <section key={fam.id} className="mt-6">
            <SectionLabel>{fam.title}</SectionLabel>
            <div className="space-y-3">
              {list.map((c) => {
                const kind = artifactKind(c.id);
                const local = artifactsFor(c.id, byKind);
                const hasMfgAsset = [...(c.latest?.files ?? []), ...(c.betaLatest?.files ?? [])].some((f) =>
                  f.name.includes("rtx40-mfg"),
                );
                const picked = preferred[kind];
                const latest = c.latest;
                const newestLocal = local[0];
                const updateAvailable =
                  !!latest &&
                  c.channel !== "nightly" &&
                  (!newestLocal || cmpVersion(latest.version, newestLocal.version) > 0);
                return (
                  <div key={c.id} className="tile p-4">
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <div className="flex flex-wrap items-center gap-2">
                          <span className="text-[13.5px] font-bold">{c.label}</span>
                          {c.channel === "nightly" && <Pill tone="violet">Nightly</Pill>}
                          {c.channel === "local" && <Pill tone="cyan">Local Payload</Pill>}
                          {updateAvailable && <Pill tone="green">Update Available</Pill>}
                          {latest?.beta && <Pill tone="amber">Pre-Release</Pill>}
                        </div>
                        <div className="mt-1 truncate text-[11px] text-deck-muted">
                          {c.source}
                          {latest
                            ? ` · Latest Stable ${latest.version}${latest.published ? ` (${fmtDate(Date.parse(latest.published) / 1000)})` : ""}`
                            : c.pinnedUrl
                              ? " · pinned download"
                              : ""}
                        </div>
                        <div className="mt-1 text-[11.5px] text-deck-muted">{c.note}</div>
                        {c.error ? <div className="mt-1 text-[11.5px] text-deck-amber">{c.error}</div> : null}
                      </div>
                      <div className="flex shrink-0 flex-col items-end gap-1.5">
                        {c.id === "dlss-enabler" && (
                          <button className="btn btn-sm" onClick={() => setShowEnablerHelp(true)}>
                            Instructions
                          </button>
                        )}
                        {c.id === "dlss-enabler" &&
                          local
                            .filter((a) => a.kind === "dlss-enabler-installer")
                            .map((a) => (
                              <button
                                key={`run-${a.path}`}
                                className="btn btn-violet btn-sm"
                                onClick={() => void runInstallerFile(a.path)}
                                title="Run the DLSS Enabler setup"
                              >
                                <Bolt />
                                Run Installer
                              </button>
                            ))}
                        {latest && (
                          <button
                            onClick={() =>
                              void doDownload(c.id, c.id.startsWith("optiscaler") ? "standard" : undefined)
                            }
                            disabled={busy}
                            className="btn btn-primary btn-sm"
                          >
                            <Download />
                            {updateAvailable ? `Update to ${latest.version}` : `Download ${latest.version}`}
                          </button>
                        )}
                        {!latest && c.pinnedUrl && (
                          <button onClick={() => void doDownload(c.id)} disabled={busy} className="btn btn-primary btn-sm">
                            <Download />
                            Download
                          </button>
                        )}
                        {c.betaLatest && (allowBeta || !latest) && (
                          <button
                            onClick={() =>
                              void doDownload(c.id, c.id.startsWith("optiscaler") ? "standard" : undefined, "beta")
                            }
                            disabled={busy}
                            className="btn btn-amber btn-sm"
                          >
                            <Bolt />
                            Beta {c.betaLatest.version}
                          </button>
                        )}
                        {c.id.startsWith("optiscaler") && hasMfgAsset && (
                          <button
                            onClick={() => void doDownload(c.id, "rtx40-mfg")}
                            disabled={busy}
                            className="btn btn-violet btn-sm"
                          >
                            <Bolt />
                            RTX40-MFG Build
                          </button>
                        )}
                      </div>
                    </div>

                    {local.length > 0 ? (
                      <div className="mt-3 space-y-1.5 border-t border-white/[0.06] pt-2.5">
                        {local.map((a) => {
                          const active = picked ? a.path === picked : a === newestLocal;
                          const fileName = a.path.split("\\").pop() ?? "";
                          return (
                            <div key={a.path} className="flex items-start justify-between gap-3 text-[11.5px]">
                              <div className="min-w-0 flex-1">
                                <div className="truncate">
                                  <span className={active ? "font-bold text-deck-green" : "text-deck-text/85"}>
                                    {a.version}
                                    {a.variant ? ` · ${a.variant}` : ""}
                                  </span>
                                  <span className="text-deck-muted">
                                    {" "}
                                    · {a.origin} · {fmtSize(a.size)}
                                    {a.mtime ? ` · updated ${fmtDate(a.mtime)}` : ""} · {a.sha256.slice(0, 10)}
                                  </span>
                                </div>
                                <div className="truncate text-[10.5px] text-deck-muted/80">{fileName}</div>
                                {a.note ? <div className="text-[10.5px] text-deck-amber/90">{a.note}</div> : null}
                              </div>
                              <button
                                onClick={() => void choose(kind, active && picked ? null : a.path)}
                                className={`btn btn-sm ${active ? "btn-success" : "btn-ghost"}`}
                                title="Use this version for installs (roll back to an older build)"
                              >
                                {active ? picked ? <Pin /> : <Check /> : <Pin />}
                                {active ? (picked ? "Unpin" : "Active") : "Use"}
                              </button>
                            </div>
                          );
                        })}
                      </div>
                    ) : (
                      <div className="mt-3 border-t border-white/[0.06] pt-2.5 text-[11.5px] text-deck-muted">
                        No Local Copy Yet — Download It or Import a Folder That Holds It
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </section>
        );
      })}
    </div>
  );
}
