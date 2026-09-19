import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

function MinIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.1">
      <path d="M0 5.5h10" />
    </svg>
  );
}

function MaxIcon({ maximized }: { maximized: boolean }) {
  if (maximized) {
    return (
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.1">
        <rect x="0.5" y="2.5" width="7" height="7" />
        <path d="M2.5 2.5V0.5h7v7h-2" />
      </svg>
    );
  }
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.1">
      <rect x="0.5" y="0.5" width="9" height="9" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.1">
      <path d="M0.5 0.5l9 9M9.5 0.5l-9 9" />
    </svg>
  );
}

export function TitleBar({ subtitle }: { subtitle: string }) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    appWindow.isMaximized().then(setMaximized).catch(() => {});
    const un = appWindow.onResized(() => {
      appWindow.isMaximized().then(setMaximized).catch(() => {});
    });
    return () => {
      void un.then((f) => f());
    };
  }, []);

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="grid h-6 w-6 place-items-center rounded-[7px] text-black" style={{ background: "#6b7cff" }}>
        <svg viewBox="0 0 24 24" className="h-3.5 w-3.5" fill="none" stroke="currentColor" strokeWidth="2.6">
          <circle cx="5" cy="12" r="2.2" />
          <circle cx="19" cy="6" r="2.2" />
          <circle cx="19" cy="18" r="2.2" />
          <path d="M7 11l10-4M7 13l10 4" />
        </svg>
      </div>
      <span className="text-[12.5px] font-bold tracking-normal" data-tauri-drag-region>
        DLSS5 AIO Injector
      </span>
      <span className="text-[11.5px] text-deck-muted" data-tauri-drag-region>
        {subtitle}
      </span>

      <div className="ml-auto flex h-full items-stretch">
        <button className="win-btn" onClick={() => void appWindow.minimize()} title="Minimize">
          <MinIcon />
        </button>
        <button className="win-btn" onClick={() => void appWindow.toggleMaximize()} title={maximized ? "Restore" : "Maximize"}>
          <MaxIcon maximized={maximized} />
        </button>
        <button className="win-btn win-btn-close" onClick={() => void appWindow.close()} title="Close">
          <CloseIcon />
        </button>
      </div>
    </div>
  );
}
