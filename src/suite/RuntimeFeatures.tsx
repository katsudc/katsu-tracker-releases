import { useEffect, useRef, useState } from "react";
import { invoke } from "./ipc";
import { listen } from "@tauri-apps/api/event";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import { useShallow } from "zustand/react/shallow";
import { useI18n } from "../lib/i18n";
import { useGameStore } from "../stores/gameStore";
import { useSettingsStore } from "../stores/settingsStore";
import { usePersonalStore } from "../features/personalStore";
import { toast } from "../features/notifications";
import { Dialog } from "../features/Dialog";
import { accountId } from "./storage";
import { useRuntimeStore } from "./runtimeStore";
import { useMarkerConfig } from "./markerConfig";

export function RuntimeFeatures() {
  const { t } = useI18n();
  const runtime = useRuntimeStore();
  const [busy, setBusy] = useState(false), [error, setError] = useState(false);
  useEffect(() => { void useRuntimeStore.getState().start(); }, []);
  async function choose(safe: boolean) {
    setBusy(true); setError(false);
    try { await runtime.choose(safe); } catch { setError(true); } finally { setBusy(false); }
  }
  if (runtime.mode === "pending") return <Dialog title={t("suite.safeTitle")} onClose={() => {}} dismissible={false}>
    <p className="personal-muted">{t("suite.safeDescription")}</p><p className="personal-muted">{t("suite.safeModules")}</p>
    <div className="personal-actions"><button disabled={busy} onClick={() => void choose(true)}>{t("suite.safeStart")}</button><button className="personal-primary" disabled={busy} onClick={() => void choose(false)}>{t("suite.normalStart")}</button></div>
    {error && <p role="alert" className="personal-error">{t("tools.error")}</p>}
  </Dialog>;
  if (runtime.mode === "safe") return <aside className="suite-runtime-banner" role="status"><strong>{t("suite.safeActive")}</strong><span>{t("suite.safeModules")}</span></aside>;
  if (runtime.mode === "unavailable" || runtime.failed_modules.length) return <aside className="suite-runtime-banner" role="status">{t(runtime.mode === "unavailable" ? "suite.runtimeError" : "suite.moduleError")}</aside>;
  return null;
}

export function useTrayBridge() {
  const { locale } = useI18n();
  const mode = useRuntimeStore(s => s.mode);
  const startedMinimized = useRuntimeStore(s => s.started_minimized);
  const stream = usePersonalStore(s => s.stream.enabled);
  const { minimizeToTray, closeToTray, startMinimized } = useSettingsStore(useShallow(s => ({ minimizeToTray: s.minimizeToTray, closeToTray: s.closeToTray, startMinimized: s.startMinimized })));
  const { agent, paused, maps, game } = useGameStore(useShallow(s => ({ agent: s.autoLockAgent, paused: s.pausedAutoLockAgent, maps: s.mapAgentPreferences, game: s.status==="CONNECTED"?s.gameState.state:"disconnected" })));
  const [spike, setSpike] = useState(false);
  useEffect(() => { if (startedMinimized) useSettingsStore.setState({ isWindowVisible: false }); }, [startedMinimized]);
  useEffect(() => {
    let live = true; const off: Array<() => void> = [];
    const subscribe = async () => {
      const a = await listen<{ enabled: boolean }>("spike-status", e => { if (live && e.payload) setSpike(e.payload.enabled); });
      if (!live) { a(); return; } off.push(a);
      const b = await listen<string>("tray-action", async e => {
        try {
          if (e.payload === "streamer") { const s = usePersonalStore.getState(); s.setStream({ enabled: !s.stream.enabled }); }
          if (e.payload === "instalock") useGameStore.getState().toggleAutoLock();
          if (e.payload === "spike" && useRuntimeStore.getState().mode === "normal") {
            const current = await invoke<{ enabled: boolean }>("spike_get_status");
            await invoke("spike_set_enabled", { enabled: !current.enabled });
          }
        } catch { toast.error(useI18n.getState().t("tools.error")); }
      });
      if (!live) { b(); return; } off.push(b);
      const current = await invoke<{ enabled: boolean }>("spike_get_status");
      if (live && current) setSpike(current.enabled);
    };
    void subscribe().catch(() => {});
    return () => { live = false; off.forEach(fn => fn()); };
  }, [mode]);
  useEffect(() => {
    if (mode === "loading") return;
    void invoke("tray_sync", { snapshot: {
      locale, streamer: stream, spike, instalock: !!agent || (Object.keys(maps).length > 0 && !paused), game,
      preferences: { minimizeToTray, closeToTray, startMinimized },
    } }).catch(() => { /* Missing tray degrades to the existing window/hotkey controls. */ });
  }, [locale, stream, spike, agent, paused, maps, game, minimizeToTray, closeToTray, startMinimized, mode]);
}

export function useMomentShortcut() {
  const { enabled, hotkey } = useMarkerConfig();
  const overlayKey = useSettingsStore(s => s.hotkey);
  const mode = useRuntimeStore(s => s.mode);
  const game = useGameStore(s => s.gameState);
  const observed = useRef<{ id: string | null; at: number }>({ id: null, at: Date.now() });
  useEffect(() => {
    if (game.state === "ingame" && game.match_id && observed.current.id !== game.match_id) observed.current = { id: game.match_id, at: Date.now() };
  }, [game.state, game.match_id]);
  useEffect(() => {
    if (!enabled || mode !== "normal") return;
    let live = true, registered = false, pressed=false;
    const same = (key: string) => key.toLowerCase().replace(/control/g, "ctrl");
    if (same(hotkey) === same(overlayKey) || same(hotkey) === "ctrl+k") {
      toast.error(useI18n.getState().t("suite.markerConflict")); return;
    }
    void register(hotkey, async event => {
      if(!live)return;
      if(event.state!=="Pressed"){pressed=false;return;}
      if(pressed)return;pressed=true;
      const current = useGameStore.getState().gameState, me = current.allies.find(p => p.is_me);
      const t = useI18n.getState().t;
      if (current.state !== "ingame" || !current.match_id || !accountId(me?.puuid)) { toast.error(t("suite.markerUnavailable")); return; }
      try {
        const { useMarkerStore } = await import("./markerStore");
        useMarkerStore.getState().add({ id: crypto.randomUUID(), account: me.puuid, match: current.match_id, at: Date.now(),
          observedElapsed: observed.current.id === current.match_id ? Math.max(0, Date.now() - observed.current.at) : null, round: null, note: "" });
        toast.success(t("suite.marked"));
      } catch { toast.error(t("tools.error")); }
    }).then(async () => { registered = true; if (!live) await unregister(hotkey).catch(() => {}); }).catch(() => {
      if (live) toast.error(useI18n.getState().t("suite.markerConflict"));
    });
    return () => { live = false; if (registered) void unregister(hotkey).catch(() => {}); };
  }, [enabled, hotkey, overlayKey, mode]);
}
