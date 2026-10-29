import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface StreamPreferences {
  enabled: boolean;
  hideNames: boolean;
  quietNotifications: boolean;
  hideMusic: boolean;
}
export const DEFAULT_STREAM: StreamPreferences = {
  enabled: false, hideNames: true, quietNotifications: true, hideMusic: true,
};
interface PersonalStore {
  stream: StreamPreferences;
  setupComplete: boolean;
  sessionStarts: Record<string, number>;
  setStream: (value: Partial<StreamPreferences>) => void;
  completeSetup: () => void;
  startSession: (account: string) => void;
}
export function validStream(value: unknown): value is StreamPreferences {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return ["enabled", "hideNames", "quietNotifications", "hideMusic"].every(k => typeof v[k] === "boolean");
}
export const usePersonalStore = create<PersonalStore>()(persist(
  (set) => ({
    stream: DEFAULT_STREAM, setupComplete: false, sessionStarts: {},
    setStream: value => set(s => ({ stream: { ...s.stream, ...value } })),
    completeSetup: () => set({ setupComplete: true }),
    startSession: account => set(s => ({
      sessionStarts: Object.fromEntries([...Object.entries(s.sessionStarts).filter(([key]) => key !== account).slice(-9), [account, Date.now()]]),
    })),
  }),
  {
    name: "katsu-personal-tools-v1",
    partialize: s => ({ stream: s.stream, setupComplete: s.setupComplete, sessionStarts: s.sessionStarts }),
    merge: (saved, current) => {
      const v = (saved ?? {}) as Partial<PersonalStore>;
      const starts = v.sessionStarts && typeof v.sessionStarts === "object" ? Object.entries(v.sessionStarts).filter(
        ([key, n]) => key.length <= 128 && typeof n === "number" && Number.isFinite(n) && n > 0 && n <= Date.now(),
      ).slice(-10) : [];
      return { ...current, stream: validStream(v.stream) ? v.stream : DEFAULT_STREAM,
        setupComplete: v.setupComplete === true, sessionStarts: Object.fromEntries(starts) };
    },
  },
));
