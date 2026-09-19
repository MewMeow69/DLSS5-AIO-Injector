import type { ReactNode } from "react";

function I({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export const Plus = () => (
  <I>
    <path d="M12 5v14M5 12h14" />
  </I>
);

export const Folder = () => (
  <I>
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  </I>
);

export const FolderPlus = () => (
  <I>
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    <path d="M12 11v6M9 14h6" />
  </I>
);

export const Refresh = () => (
  <I>
    <path d="M20.5 12a8.5 8.5 0 1 1-2.6-6.1" />
    <path d="M20.5 4.5V9h-4.5" />
  </I>
);

export const Download = () => (
  <I>
    <path d="M12 3v12" />
    <path d="M7 11l5 5 5-5" />
    <path d="M4 20h16" />
  </I>
);

export const Upload = () => (
  <I>
    <path d="M12 21V9" />
    <path d="M7 13l5-5 5 5" />
    <path d="M4 4h16" />
  </I>
);

export const Play = () => (
  <I>
    <path d="M6 4.5l13 7.5-13 7.5z" />
  </I>
);

export const Undo = () => (
  <I>
    <path d="M9 14L4 9l5-5" />
    <path d="M4 9h10a6 6 0 0 1 0 12h-3" />
  </I>
);

export const Trash = () => (
  <I>
    <path d="M4 7h16" />
    <path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" />
    <path d="M6.5 7l1 13a1 1 0 0 0 1 1h7a1 1 0 0 0 1-1l1-13" />
  </I>
);

export const List = () => (
  <I>
    <path d="M8 6h13M8 12h13M8 18h13" />
    <path d="M3.5 6h.01M3.5 12h.01M3.5 18h.01" />
  </I>
);

export const Copy = () => (
  <I>
    <rect x="9" y="9" width="11" height="11" rx="2" />
    <path d="M5 15V6a2 2 0 0 1 2-2h9" />
  </I>
);

export const Close = () => (
  <I>
    <path d="M6 6l12 12M18 6L6 18" />
  </I>
);

export const Bolt = () => (
  <I>
    <path d="M13 2.5L4.5 14H11l-1 7.5L18.5 10H12z" />
  </I>
);

export const Check = () => (
  <I>
    <path d="M4.5 12.5l5 5 10-11" />
  </I>
);

export const Pin = () => (
  <I>
    <path d="M12 17v5" />
    <path d="M8 3h8l-1 6 3 3v2H6v-2l3-3z" />
  </I>
);
