import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { accountId, suiteStorage } from "./storage";
export interface SessionGoal { rr: number | null; matches: number | null; minutes: number | null; losses: number | null; notify: boolean }
export const EMPTY_GOAL: SessionGoal = { rr: null, matches: null, minutes: null, losses: null, notify: false };
const limit = (v: unknown, max: number): boolean => v === null || (typeof v === "number" && Number.isInteger(v) && v > 0 && v <= max);
export function validGoal(v: SessionGoal): boolean { return !!v && limit(v.rr, 1000) && limit(v.matches, 100) && limit(v.minutes, 1440) && limit(v.losses, 20) && typeof v.notify === "boolean"; }
interface Goals { byAccount: Record<string, SessionGoal>; save: (account: string, goal: SessionGoal) => boolean; clear: () => void }
export const useGoalStore = create<Goals>()(persist((set) => ({
  byAccount: {}, save: (account, goal) => {
    if (!accountId(account) || !validGoal(goal)) return false;
    set(s => ({ byAccount: Object.fromEntries([...Object.entries(s.byAccount).filter(([id]) => id !== account).slice(-9), [account, { ...goal }]]) })); return true;
  }, clear: () => set({ byAccount: {} }),
}), { name: "katsu-session-goals-v1", storage: createJSONStorage(() => suiteStorage), partialize: s => ({ byAccount: s.byAccount }),
  merge: (saved, current) => {
    const v = (saved as Partial<Goals> | null)?.byAccount;
    return { ...current, byAccount: v && typeof v === "object" ? Object.fromEntries(Object.entries(v).filter(([id, goal]) => accountId(id) && validGoal(goal)).slice(-10)) : {} };
  },
}));
