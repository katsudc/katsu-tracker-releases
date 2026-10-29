import type { LineupFilters } from "../stores/lineupsStore";

export const LIBRARY_LIMIT = 200;
export interface SavedLineup extends LineupFilters {
  id: string; title: string; url: string; seconds: number; notes: string;
  favorite: boolean; createdAt: number;
}
const hosts = ["youtube.com", "www.youtube.com", "m.youtube.com", "youtu.be", "tracker.gg", "www.tracker.gg", "twitch.tv", "www.twitch.tv", "clips.twitch.tv", "medal.tv", "www.medal.tv"];
export function videoUrl(value: string): string | null {
  try {
    const url = new URL(value.trim());
    if (url.protocol !== "https:" || !hosts.includes(url.hostname) || url.username || url.password || url.port) return null;
    if (url.hostname.endsWith("youtube.com")) {
      if (url.pathname === "/watch") {
        if (!/^[\w-]{11}$/.test(url.searchParams.get("v") ?? "")) return null;
      } else if (!/^\/(shorts|live)\/[\w-]{11}\/?$/.test(url.pathname)) return null;
    } else if (url.hostname === "youtu.be" && !/^\/[\w-]{11}\/?$/.test(url.pathname)) return null;
    else if (url.hostname.endsWith("tracker.gg") && !url.pathname.startsWith("/valorant/guides/")) return null;
    else if ((url.hostname.endsWith("twitch.tv") || url.hostname.endsWith("medal.tv")) && url.pathname === "/") return null;
    url.hash = "";
    if (url.toString().length > 2048) return null;
    return url.toString();
  } catch { return null; }
}
export function parseTime(value: string): number | null {
  if (!value.trim()) return 0;
  if (!/^\d{1,4}:[0-5]\d$/.test(value.trim())) return null;
  const [m, s] = value.trim().split(":").map(Number);
  const seconds = m * 60 + s;
  return seconds <= 86400 ? seconds : null;
}
export const timeText = (seconds: number) => Math.floor(seconds / 60) + ":" + String(seconds % 60).padStart(2, "0");
export function playbackUrl(item: SavedLineup): string | null {
  const canonical = videoUrl(item.url);
  if (!canonical) return null;
  const url = new URL(canonical);
  if (url.hostname === "youtu.be" || url.hostname.endsWith("youtube.com")) {
    url.searchParams.delete("start");
    url.searchParams.set("t", String(item.seconds) + "s");
  }
  return url.toString();
}
export function validLineup(value: unknown): value is SavedLineup {
  if (!value || typeof value !== "object") return false;
  const v = value as SavedLineup;
  return typeof v.id === "string" && /^[\w-]{1,80}$/.test(v.id) &&
    typeof v.title === "string" && v.title.trim().length > 0 && v.title.length <= 100 &&
    typeof v.url === "string" && videoUrl(v.url) !== null &&
    typeof v.notes === "string" && v.notes.length <= 3000 &&
    [v.agent, v.map].every(s => typeof s === "string" && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(s) && s.length <= 48) &&
    ["attack", "defense"].includes(v.side) && Number.isInteger(v.seconds) && v.seconds >= 0 && v.seconds <= 86400 &&
    typeof v.favorite === "boolean" && Number.isFinite(v.createdAt) && v.createdAt > 0;
}
export function cleanLibrary(value: unknown): SavedLineup[] {
  if (!Array.isArray(value)) return [];
  const unique = new Map<string, SavedLineup>();
  for (const item of value.slice(0, LIBRARY_LIMIT * 2)) {
    if (validLineup(item) && !unique.has(item.id)) unique.set(item.id, {
      id: item.id, title: item.title, url: videoUrl(item.url)!, agent: item.agent, map: item.map,
      side: item.side, seconds: item.seconds, notes: item.notes, favorite: item.favorite, createdAt: item.createdAt,
    });
    if (unique.size === LIBRARY_LIMIT) break;
  }
  return [...unique.values()];
}
