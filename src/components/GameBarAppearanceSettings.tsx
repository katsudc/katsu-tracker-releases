import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../lib/i18n";
import { useSpikeAppearance, type GameBarAppearance } from "./spikeAppearance";

export function GameBarAppearanceSettings() {
  const { t } = useI18n();
  const appearance = useSpikeAppearance();
  const [value, setValue] = useState<GameBarAppearance>({font_size:30,background_opacity:0.78});
  const [state, setState] = useState<"saved"|"saving"|"error">("saved");
  const pending = useRef<GameBarAppearance | null>(null), queue = useRef(Promise.resolve());
  const debounce = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const mounted = useRef(true), revision = useRef(0);
  useEffect(() => {
    if (!pending.current) setValue(appearance.game_bar ?? {font_size:appearance.font_size,background_opacity:appearance.background_opacity});
  }, [appearance]);
  function save(next: GameBarAppearance, id = revision.current) {
    queue.current = queue.current.catch(() => {}).then(async () => {
      try {
        await invoke("set_spike_gamebar_appearance", {appearance:next});
        if (id === revision.current) { pending.current = null; if (mounted.current) setState("saved"); }
      } catch { if (mounted.current && id === revision.current) setState("error"); }
    });
  }
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; clearTimeout(debounce.current); if (pending.current) save(pending.current); };
  }, []);
  function change(next: GameBarAppearance) {
    const id=++revision.current;pending.current = next;setValue(next);setState("saving");clearTimeout(debounce.current);
    debounce.current = setTimeout(() => save(next,id),180);
  }
  return <div className="gamebar-appearance-settings">
    <label><span>{t("spike.size")} <strong>{value.font_size}px</strong></span>
      <input aria-label={`${t("spike.gameBarAppearance")} · ${t("spike.size")}`} type="range" min="14" max="84" step="1" value={value.font_size} onChange={e=>change({...value,font_size:Number(e.target.value)})}/></label>
    <label><span>{t("spike.background")} <strong>{Math.round(value.background_opacity*100)}%</strong></span>
      <input aria-label={`${t("spike.gameBarAppearance")} · ${t("spike.background")}`} type="range" min="0" max="100" step="1" value={Math.round(value.background_opacity*100)} onChange={e=>change({...value,background_opacity:Number(e.target.value)/100})}/></label>
    <small role="status" className={state === "error" ? "spike-warning" : ""}>{t(state === "error" ? "spike.autoSaveError" : state === "saving" ? "spike.saving" : "spike.saved")}</small>
  </div>;
}
