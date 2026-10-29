import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { suiteStorage } from "../suite/storage";
import { cleanLibrary, LIBRARY_LIMIT, validLineup, type SavedLineup } from "./libraryData";

interface LibraryStore {
  items: SavedLineup[];
  save: (item: SavedLineup) => boolean;
  remove: (id: string) => void;
  favorite: (id: string) => void;
}
export const useLibraryStore = create<LibraryStore>()(persist(
  (set, get) => ({
    items: [],
    save: item => {
      if (!validLineup(item) || (get().items.length >= LIBRARY_LIMIT && !get().items.some(v => v.id === item.id))) return false;
      set(s => ({ items: cleanLibrary([item, ...s.items.filter(v => v.id !== item.id)]) }));
      return true;
    },
    remove: id => set(s => ({ items: s.items.filter(v => v.id !== id) })),
    favorite: id => set(s => ({ items: s.items.map(v => v.id === id ? { ...v, favorite: !v.favorite } : v) })),
  }),
  { name: "katsu-lineup-library-v1", storage: createJSONStorage(()=>suiteStorage), partialize: s => ({ items: s.items }),
    merge: (saved, current) => ({ ...current, items: cleanLibrary((saved as Partial<LibraryStore> | null)?.items) }) },
));
