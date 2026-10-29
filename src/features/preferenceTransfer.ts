import { invoke } from "../suite/ipc";
import { useSettingsStore } from "../stores/settingsStore";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "../lib/i18n";
import { usePersonalStore } from "./personalStore";
import { settingsFile, type PortablePreferences, type SettingsFile } from "./preferenceData";
import { DEFAULT_APPEARANCE, type SpikeAppearance } from "../components/spikeAppearance";

export async function capturePreferences(): Promise<SettingsFile> {
  const s = useSettingsStore.getState(), g = useGameStore.getState();
  const [appearance, spike] = await Promise.all([invoke<SpikeAppearance>("get_spike_appearance"), invoke<{ enabled: boolean }>("spike_get_status")]);
  return settingsFile({
    theme: s.theme, locale: useI18n.getState().locale, hotkey: s.hotkey, windowStyle: s.windowStyle,
    autoLockDelaySeconds: s.autoLockDelaySeconds, discordRpcEnabled: s.discordRpcEnabled, minimizeToTray: s.minimizeToTray, closeToTray: s.closeToTray, startMinimized: s.startMinimized,
    autoLockAgent: g.autoLockAgent, pausedAutoLockAgent: g.pausedAutoLockAgent, mapAgentPreferences: { ...g.mapAgentPreferences },
    spikeEnabled: spike.enabled, appearance: { ...appearance, game_bar: appearance.game_bar ?? { font_size: DEFAULT_APPEARANCE.font_size, background_opacity: DEFAULT_APPEARANCE.background_opacity } },
    stream: { ...usePersonalStore.getState().stream },
  });
}
function nativeSteps(p: PortablePreferences): Array<[string, Record<string, unknown>]> {
  return [
    ["set_auto_lock", { agent: p.autoLockAgent }],
    ["set_map_preferences", { preferences: p.mapAgentPreferences }],
    ["set_auto_lock_delay", { seconds: p.autoLockDelaySeconds }],
    ["set_discord_rpc", { enabled: !p.stream.enabled && p.discordRpcEnabled }],
    ["set_spike_appearance", { appearance: p.appearance }],
    ["spike_set_enabled", { enabled: p.spikeEnabled }],
  ];
}
async function nativePreferences(p: PortablePreferences) {
  for (const [command, args] of nativeSteps(p)) await invoke(command, args);
}
export async function applyPreferences(file: SettingsFile): Promise<void> {
  if (["ingame", "pregame"].includes(useGameStore.getState().gameState.state)) throw new Error("busy_game");
  const old = await capturePreferences(), p = settingsFile(file.preferences).preferences;
  if (["ingame", "pregame"].includes(useGameStore.getState().gameState.state)) throw new Error("busy_game");
  try {
    await nativePreferences(p);
    if (!await useSettingsStore.getState().setHotkey(p.hotkey)) throw new Error("hotkey");
  } catch {
    const rollback = await Promise.allSettled([
      ...nativeSteps(old.preferences).map(([command, args]) => invoke(command, args)),
      useSettingsStore.getState().setHotkey(old.preferences.hotkey).then(ok => { if (!ok) throw new Error("hotkey"); }),
    ]);
    throw new Error(rollback.every(v => v.status === "fulfilled") ? "apply_restored" : "apply_partial");
  }
  useSettingsStore.setState({
    theme: p.theme, windowStyle: p.windowStyle, autoLockDelaySeconds: p.autoLockDelaySeconds,
    discordRpcEnabled: p.discordRpcEnabled, minimizeToTray: p.minimizeToTray, closeToTray: p.closeToTray === true, startMinimized: p.startMinimized === true,
  });
  useGameStore.setState({ autoLockAgent: p.autoLockAgent, pausedAutoLockAgent: p.pausedAutoLockAgent, mapAgentPreferences: { ...p.mapAgentPreferences } });
  useI18n.getState().setLocale(p.locale);
  usePersonalStore.getState().setStream(p.stream);
}
