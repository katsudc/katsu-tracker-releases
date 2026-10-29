import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { accountId, dateNumber, suiteStorage, textValue, validId } from "./storage";
export interface Moment { id: string; account: string; match: string; at: number; observedElapsed: number | null; round: number | null; note: string }
const valid = (v: Moment) => v && validId(v.id) && accountId(v.account) && validId(v.match) && dateNumber(v.at) &&
  (v.observedElapsed === null || (Number.isFinite(v.observedElapsed) && v.observedElapsed >= 0)) &&
  (v.round === null || (Number.isInteger(v.round) && v.round > 0 && v.round < 200)) && textValue(v.note, 3000);
interface MarkerData { moments: Moment[]; add: (v: Moment) => void; note: (id: string, note: string) => void; remove: (id: string) => void; clear: () => void }
export const useMarkerStore = create<MarkerData>()(persist(
  set => ({
    moments: [],
    add: v => { if (valid(v)) set(s => ({ moments: [...s.moments, v].slice(-500) })); },
    note: (id, note) => { if (textValue(note, 3000)) set(s => ({ moments: s.moments.map(v => v.id === id ? { ...v, note } : v) })); },
    remove: id => set(s => ({ moments: s.moments.filter(v => v.id !== id) })), clear: () => set({ moments: [] }),
  }),
  { name: "katsu-moments-v1", storage: createJSONStorage(() => suiteStorage), partialize: s => ({ moments: s.moments }),
    merge: (saved, current) => ({ ...current, moments: Array.isArray((saved as MarkerData)?.moments) ? (saved as MarkerData).moments.filter(valid).slice(-500) : [] }) },
));
