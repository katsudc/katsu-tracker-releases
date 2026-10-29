import type { StateStorage } from "zustand/middleware";
// Keeps a bounded original copy if a legacy/user-generated save cannot be decoded.
// No migration path removes the source value or touches another application's data.
const blockedWrites=new Set<string>();
export const suiteStorage: StateStorage = {
  getItem: name => {
    try {
      const text = localStorage.getItem(name);
      if (text === null) return null;
      try { JSON.parse(text); return text; }
      catch { try { if (!localStorage.getItem(name + ".recovery")) localStorage.setItem(name + ".recovery", text); } catch { blockedWrites.add(name); } return null; }
    } catch { return null; }
  },
  setItem: (name, text) => { if(blockedWrites.has(name))throw Error("storage_recovery_unavailable");localStorage.setItem(name, text); },
  removeItem: name => { blockedWrites.delete(name);localStorage.removeItem(name); },
};
export const validId = (v: unknown): v is string => typeof v === "string" && /^[\w-]{1,128}$/.test(v);
export const accountId = (v: unknown): v is string => typeof v === "string" && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(v);
export const textValue = (v: unknown, max: number) => typeof v === "string" && v.length <= max;
export const dateNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v) && v > 0;
export const recordValue = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
