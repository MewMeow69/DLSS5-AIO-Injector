import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import { ComponentsPage } from "./components/ComponentsPage";
import { GameCard } from "./components/GameCard";
import { GameDetail } from "./components/GameDetail";
import { FolderPlus, Refresh, Upload } from "./components/icons";
import { Sidebar, type View } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { addManualRoot, detectGame, listArtifacts, listComponents, scanLibrary, systemInfo } from "./lib/api";
import { cachedArtId, resolveArtId } from "./lib/art";
import { loadCached, saveCached } from "./lib/detectCache";
import type { Artifact, Component, Detection, Game, GpuInfo } from "./lib/types";
import { updatesAvailable } from "./lib/updates";

type Filter = "all" | "modded" | "nr-ready" | "no-upscaler" | "attention";

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All Games" },
  { id: "nr-ready", label: "NR Ready" },
  { id: "modded", label: "Modded" },
  { id: "no-upscaler", label: "No Upscaler" },
  { id: "attention", label: "Attention" },
];

async function runPool<T>(items: T[], workers: number, fn: (item: T) => Promise<void>) {
  let cursor = 0;
  const next = async (): Promise<void> => {
    const idx = cursor++;
    if (idx >= items.length) return;
    await fn(items[idx]);
    return next();
  };
  await Promise.all(Array.from({ length: Math.max(1, Math.min(workers, items.length)) }, next));
}

export default function App() {
  const [view, setView] = useState<View>("library");
  const [games, setGames] = useState<Game[]>([]);
  const [detections, setDetections] = useState<Record<string, Detection>>({});
  const [artIds, setArtIds] = useState<Record<string, string>>({});
  const [gpu, setGpu] = useState<GpuInfo | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [selected, setSelected] = useState<string | null>(null);
  const [busy, setBusy] = useState(true);
  const [progress, setProgress] = useState({ done: 0, total: 0 });
  const [loadingLabel, setLoadingLabel] = useState("Scanning Game Libraries…");
  const [artifacts, setArtifacts] = useState<Artifact[]>([]);
  const [components, setComponents] = useState<Component[]>([]);
  const detectionsRef = useRef(detections);
  detectionsRef.current = detections;

  const allowBeta = localStorage.getItem("neurodeck.beta") === "1";
  const updates = useMemo(() => updatesAvailable(components, artifacts, allowBeta), [components, artifacts, allowBeta]);

  useEffect(() => {
    listArtifacts().then(setArtifacts).catch(() => {});
    listComponents(false, allowBeta).then(setComponents).catch(() => {});
  }, [allowBeta]);

  const refresh = useCallback(async (force = false) => {
    setBusy(true);
    setLoadingLabel("Scanning Game Libraries…");
    let list: Game[] = [];
    try {
      list = await scanLibrary();
    } catch {
      list = [];
    }
    setGames(list);
    const initialArt: Record<string, string> = {};
    for (const g of list) {
      const id = cachedArtId(g);
      if (id) initialArt[g.id] = id;
    }
    setArtIds((prev) => ({ ...initialArt, ...prev }));

    const queue = list.filter((g) => g.libraryOnline && (force || !detectionsRef.current[g.id]));
    setProgress({ done: 0, total: queue.length });
    let done = 0;
    await runPool(queue, 5, async (g) => {
      let d = force ? null : loadCached(g.installDir);
      if (!d) {
        try {
          d = await detectGame(g.installDir, g.exe);
          saveCached(g.installDir, d);
        } catch {
          d = null;
        }
      }
      if (d) {
        setDetections((prev) => ({ ...prev, [g.id]: d as Detection }));
      }
      done += 1;
      setProgress({ done, total: queue.length });
      if (!initialArt[g.id]) {
        const id = await resolveArtId(g);
        if (id) setArtIds((prev) => ({ ...prev, [g.id]: id }));
      }
    });
    setBusy(false);
  }, []);

  useEffect(() => {
    systemInfo().then(setGpu).catch(() => {});
    void refresh(false);
  }, [refresh]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setSelected(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return games.filter((g) => {
      if (q && !g.name.toLowerCase().includes(q) && !g.store.includes(q)) return false;
      const d = detections[g.id];
      switch (filter) {
        case "modded":
          return !!d?.mods.optiscaler;
        case "nr-ready":
          return !!gpu?.nrSupported && !!d && (d.upscalers.length > 0 || d.framegen.length > 0 || d.mods.feeder);
        case "no-upscaler":
          return !!d && d.upscalers.length === 0 && d.framegen.length === 0;
        case "attention":
          return !!d && (d.warnings.length > 0 || !!d.error);
        default:
          return true;
      }
    });
  }, [games, detections, query, filter, gpu]);

  const selectedGame = games.find((g) => g.id === selected) ?? null;

  const rescanOne = useCallback(async (g: Game) => {
    try {
      const d = await detectGame(g.installDir, g.exe);
      saveCached(g.installDir, d);
      setDetections((prev) => ({ ...prev, [g.id]: d }));
    } catch {
      /* keep last result */
    }
  }, []);

  const addFolder = useCallback(async () => {
    try {
      const picked = await open({ directory: true, multiple: false, title: "Add a Game Folder" });
      if (typeof picked !== "string") return;
      const list = await addManualRoot(picked);
      setGames(list);
      void refresh(false);
    } catch {
      /* dialog cancelled */
    }
  }, [refresh]);

  const onArtifactsChanged = useCallback((next: Artifact[]) => {
    setArtifacts(next);
  }, []);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <TitleBar
        subtitle={
          view === "components"
            ? "Components"
            : `${games.length} Games${updates.count ? ` · ${updates.count} Update${updates.count === 1 ? "" : "s"}` : ""}`
        }
      />
      <div className="relative flex min-h-0 flex-1 overflow-hidden">
        <Sidebar gpu={gpu} view={view} onView={setView} updateCount={updates.count} />

        <main className="flex min-w-0 flex-1 flex-col">
        {view === "components" ? (
          <ComponentsPage onArtifactsChanged={onArtifactsChanged} />
        ) : (
          <>
            <header className="shrink-0 border-b border-deck-line px-6 pb-4 pt-5">
              <div className="flex items-center justify-between gap-4">
                <div>
                  <h1 className="text-[20px] font-bold tracking-normal">Library</h1>
                  <div className="mt-1 text-[12px] font-medium text-deck-muted">
                    {busy
                      ? `${loadingLabel} ${progress.total ? `(${progress.done}/${progress.total})` : ""}`
                      : `${filtered.length} of ${games.length} Games`}
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <div className="inset flex h-[38px] items-center px-3">
                    <svg viewBox="0 0 24 24" className="mr-2 h-3.5 w-3.5 text-deck-muted" fill="none" stroke="currentColor" strokeWidth="2.4">
                      <circle cx="11" cy="11" r="7" />
                      <path d="M20 20l-3.5-3.5" />
                    </svg>
                    <input
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      placeholder="Search Games…"
                      className="w-56 bg-transparent text-[13px] outline-none placeholder:text-deck-muted/70"
                    />
                  </div>
                  <button onClick={() => void addFolder()} className="btn btn-primary">
                    <FolderPlus />
                    Add Folder
                  </button>
                  <button onClick={() => void refresh(true)} disabled={busy} className="btn">
                    <Refresh />
                    Rescan All
                  </button>
                </div>
              </div>

              <div className="mt-3.5 flex flex-wrap items-center gap-2">
                {FILTERS.map((f) => (
                  <button
                    key={f.id}
                    onClick={() => setFilter(f.id)}
                    className={`chip ${filter === f.id ? "chip-on chip-on-accent" : ""}`}
                  >
                    {f.label}
                  </button>
                ))}
                {busy && progress.total > 0 && (
                  <div className="ml-auto h-2 w-48 overflow-hidden rounded-full" style={{ boxShadow: "inset 0 2px 4px rgba(0,0,0,0.7)" }}>
                    <div
                      className="h-full rounded-full bg-gradient-to-r from-deck-green to-deck-cyan transition-[width] duration-200"
                      style={{ width: `${(progress.done / Math.max(1, progress.total)) * 100}%` }}
                    />
                  </div>
                )}
              </div>

              {updates.count > 0 && (
                <div
                  className="pop-in mt-3.5 flex items-center justify-between gap-3 rounded-2xl px-3.5 py-2.5"
                  style={{
                    background: "rgba(107,124,255,0.12)",
                    boxShadow:
                      "inset 0 0 0 1px rgba(107,124,255,0.28)",
                  }}
                >
                  <div className="min-w-0 text-[12px]">
                    <span className="font-bold text-deck-green">
                      {updates.count} Component Update{updates.count === 1 ? "" : "s"} Available
                    </span>
                    <span className="ml-2 truncate text-deck-muted">{updates.items.slice(0, 2).join(" · ")}</span>
                  </div>
                  <button onClick={() => setView("components")} className="btn btn-success btn-sm shrink-0">
                    <Upload />
                    Review Updates
                  </button>
                </div>
              )}
            </header>

            <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
              {!busy && filtered.length === 0 ? (
                <div className="mt-24 text-center text-deck-muted">
                  <div className="text-[15px] font-bold text-deck-text">No Games Match</div>
                  <div className="mt-1 text-[12.5px]">Try a different filter or search.</div>
                </div>
              ) : (
                <div className="grid grid-cols-[repeat(auto-fill,minmax(172px,1fr))] gap-4">
                  {filtered.map((g) => (
                    <GameCard
                      key={g.id}
                      game={g}
                      detection={detections[g.id]}
                      artId={artIds[g.id] ?? null}
                      detecting={busy && !detections[g.id] && !loadCached(g.installDir)}
                      selected={selected === g.id}
                      onSelect={() => setSelected(g.id)}
                    />
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </main>
      </div>

      {selectedGame && view === "library" && (
        <GameDetail
          game={selectedGame}
          detection={detections[selectedGame.id]}
          artId={artIds[selectedGame.id] ?? null}
          gpu={gpu}
          detecting={busy && !detections[selectedGame.id]}
          onClose={() => setSelected(null)}
          onRescan={() => void rescanOne(selectedGame)}
        />
      )}
    </div>
  );
}
