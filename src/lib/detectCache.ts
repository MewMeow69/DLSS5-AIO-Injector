import type { Detection } from "./types";

const KEY = "neurodeck.detect.v1";
const TTL_MS = 6 * 60 * 60 * 1000;

type Entry = { t: number; d: Detection };
type Store = Record<string, Entry>;

function load(): Store {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "{}") as Store;
  } catch {
    return {};
  }
}

let store = load();
let dirty = false;

export function loadCached(dir: string): Detection | null {
  const e = store[dir.toLowerCase()];
  if (!e) return null;
  if (Date.now() - e.t > TTL_MS) return null;
  return e.d;
}

export function saveCached(dir: string, d: Detection) {
  store[dir.toLowerCase()] = { t: Date.now(), d };
  dirty = true;
  scheduleFlush();
}

let timer: number | undefined;
function scheduleFlush() {
  if (timer) return;
  timer = window.setTimeout(() => {
    timer = undefined;
    if (!dirty) return;
    dirty = false;
    try {
      localStorage.setItem(KEY, JSON.stringify(store));
    } catch {
      store = {};
    }
  }, 400);
}
