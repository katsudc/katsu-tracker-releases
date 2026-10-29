import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { accountId, dateNumber, suiteStorage, textValue, validId } from "./storage";
export interface PersonalNote { id: string; account: string | null; kind: "match" | "reference"; match: string | null; agent: string; map: string; body: string; modified: number }
export const validNote = (v: PersonalNote) => !!v && validId(v.id) && (v.account === null || accountId(v.account)) &&
  ["match", "reference"].includes(v.kind) && (v.match === null || validId(v.match)) && textValue(v.agent, 60) && textValue(v.map, 60) && textValue(v.body, 4000) && dateNumber(v.modified);
interface Notes { notes: PersonalNote[]; save: (v: PersonalNote) => boolean; remove: (id: string) => void; clear: () => void }
export const useNoteStore = create<Notes>()(persist(
  (set, get) => ({
    notes: [],
    save: v => {
      if (!validNote(v) || (!get().notes.some(n => n.id === v.id) && get().notes.length >= 500)) return false;
      set(s => ({ notes: [v, ...s.notes.filter(n => n.id !== v.id)] })); return true;
    },
    remove: id => set(s => ({ notes: s.notes.filter(n => n.id !== id) })), clear: () => set({ notes: [] }),
  }),
  { name: "katsu-notes-v1", storage: createJSONStorage(() => suiteStorage), partialize: s => ({ notes: s.notes }),
    merge: (saved, current) => ({ ...current, notes: Array.isArray((saved as Notes)?.notes) ? (saved as Notes).notes.filter(validNote).slice(0, 500) : [] }) },
));
