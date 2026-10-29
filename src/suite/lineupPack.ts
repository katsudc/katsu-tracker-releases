import { cleanLibrary, validLineup, LIBRARY_LIMIT, type SavedLineup } from "../features/libraryData";
export const PACK_LIMIT = 1024 * 1024;
export interface LineupPack { format: "katsu-lineup-pack"; version: 1; name: string; createdAt: string; items: SavedLineup[] }
export type DuplicatePolicy = "skip" | "replace" | "both";
export function parseLineupPack(text: string): LineupPack {
  if (new TextEncoder().encode(text).length > PACK_LIMIT) throw Error("invalid");
  const v = JSON.parse(text.replace(/^\uFEFF/, ""));
  if (!v || v.format !== "katsu-lineup-pack" || v.version !== 1 || typeof v.name !== "string" || !v.name.trim() || v.name.length > 100 || typeof v.createdAt !== "string" || !Number.isFinite(Date.parse(v.createdAt)) || !Array.isArray(v.items) || v.items.length > LIBRARY_LIMIT || !v.items.every(validLineup) || new Set(v.items.map((x: SavedLineup) => x.id)).size !== v.items.length) throw Error("invalid");
  return { format: "katsu-lineup-pack", version: 1, name: v.name, createdAt: v.createdAt, items: cleanLibrary(v.items) };
}
export function sameLineup(a: SavedLineup, b: SavedLineup) { return a.id === b.id || (a.url === b.url && a.seconds === b.seconds && a.agent === b.agent && a.map === b.map && a.side === b.side); }
export function mergePack(current: SavedLineup[], incoming: SavedLineup[], policy: DuplicatePolicy): SavedLineup[] {
  const result = [...current];
  for (const item of incoming) {
    const index = result.findIndex(v => sameLineup(v, item));
    if (index < 0) result.push(item);
    else if (policy === "replace") result[index] = { ...item, id: result[index].id };
    else if (policy === "both") result.push({ ...item, id: crypto.randomUUID() });
  }
  if (result.length > LIBRARY_LIMIT) throw Error("capacity");
  return cleanLibrary(result);
}
