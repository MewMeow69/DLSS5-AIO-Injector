import type { GpuInfo } from "../lib/types";
import { RUNTIME_LABEL } from "../lib/api";
import { Pill } from "./ui";

export type View = "library" | "components" | "guide" | "settings";

function NavItem({
  label,
  active,
  soon,
  badge,
  onClick,
}: {
  label: string;
  active?: boolean;
  soon?: boolean;
  badge?: number;
  onClick?: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={soon}
      className={`relative flex w-full items-center justify-between rounded-xl px-3 py-2 text-[13px] font-bold transition-all duration-150 ${
        soon ? "cursor-default text-deck-muted/60" : active ? "text-deck-text" : "text-deck-muted hover:text-deck-text"
      }`}
      style={
        active
          ? { background: "#262632", boxShadow: "inset 0 0 0 1px rgba(255,255,255,0.08)" }
          : { background: "transparent" }
      }
    >
      <span>{label}</span>
      {badge ? (
        <span
          className="rounded-md px-1.5 py-0.5 text-[10px] font-bold text-black"
          style={{ background: "#6b7cff" }}
        >
          {badge}
        </span>
      ) : soon ? (
        <span className="text-[9.5px] font-medium uppercase tracking-[0.16em] text-deck-muted/60">Soon</span>
      ) : null}
    </button>
  );
}

export function Sidebar({
  gpu,
  view,
  onView,
  updateCount,
}: {
  gpu: GpuInfo | null;
  view: View;
  onView: (v: View) => void;
  updateCount: number;
}) {
  return (
    <aside className="flex w-[224px] shrink-0 flex-col border-r border-deck-line bg-black/25 px-3 py-4">
      <nav className="space-y-2">
        <NavItem label="Library" active={view === "library"} onClick={() => onView("library")} />
        <NavItem
          label="Components"
          active={view === "components"}
          onClick={() => onView("components")}
          badge={updateCount}
        />
        <NavItem label="How to Use" active={view === "guide"} onClick={() => onView("guide")} />
        <NavItem label="Settings" active={view === "settings"} onClick={() => onView("settings")} />
      </nav>

      <div className="mt-auto tile p-3">
        <div className="text-[10px] font-bold uppercase tracking-[0.14em] text-deck-muted">GPU</div>
        <div className="mt-1.5 text-[12px] font-bold leading-snug tracking-normal">{gpu?.name ?? "Detecting…"}</div>
        {gpu ? (
          <>
            <div className="mt-1 text-[11px] text-deck-muted">
              {gpu.family}
              {gpu.vramBytes ? ` · ${Math.round(gpu.vramBytes / 1024 ** 3)} GB` : ""}
            </div>
            <div className="mt-2">
              <Pill tone={gpu.nrSupported ? "green" : "amber"}>
                {RUNTIME_LABEL[gpu.nrRuntime] ?? gpu.nrRuntime}
              </Pill>
            </div>
          </>
        ) : null}
      </div>
    </aside>
  );
}
