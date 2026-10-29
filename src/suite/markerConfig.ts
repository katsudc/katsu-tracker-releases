import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { suiteStorage } from "./storage";
interface MarkerConfig { enabled: boolean; hotkey: string; set: (enabled: boolean, hotkey: string) => void }
export const validShortcut = (v: unknown): v is string => typeof v === "string" && v.length <= 60 && /^(?:(?:Ctrl|Control|Alt|Shift)\+)*(?:F(?:[1-9]|1[0-9]|2[0-4])|[A-Z0-9])$/.test(v);
export const useMarkerConfig = create<MarkerConfig>()(persist(
  set => ({ enabled: false, hotkey: "F8", set: (enabled, hotkey) => { if (validShortcut(hotkey)) set({ enabled, hotkey }); } }),
  { name: "katsu-marker-config-v1", storage: createJSONStorage(() => suiteStorage), partialize: s => ({ enabled: s.enabled, hotkey: s.hotkey }),
    merge: (saved, current) => { const v = (saved ?? {}) as Partial<MarkerConfig>; return { ...current, enabled: v.enabled === true, hotkey: validShortcut(v.hotkey) ? v.hotkey : "F8" }; } },
));
