import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type GameBarAppearance = { font_size:number; background_opacity:number };
export type SpikeAppearance = { game_bar?:GameBarAppearance; position: { x:number; y:number } | null; font_size:number; background_opacity:number };
export const DEFAULT_APPEARANCE:SpikeAppearance = { position:null, font_size:30, background_opacity:0.78 };
export function useSpikeAppearance() {
  const [appearance,setAppearance]=useState(DEFAULT_APPEARANCE);
  useEffect(()=>{
    let disposed=false, changed=false;
    let off:(()=>void)|undefined;
    void listen<SpikeAppearance>("spike-overlay-appearance",e=>{changed=true;if(!disposed)setAppearance(e.payload);}).then(async unlisten=>{
      if(disposed){unlisten();return;}off=unlisten;
      const value=await invoke<SpikeAppearance>("get_spike_appearance");
      if(!disposed&&!changed&&value)setAppearance(value);
    }).catch(()=>{/* Defaults remain usable while the backend starts. */});
    return()=>{disposed=true;off?.();};
  },[]);
  return appearance;
}
