import { useEffect, useState } from "react";
import { steamHeader, steamPortrait } from "../lib/art";
import type { Detection, Game } from "../lib/types";
import { API_LABEL, STORE_LABEL, TECH_LABEL } from "../lib/api";
import { Pill } from "./ui";

export function GameArt({
  game,
  artId,
  className = "",
}: {
  game: Game;
  artId: string | null;
  className?: string;
}) {
  const [step, setStep] = useState(0);
  useEffect(() => setStep(0), [artId]);

  if (!artId || step > 1) {
    return (
      <div className={`flex items-center justify-center bg-gradient-to-br from-[#1b2333] via-[#0f141d] to-[#0a0d14] ${className}`}>
        <span className="text-3xl font-black tracking-tight text-white/15">
          {game.name.slice(0, 2).toUpperCase()}
        </span>
      </div>
    );
  }
  return (
    <img
      src={step === 0 ? steamPortrait(artId) : steamHeader(artId)}
      alt=""
      loading="lazy"
      draggable={false}
      onError={() => setStep((s) => s + 1)}
      className={`object-cover ${className}`}
    />
  );
}

export function GameCard({
  game,
  detection,
  artId,
  detecting,
  selected,
  onSelect,
}: {
  game: Game;
  detection?: Detection;
  artId: string | null;
  detecting: boolean;
  selected: boolean;
  onSelect: () => void;
}) {
  const mods = detection?.mods;
  const installed = !!mods?.optiscaler;
  const offline = !game.libraryOnline;
  const noUpscaler = detection ? detection.upscalers.length === 0 && detection.framegen.length === 0 : false;
  const warn = (detection?.warnings.length ?? 0) > 0;

  return (
    <button
      onClick={onSelect}
      className={`card group text-left focus:outline-none ${selected ? "card-selected" : ""}`}
    >
      <div className="card-well aspect-[2/3] w-full">
        <GameArt
          game={game}
          artId={artId}
          className="absolute inset-0 h-full w-full transition-transform duration-500 group-hover:scale-[1.06]"
        />
        <div className="absolute inset-0 bg-gradient-to-t from-black/95 via-black/40 to-black/5" />
        {detecting && <div className="absolute inset-0 shimmer" />}
        {offline && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/70">
            <Pill tone="amber">Drive Offline</Pill>
          </div>
        )}

        <div className="absolute left-2 top-2 flex flex-wrap gap-1">
          <Pill>{STORE_LABEL[game.store] ?? game.store}</Pill>
          {installed && <Pill tone="green">Modded</Pill>}
        </div>

        <div className="absolute inset-x-0 bottom-0 p-2.5">
          <div className="line-clamp-2 text-[13px] font-bold leading-snug drop-shadow-md">{game.name}</div>
          <div className="mt-1.5 flex flex-wrap gap-1">
            {(() => {
              const badges: { key: string; tone: string; label: string }[] = [];
              for (const a of detection?.api.slice(0, 1) ?? []) {
                badges.push({ key: `api-${a}`, tone: "cyan", label: API_LABEL[a] ?? a });
              }
              for (const u of detection?.upscalers ?? []) {
                badges.push({ key: `up-${u}`, tone: "violet", label: TECH_LABEL[u] ?? u });
              }
              for (const f of detection?.framegen ?? []) {
                badges.push({ key: `fg-${f}`, tone: "green", label: TECH_LABEL[f] ?? f });
              }
              if (noUpscaler) {
                badges.push({ key: "no-upscaler", tone: "amber", label: "No Upscaler" });
              }
              const shown = badges.slice(0, 3);
              const extra = badges.length - shown.length;
              return (
                <>
                  {shown.map((b) => (
                    <Pill key={b.key} tone={b.tone}>
                      {b.label}
                    </Pill>
                  ))}
                  {extra > 0 && <Pill>+{extra}</Pill>}
                  {warn && <Pill tone="amber">!</Pill>}
                </>
              );
            })()}
          </div>
        </div>
      </div>
    </button>
  );
}
