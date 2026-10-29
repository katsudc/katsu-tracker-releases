import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { suiteStorage } from "./storage";
import { validCrosshair, validSensitivity, type CrosshairProfile, type SensitivityProfile } from "./profileData";
interface Profiles {
  crosshairs: CrosshairProfile[]; sensitivity: SensitivityProfile[];
  saveCrosshair: (p: CrosshairProfile) => boolean; saveSensitivity: (p: SensitivityProfile) => boolean;
  remove: (kind: "crosshairs" | "sensitivity", id: string) => void;
  favorite: (kind: "crosshairs" | "sensitivity", id: string) => void; clear: () => void;
}
export const useProfileTools = create<Profiles>()(persist((set, get) => ({
  crosshairs: [], sensitivity: [],
  saveCrosshair: p => { if (!validCrosshair(p) || (get().crosshairs.length >= 100 && !get().crosshairs.some(v => v.id === p.id))) return false; set(s => ({ crosshairs: [p, ...s.crosshairs.filter(v => v.id !== p.id)] })); return true; },
  saveSensitivity: p => { if (!validSensitivity(p) || (get().sensitivity.length >= 100 && !get().sensitivity.some(v => v.id === p.id))) return false; set(s => ({ sensitivity: [p, ...s.sensitivity.filter(v => v.id !== p.id)] })); return true; },
  remove: (kind, id) => set(s => kind === "crosshairs" ? { crosshairs: s.crosshairs.filter(v => v.id !== id) } : { sensitivity: s.sensitivity.filter(v => v.id !== id) }),
  favorite: (kind, id) => set(s => kind === "crosshairs" ? { crosshairs: s.crosshairs.map(v => v.id === id ? { ...v, favorite: !v.favorite } : v) } : { sensitivity: s.sensitivity.map(v => v.id === id ? { ...v, favorite: !v.favorite } : v) }),
  clear: () => set({ crosshairs: [], sensitivity: [] }),
}), {
  name: "katsu-player-tools-v1", storage: createJSONStorage(() => suiteStorage),
  partialize: s => ({ crosshairs: s.crosshairs, sensitivity: s.sensitivity }),
  merge: (saved, current) => { const v = (saved ?? {}) as Partial<Profiles>; return { ...current,
    crosshairs: Array.isArray(v.crosshairs) ? v.crosshairs.filter(validCrosshair).slice(0, 100) : [],
    sensitivity: Array.isArray(v.sensitivity) ? v.sensitivity.filter(validSensitivity).slice(0, 100) : [] }; },
}));
