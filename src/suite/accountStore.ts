import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";
import { accountId, recordValue, suiteStorage } from "./storage";
export interface AgentPreferences { autoLockAgent: string | null; pausedAutoLockAgent: string | null; mapAgentPreferences: Record<string, string>; delay: number }
interface Account { id: string; name: string | null; seen: number; agents: AgentPreferences | null }
const validAgent = (v: unknown) => v === null || (typeof v === "string" && /^[A-Za-z0-9/ -]{1,48}$/.test(v));
function validPreferences(v: unknown): v is AgentPreferences { return recordValue(v) && validAgent(v.autoLockAgent) && validAgent(v.pausedAutoLockAgent) && recordValue(v.mapAgentPreferences) && Object.keys(v.mapAgentPreferences).length <= 64 && Object.entries(v.mapAgentPreferences).every(([k,a]) => /^[A-Za-z0-9 -]{1,48}$/.test(k) && typeof a === "string" && validAgent(a)) && typeof v.delay === "number" && Number.isInteger(v.delay) && v.delay >= 1 && v.delay <= 10; }
export const useAccountStore = create<{ accounts: Account[]; separate: boolean; active: string | null; setActive: (id: string | null) => void; observe: (id: string, name: string | null) => void; save: (id: string, prefs: AgentPreferences) => void; setSeparate: (v: boolean) => void; clear: () => void }>()(persist((set) => ({
  accounts: [], separate: false, active: null,
  setActive: active => set({ active }),
  observe: (id, name) => { if (!accountId(id)) return; set(s => ({ accounts: [{ id, name: name?.slice(0,100) ?? s.accounts.find(a => a.id === id)?.name ?? null, seen: Date.now(), agents: s.accounts.find(a => a.id === id)?.agents ?? null }, ...s.accounts.filter(a => a.id !== id)].slice(0,10) })); },
  save: (id, agents) => { if (validPreferences(agents)) set(s => ({ accounts: s.accounts.map(a => a.id === id ? { ...a, agents } : a) })); },
  setSeparate: separate => set({ separate }), clear: () => set({ accounts: [], separate: false, active: null }),
}), { name: "katsu-accounts-v1", storage: createJSONStorage(() => suiteStorage), partialize: s => ({ accounts:s.accounts, separate:s.separate }),
  merge: (saved, current) => { const v = recordValue(saved) ? saved : {}; return { ...current, separate: v.separate === true, accounts: Array.isArray(v.accounts) ? v.accounts.filter(a => recordValue(a) && accountId(a.id) && (a.name === null || (typeof a.name === "string" && a.name.length <= 100)) && typeof a.seen === "number" && Number.isFinite(a.seen) && (a.agents === null || validPreferences(a.agents))).slice(0,10) as Account[] : [] }; },
}));
