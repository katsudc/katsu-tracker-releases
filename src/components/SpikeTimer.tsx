import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./spike.css";
import { useI18n } from "../lib/i18n";
import { useSpikeAppearance } from "./spikeAppearance";
import { SpikeOverlayEditor } from "./SpikeOverlayEditor";

type Status = {
  icon_enabled:boolean; icon_status:string; icon_similarity:number;
  enabled: boolean; calibrated: boolean; recording: boolean; recorded_seconds: number;
  recording_ready: boolean; remaining: number | null; full_defuse: boolean; half_defuse: boolean;
  correlation_ready: boolean; correlation_threshold: number; correlation_similarity: number;
  full_sequence: boolean; reference_seconds: number;
  demo: boolean; similarity: number; peak_similarity: number; listening: boolean; level: number; threshold: number; reference_name: string; error: string | null;
};
const INITIAL: Status = { icon_enabled:true, icon_status:"Bekleniyor", icon_similarity:0, enabled:false, calibrated:false, recording:false, recorded_seconds:0,
  recording_ready:false, remaining:null, full_defuse:false, half_defuse:false, demo:false,
  correlation_ready:false, correlation_threshold:0.25, correlation_similarity:0, full_sequence:false, reference_seconds:0, similarity:0, peak_similarity:0, listening:false, level:0, threshold:0.9, reference_name:"Yok", error:null };

function useSpikeStatus() {
  const [status, setStatus] = useState(INITIAL);
  const [error, setError] = useState("");
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const update = (s: Status) => { if (!disposed && s) setStatus(s); };
    // Subscribe before reading so a state change cannot be lost between the two.
    void listen<Status>("spike-status", e => update(e.payload)).then(async off => {
      if (disposed) { off(); return; }
      unlisten = off;
      update(await invoke<Status>("spike_get_status"));
    }).catch(e => { if (!disposed) setError(String(e)); });
    return () => { disposed = true; unlisten?.(); };
  }, []);
  return { status, error, setError };
}

export function SpikeOverlay() {
  const { status:s } = useSpikeStatus();
  const appearance = useSpikeAppearance();
  useEffect(() => {
    document.documentElement.classList.add("spike-window");
    return () => document.documentElement.classList.remove("spike-window");
  }, []);
  if (s.remaining === null) return null;
  return <div className={`spike-overlay ${s.full_defuse ? "spike-full" : s.half_defuse ? "spike-half" : "spike-danger"}`}>
    <strong className="spike-seconds" style={{fontSize:appearance.font_size,lineHeight:1.2,minWidth:appearance.font_size*2.5,background:`rgba(8,14,18,${appearance.background_opacity})`}}>{s.remaining.toFixed(1)}</strong>
  </div>;
}

export function SpikeSettings() {
  const { status:s, error, setError } = useSpikeStatus();
  const [busy, setBusy] = useState(false);
  const [editing,setEditing] = useState(false);
  const {t} = useI18n();
  async function run(command:string, args?:Record<string,unknown>) {
    setBusy(true); setError("");
    try { await invoke(command,args); } catch(e) { setError(String(e)); }
    finally {setBusy(false);}
  }
  return <section className="spike-settings spike-simple">
    <div className="spike-setting-heading"><h3>SPIKE TIMER</h3>
      <button role="switch" aria-label="Spike timer" aria-checked={s.enabled} disabled={busy}
        onClick={()=>void run("spike_set_enabled",{enabled:!s.enabled})}>{t(s.enabled ? "spike.on" : "spike.off")}</button></div>
    <button className="spike-edit-button" onClick={()=>setEditing(true)}>{t("spike.edit")}</button>
    {editing && <SpikeOverlayEditor onClose={()=>setEditing(false)}/>}
    {error && <p role="alert" className="spike-warning">{error}</p>}
  </section>;
}
