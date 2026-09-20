import { useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import type { Detection, Game, GpuInfo } from "../lib/types";
import { API_LABEL, fmtBytes, RUNTIME_LABEL, STORE_LABEL, TECH_LABEL } from "../lib/api";
import { GameArt } from "./GameCard";
import { Copy, Close, Folder, Refresh } from "./icons";
import { InstallPanel } from "./InstallPanel";
import { SettingsPanel } from "./SettingsPanel";
import { Check, Pill, Row, SectionLabel } from "./ui";

export function GameDetail({
  game,
  detection,
  artId,
  gpu,
  detecting,
  onClose,
  onRescan,
}: {
  game: Game;
  detection?: Detection;
  artId: string | null;
  gpu: GpuInfo | null;
  detecting: boolean;
  onClose: () => void;
  onRescan: () => void;
}) {
  const [copied, setCopied] = useState<string | null>(null);
  const [tab, setTab] = useState<"install" | "detection" | "settings">("install");
  const mods = detection?.mods;
  const nrReady =
    !!gpu?.nrSupported && (!!detection && (detection.upscalers.length > 0 || detection.framegen.length > 0 || mods?.feeder));
  const unityVulkanHint = detection?.engine === "Unity";

  const copy = (text: string) => {
    navigator.clipboard.writeText(text).catch(() => {});
    setCopied(text);
    setTimeout(() => setCopied(null), 1500);
  };

  return (
    <>
      <div className="fade-in absolute inset-0 z-30 bg-black/60 backdrop-blur-[2px]" onClick={onClose} />
      <aside className="slide-in absolute inset-y-0 right-0 z-40 flex w-[472px] max-w-[94vw] flex-col border-l border-deck-line bg-deck-panel/95 backdrop-blur-2xl">
        <div className="relative h-56 shrink-0 overflow-hidden">
          <GameArt game={game} artId={artId} className="absolute inset-0 h-full w-full" />
          <div className="absolute inset-0 bg-gradient-to-t from-deck-panel via-deck-panel/45 to-transparent" />
          <button onClick={onClose} className="btn btn-ghost btn-sm absolute right-3 top-3" title="Close">
            <Close />
            Esc
          </button>
          <div className="absolute inset-x-5 bottom-3">
            <div className="text-[19px] font-bold leading-tight drop-shadow-md">{game.name}</div>
            <div className="mt-1 flex items-center gap-2 text-[11.5px] font-medium text-deck-muted">
              <span>{STORE_LABEL[game.store] ?? game.store}</span>
              {game.sizeBytes ? <span>· {fmtBytes(game.sizeBytes)}</span> : null}
              {game.appId ? <span>· {game.appId}</span> : null}
            </div>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto px-5 pb-6">
          <div className="fade-in mt-3 flex flex-wrap gap-1.5">
            {detection?.api.map((a) => (
              <Pill key={a} tone="cyan">
                {API_LABEL[a] ?? a}
              </Pill>
            ))}
            {detection?.engine && <Pill>{detection.engine}</Pill>}
            {detection?.arch && <Pill>{detection.arch}</Pill>}
            {detection?.upscalers.map((u) => (
              <Pill key={u} tone="violet">
                {TECH_LABEL[u] ?? u}
              </Pill>
            ))}
            {detection?.framegen.map((f) => (
              <Pill key={f} tone="green">
                {TECH_LABEL[f] ?? f}
              </Pill>
            ))}
            {nrReady && <Pill tone="green">NR Ready</Pill>}
            {detecting && <Pill tone="cyan">Scanning…</Pill>}
          </div>

          {detection?.error ? (
            <div
              className="mt-3 rounded-xl p-3 text-[12.5px] text-deck-rose"
              style={{ background: "rgba(255,125,148,0.09)", boxShadow: "inset 0 0 0 1px rgba(255,125,148,0.28)" }}
            >
              {detection.error}
            </div>
          ) : null}

          {detection?.warnings.map((w) => (
            <div
              key={w}
              className="mt-3 rounded-xl p-3 text-[12.5px] text-deck-amber"
              style={{ background: "rgba(253,192,69,0.09)", boxShadow: "inset 0 0 0 1px rgba(253,192,69,0.28)" }}
            >
              {w}
            </div>
          ))}

          <div className="tabs mt-4">
            {(["install", "detection", "settings"] as const).map((t) => (
              <button key={t} className={`tab ${tab === t ? "tab-on" : ""}`} onClick={() => setTab(t)}>
                {t === "install" ? "Install" : t === "detection" ? "Detection" : "Settings"}
              </button>
            ))}
          </div>

          {tab === "settings" && <SettingsPanel game={game} detection={detection} />}

          {tab === "install" && <InstallPanel game={game} detection={detection} gpu={gpu} />}

          {tab === "detection" && (
            <>
          <div className="mt-4">
            <SectionLabel>Detection</SectionLabel>
            <div className="tile-flush px-3.5 py-1.5">
              <Row label="Executable">
                <span className="break-all text-deck-muted">{detection?.exe?.split("\\").slice(-1)[0] ?? "-"}</span>
              </Row>
              <Row label="Folder">
                <button
                  className="max-w-[240px] truncate font-semibold text-deck-cyan hover:underline"
                  onClick={() => openPath(game.installDir).catch(() => {})}
                >
                  {game.installDir}
                </button>
              </Row>
              <Row label="Files Scanned">{detection?.scannedFiles ?? "-"}</Row>
              <Row label="GPU Runtime">
                <span className={gpu?.nrSupported ? "font-semibold text-deck-green" : "text-deck-muted"}>
                  {RUNTIME_LABEL[gpu?.nrRuntime ?? "none"] ?? gpu?.nrRuntime ?? "-"}
                </span>
              </Row>
            </div>
          </div>

          <div className="mt-4">
            <SectionLabel>Installed Mods</SectionLabel>
            <div className="tile-flush px-3.5 py-1.5">
              <Row label="OptiScaler">
                <span className="inline-flex items-center gap-2">
                  <Check on={!!mods?.optiscaler} />
                  <span className="text-deck-muted">{mods?.optiscaler ?? "Not Installed"}</span>
                </span>
              </Row>
              <Row label="ReShade">
                <span className="inline-flex items-center gap-2">
                  <Check on={!!mods?.reshade} />
                  <span className="text-deck-muted">{mods?.reshade ?? "Not Installed"}</span>
                </span>
              </Row>
              <Row label="DLSS5 Feeder">
                <span className="inline-flex items-center gap-2">
                  <Check on={!!mods?.feeder} />
                </span>
              </Row>
              <Row label="NR Runtime">
                <span className="inline-flex items-center gap-2">
                  <Check on={!!mods?.nrRuntime} />
                  <span className="text-deck-muted">{mods?.nrRuntimeKind ?? "Missing"}</span>
                </span>
              </Row>
              <Row label="DLSSG SM86 FG">
                <span className="inline-flex items-center gap-2">
                  <Check on={!!mods?.dlssgSm86} />
                </span>
              </Row>
              <Row label="OptiPatcher">
                <span className="inline-flex items-center gap-2">
                  <Check on={!!mods?.optipatcher} />
                </span>
              </Row>
              <Row label="Streamline">
                <span className="inline-flex items-center gap-2">
                  <Check on={!!mods?.streamline} />
                </span>
              </Row>
              {mods?.optiscalerAsi && (
                <Row label="OptiScaler add-on">
                  <span className="inline-flex items-center gap-2">
                    <Check on={true} />
                    <span className="text-deck-muted">OptiScaler.asi - replaced on the next install</span>
                  </span>
                </Row>
              )}
              {mods?.dxvk && (
                <Row label="DXVK">
                  <span className="inline-flex items-center gap-2">
                    <Check on={true} />
                    <span className="text-deck-muted">Vulkan wrapper - OptiScaler hooks will not attach</span>
                  </span>
                </Row>
              )}
              {mods?.dgvoodoo && (
                <Row label="dgVoodoo2">
                  <span className="inline-flex items-center gap-2">
                    <Check on={true} />
                    <span className="text-deck-muted">wrapped output bypasses OptiScaler</span>
                  </span>
                </Row>
              )}
              {mods?.dlssEnabler && (
                <Row label="DLSS Enabler">
                  <span className="inline-flex items-center gap-2">
                    <Check on={true} />
                    <span className="text-deck-muted">nvngx.dll wrapper - can conflict with NR</span>
                  </span>
                </Row>
              )}
              {mods?.fakenvapi && (
                <Row label="fakenvapi">
                  <span className="inline-flex items-center gap-2">
                    <Check on={true} />
                    <span className="text-deck-muted">Reflex → XeLL bridge</span>
                  </span>
                </Row>
              )}
            </div>
          </div>

          {unityVulkanHint && (
            <div
              className="mt-4 rounded-2xl p-3.5 text-[12.5px]"
              style={{
                background: "rgba(69,215,245,0.08)",
                boxShadow: "inset 0 1px 0 rgba(255,255,255,0.1), inset 0 0 0 1px rgba(69,215,245,0.24)",
              }}
            >
              <div className="font-bold text-deck-cyan">Unity Title</div>
              <div className="mt-1 text-deck-muted">
                The feeder's D3D11 path crashes in some Unity games. Launch with{" "}
                <code className="rounded bg-black/45 px-1.5 py-0.5 font-bold text-deck-text">-force-vulkan</code>.
              </div>
              <button className="btn btn-cyan btn-sm mt-2.5" onClick={() => copy("-force-vulkan")}>
                <Copy />
                {copied === "-force-vulkan" ? "Copied" : "Copy Launch Option"}
              </button>
            </div>
          )}

          <div className="mt-4 text-[11.5px] leading-relaxed text-deck-muted">
            In Game: <b className="text-deck-text">Insert</b> opens OptiScaler, <b className="text-deck-text">Home</b>{" "}
            opens ReShade. NR stays off until you enable it in the OptiScaler overlay.
          </div>
            </>
          )}
        </div>

        <div className="flex shrink-0 items-center gap-2 border-t border-deck-line px-5 py-3.5">
          <button className="btn" onClick={onRescan} disabled={detecting}>
            <Refresh />
            Rescan
          </button>
          <button className="btn" onClick={() => openPath(game.installDir).catch(() => {})}>
            <Folder />
            Open Folder
          </button>
        </div>
      </aside>
    </>
  );
}
