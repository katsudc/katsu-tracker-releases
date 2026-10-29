import { create } from "zustand";
import { invoke } from "./ipc";
import { listen } from "@tauri-apps/api/event";
export interface RuntimeStatus {
  mode: "loading" | "normal" | "safe" | "pending" | "unavailable";
  previous_unclean: boolean; disabled_modules: string[]; failed_modules: string[]; started_minimized: boolean;
}
let bootstrap: Promise<void> | null = null;
export const useRuntimeStore = create<RuntimeStatus & { start: () => Promise<void>; choose: (safe: boolean) => Promise<void> }>(set => ({
  mode: "loading", previous_unclean: false, disabled_modules: [], failed_modules: [], started_minimized: false,
  start: () => {
    if (bootstrap) return bootstrap;
    bootstrap = (async () => {
      let received = false;
      try {
        await listen<RuntimeStatus>("runtime-status", e => { received = true; if (e.payload) set(e.payload); });
        const value = await invoke<RuntimeStatus>("runtime_status");
        if (value && !received) set(value);
        else if (!value && !received) set({ mode: "unavailable" });
      } catch { set({ mode: "unavailable" }); }
    })();
    return bootstrap;
  },
  choose: async safe => { const value = await invoke<RuntimeStatus>("runtime_choose", { safe }); if (value) set(value); },
}));
