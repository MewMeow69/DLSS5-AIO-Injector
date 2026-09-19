import type { ReactNode } from "react";

const TONES: Record<string, string> = {
  neutral: "text-deck-text/75 bg-white/[0.06]",
  green: "text-deck-green bg-[rgba(142,217,74,0.14)]",
  cyan: "text-deck-cyan bg-[rgba(76,199,232,0.14)]",
  violet: "text-deck-violet bg-[rgba(167,139,250,0.14)]",
  amber: "text-deck-amber bg-[rgba(232,176,75,0.14)]",
  rose: "text-deck-rose bg-[rgba(224,82,109,0.14)]",
  accent: "text-deck-accent bg-[rgba(107,124,255,0.16)]",
};

export function Pill({
  children,
  tone = "neutral",
  className = "",
}: {
  children: ReactNode;
  tone?: keyof typeof TONES | string;
  className?: string;
}) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-md px-1.5 py-[3px] text-[10px] font-bold uppercase tracking-wide ${TONES[tone] ?? TONES.neutral} ${className}`}
    >
      {children}
    </span>
  );
}

export function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4 border-b border-white/[0.045] py-1.5 last:border-0">
      <span className="text-[10.5px] font-bold uppercase tracking-wider text-deck-muted">{label}</span>
      <span className="text-right text-[12.5px]">{children}</span>
    </div>
  );
}

export function Check({ on }: { on: boolean }) {
  return (
    <span
      className="inline-block h-2.5 w-2.5 rounded-full"
      style={{ background: on ? "#8ed94a" : "#33333f" }}
    />
  );
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <div className="mb-1.5 text-[10.5px] font-bold uppercase tracking-[0.14em] text-deck-muted">{children}</div>
  );
}

export function Toggle({ on, onChange, disabled }: { on: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      onClick={() => !disabled && onChange(!on)}
      disabled={disabled}
      className="relative h-[18px] w-[32px] shrink-0 rounded-full transition-colors disabled:opacity-40"
      style={{
        background: on ? "#6b7cff" : "#33333f",
        boxShadow: "inset 0 0 0 1px rgba(255,255,255,0.08)",
      }}
    >
      <span
        className="absolute top-[2px] h-[14px] w-[14px] rounded-full bg-white transition-all"
        style={{ left: on ? 16 : 2, boxShadow: "0 1px 2px rgba(0,0,0,0.5)" }}
      />
    </button>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-white/[0.045] py-1.5 last:border-0">
      <div className="min-w-0">
        <div className="text-[12px] font-medium">{label}</div>
        {hint ? <div className="text-[10.5px] text-deck-muted">{hint}</div> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export function Range({
  value,
  min,
  max,
  step,
  onChange,
}: {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="h-1 w-[104px] cursor-pointer appearance-none rounded-full bg-white/15 accent-deck-accent"
      />
      <span className="w-[38px] text-right text-[11.5px] tabular-nums text-deck-muted">{value.toFixed(2)}</span>
    </div>
  );
}

export function Select({
  value,
  options,
  onChange,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="inset cursor-pointer px-2 py-1 text-[11.5px] outline-none"
    >
      {options.map((o) => (
        <option key={o.value} value={o.value} className="bg-[#16161f]">
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function Progress({ value, max }: { value: number; max: number }) {
  const pct = max > 0 ? Math.min(100, Math.round((value / max) * 100)) : 0;
  return (
    <div className="flex items-center gap-2">
      <div className="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full" style={{ background: "#0e0e14" }}>
        {max > 0 ? (
          <div
            className="h-full rounded-full transition-[width] duration-200"
            style={{ width: `${pct}%`, background: "linear-gradient(90deg, #3f8fd6, #7cc3f2)" }}
          />
        ) : (
          <div className="shimmer h-full w-full" />
        )}
      </div>
      <span className="w-[92px] shrink-0 text-right text-[10.5px] tabular-nums text-deck-muted">
        {max > 0 ? `${pct}% · ${(max / 1024 ** 2).toFixed(0)} MB` : ""}
      </span>
    </div>
  );
}

export function Popup({
  open,
  title,
  onClose,
  children,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6" onClick={onClose}>
      <div
        className="tile pop-in max-h-[70vh] w-[500px] overflow-y-auto p-5"
        onClick={(e) => e.stopPropagation()}
        style={{ background: "#191922" }}
      >
        <div className="flex items-start justify-between gap-4">
          <div className="text-[14px] font-bold">{title}</div>
          <button className="btn btn-sm" onClick={onClose}>
            Close
          </button>
        </div>
        <div className="mt-3 space-y-2 text-[12px] leading-relaxed text-deck-text/85">{children}</div>
      </div>
    </div>
  );
}
