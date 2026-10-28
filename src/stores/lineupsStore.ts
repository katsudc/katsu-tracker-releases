import { create } from "zustand";
import { persist } from "zustand/middleware";

export type LineupSide = "attack" | "defense";
export interface LineupFilters { agent: string; map: string; side: LineupSide }
export const LINEUP_MAPS = ["Abyss", "Ascent", "Bind", "Breeze", "Corrode", "District", "Drift", "Fracture", "Glitch", "Haven", "Icebox", "Kasbah", "Lotus", "Pearl", "Piazza", "Skirmish A", "Skirmish B", "Skirmish C", "Skirmish D", "Skirmish E", "Split", "Summit", "Sunset"];
export function lineupSlug(value: string) {
  return value.trim().toLowerCase().replace(/\//g, "").replace(/\s+/g, "-");
}
function validSlug(value: unknown): value is string {
  return typeof value === "string" && /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value) && value.length <= 48;
}
export function lineupUrl(filters: LineupFilters) {
  if (!validSlug(filters.agent) || !validSlug(filters.map) || !["attack", "defense"].includes(filters.side)) return null;
  const url = new URL("https://tracker.gg/valorant/guides/clips");
  url.searchParams.set("agent", filters.agent);
  url.searchParams.set("map", filters.map);
  url.searchParams.set("objective", filters.side);
  url.searchParams.set("sort", "ranking");
  return url.toString();
}
interface LineupsStore extends LineupFilters {
  setFilters: (filters: Partial<LineupFilters>) => void;
}
export const useLineupsStore = create<LineupsStore>()(persist(
  set => ({ agent: "brimstone", map: "haven", side: "attack", setFilters: filters => set(filters) }),
  {
    name: "katsu-lineups-v1",
    partialize: state => ({ agent: state.agent, map: state.map, side: state.side }),
    merge: (saved, current) => {
      const value = (saved ?? {}) as Partial<LineupFilters>;
      return { ...current, agent: validSlug(value.agent) ? value.agent : current.agent,
        map: validSlug(value.map) ? value.map : current.map,
        side: value.side === "defense" ? "defense" : "attack" };
    },
  },
));
