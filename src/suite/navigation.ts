import { create } from "zustand";
export type ToolTab = "home" | "lineups" | "crosshairs" | "sensitivity" | "notes" | "diagnostics" | "privacy";
export type StatTab = "overview" | "coach" | "agents" | "maps" | "rank" | "recap";
export const useSuiteNavigation = create<{
  view: string; tool: ToolTab; stats: StatTab; focus: string | null;
  go: (view: string, tool?: ToolTab, focus?: string | null) => void;
  setTool: (tool: ToolTab) => void; setStats: (stats: StatTab) => void; consumeFocus: () => void;
}>(set => ({
  view: "match", tool: "home", stats: "overview", focus: null,
  go: (view, tool, focus = null) => set(s => ({ view, tool: tool ?? s.tool, focus })),
  setTool: tool => set({ tool }), setStats: stats => set({ stats }), consumeFocus: () => set({ focus: null }),
}));
