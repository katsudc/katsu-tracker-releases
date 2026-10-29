import type { Theme, WindowStyle } from "../stores/settingsStore";
import type { SpikeAppearance } from "../components/spikeAppearance";
import type { StreamPreferences } from "./personalStore";
export interface PortablePreferences {
  theme: Theme; locale: "en" | "tr"; hotkey: string; windowStyle: WindowStyle;
  autoLockDelaySeconds: number; discordRpcEnabled: boolean; minimizeToTray: boolean; closeToTray?: boolean; startMinimized?: boolean;
  autoLockAgent: string | null; pausedAutoLockAgent: string | null; mapAgentPreferences: Record<string, string>;
  spikeEnabled: boolean; appearance: SpikeAppearance; stream: StreamPreferences;
}
export interface SettingsFile { format: "katsu-tracker-settings"; version: 1; createdAt: string; preferences: PortablePreferences }
export const SETTINGS_FILE_LIMIT = 128 * 1024;
const object = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === "object" && !Array.isArray(v);
const finite = (v: unknown, min: number, max: number): v is number => typeof v === "number" && Number.isFinite(v) && v >= min && v <= max;
const agent = (v: unknown): v is string | null => v === null || (typeof v === "string" && /^[A-Za-z0-9/ -]{1,48}$/.test(v));
export function parseSettingsFile(text: string): SettingsFile {
  if (new TextEncoder().encode(text).length > SETTINGS_FILE_LIMIT) throw new Error("invalid");
  const doc: unknown = JSON.parse(text.replace(/^\uFEFF/, ""));
  if (!object(doc) || doc.format !== "katsu-tracker-settings" || doc.version !== 1 || typeof doc.createdAt !== "string" || !Number.isFinite(Date.parse(doc.createdAt)) || !object(doc.preferences)) throw new Error("invalid");
  const p = doc.preferences, a = p.appearance, s = p.stream;
  if (!["mint", "blue", "violet", "rose", "amber"].includes(String(p.theme)) || !["en", "tr"].includes(String(p.locale)) ||
    typeof p.hotkey !== "string" || p.hotkey.length > 80 || !/^[A-Za-z0-9]+(?:\+[A-Za-z0-9]+){0,4}$/.test(p.hotkey) ||
    !["free", "docked"].includes(String(p.windowStyle)) || !finite(p.autoLockDelaySeconds, 1, 10) || !Number.isInteger(p.autoLockDelaySeconds) ||
    ![p.discordRpcEnabled, p.minimizeToTray, p.spikeEnabled].every(v => typeof v === "boolean") ||
    ((p.closeToTray !== undefined && typeof p.closeToTray !== "boolean") || (p.startMinimized !== undefined && typeof p.startMinimized !== "boolean")) ||
    !agent(p.autoLockAgent) || !agent(p.pausedAutoLockAgent) || !object(p.mapAgentPreferences) || Object.keys(p.mapAgentPreferences).length > 64 ||
    !Object.entries(p.mapAgentPreferences).every(([key, value]) => /^[A-Za-z0-9 -]{1,48}$/.test(key) && typeof value === "string" && agent(value)) ||
    !object(a) || !finite(a.font_size, 14, 84) || !finite(a.background_opacity, 0, 1) ||
    !(a.position === null || (object(a.position) && finite(a.position.x, 0, 1) && finite(a.position.y, 0, 1))) ||
    !object(a.game_bar) || !finite(a.game_bar.font_size, 14, 84) || !finite(a.game_bar.background_opacity, 0, 1) ||
    !object(s) || !["enabled", "hideNames", "quietNotifications", "hideMusic"].every(key => typeof s[key] === "boolean")) throw new Error("invalid");
  // Construct a fresh allowlisted object. Never hydrate arbitrary store fields or credentials.
  return {
    format: "katsu-tracker-settings", version: 1, createdAt: doc.createdAt,
    preferences: {
      theme: p.theme as Theme, locale: p.locale as "en" | "tr", hotkey: p.hotkey, windowStyle: p.windowStyle as WindowStyle,
      autoLockDelaySeconds: p.autoLockDelaySeconds, discordRpcEnabled: p.discordRpcEnabled as boolean, minimizeToTray: p.minimizeToTray as boolean, closeToTray: p.closeToTray === true, startMinimized: p.startMinimized === true,
      autoLockAgent: p.autoLockAgent as string | null, pausedAutoLockAgent: p.pausedAutoLockAgent as string | null,
      mapAgentPreferences: Object.fromEntries(Object.entries(p.mapAgentPreferences)) as Record<string, string>, spikeEnabled: p.spikeEnabled as boolean,
      appearance: { position: a.position === null ? null : { x: a.position.x as number, y: a.position.y as number },
        font_size: a.font_size, background_opacity: a.background_opacity,
        game_bar: { font_size: a.game_bar.font_size, background_opacity: a.game_bar.background_opacity } },
      stream: { enabled: s.enabled as boolean, hideNames: s.hideNames as boolean, quietNotifications: s.quietNotifications as boolean, hideMusic: s.hideMusic as boolean },
    },
  };
}
export function settingsFile(preferences: PortablePreferences): SettingsFile {
  return parseSettingsFile(JSON.stringify({ format: "katsu-tracker-settings", version: 1, createdAt: new Date().toISOString(), preferences }));
}
