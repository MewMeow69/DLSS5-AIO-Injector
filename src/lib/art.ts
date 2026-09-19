import { steamSearchAppId } from "./api";
import type { Game } from "./types";

const CACHE_KEY = "neurodeck.art.v1";

type ArtCache = Record<string, string>;

function loadCache(): ArtCache {
  try {
    return JSON.parse(localStorage.getItem(CACHE_KEY) ?? "{}") as ArtCache;
  } catch {
    return {};
  }
}

let cache = loadCache();

function saveCache() {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(cache));
  } catch {
    /* storage full or unavailable; art just re-resolves next launch */
  }
}

export function steamPortrait(appId: string) {
  return `https://cdn.cloudflare.steamstatic.com/steam/apps/${appId}/library_600x900.jpg`;
}

export function steamHeader(appId: string) {
  return `https://cdn.cloudflare.steamstatic.com/steam/apps/${appId}/header.jpg`;
}

export function cachedArtId(game: Game): string | null {
  if (game.store === "steam" && game.appId) return game.appId;
  return cache[game.id] ?? null;
}

function cleanName(name: string) {
  return name
    .replace(/[™®©]/g, "")
    .replace(/\s*[-–—]\s*(definitive|ultimate|deluxe|complete|enhanced|gold|premium)\s*edition.*$/i, "")
    .replace(/\s*\((?:[^)]*)\)\s*$/, "")
    .trim();
}

export async function resolveArtId(game: Game): Promise<string | null> {
  const hit = cachedArtId(game);
  if (hit) return hit;
  if (!game.name) return null;
  try {
    const id = await steamSearchAppId(cleanName(game.name));
    if (id) {
      cache[game.id] = id;
      saveCache();
      return id;
    }
  } catch {
    return null;
  }
  return null;
}
