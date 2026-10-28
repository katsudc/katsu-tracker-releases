import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../lib/i18n";
import { useSettingsStore } from "./settingsStore";

export type UpdateStatus = {
  revision:number; phase:string; version:string|null; downloaded:number; total:number|null;
  error:string|null; cached:boolean; locale:string; theme:string;
};
const INITIAL:UpdateStatus={revision:0,phase:"checking",version:null,downloaded:0,total:null,error:null,cached:false,locale:"en",theme:"rose"};
export const applying=(phase:string)=>["downloading","verifying","preparing","installing","restarting"].includes(phase);
interface UpdateStore {
  status:UpdateStatus; readyVersion:string|null; isRestarting:boolean;
  init:()=>Promise<void>; check:()=>Promise<void>; restartToUpdate:()=>Promise<void>;
}
let initialization:Promise<void>|undefined;
export const useUpdateStore=create<UpdateStore>((set,get)=>{
  const accept=(status:UpdateStatus)=>{
    if(!status||status.revision<get().status.revision)return;
    set({status,readyVersion:status.version,isRestarting:applying(status.phase)});
    if(location.hash==="#updater") {
      useI18n.getState().setLocale(status.locale==="tr"?"tr":"en");
      document.documentElement.dataset.theme=status.theme;
    }
  };
  return {
    status:INITIAL,readyVersion:null,isRestarting:false,
    init:()=>initialization??=(async()=>{
      try {
        await listen<UpdateStatus>("update_status",e=>accept(e.payload));
        if(location.hash!=="#updater")await listen<string>("update_ready",e=>{
          const status=get().status;
          if(!applying(status.phase)&&status.phase!=="available")accept({...status,phase:"available",version:e.payload});
        });
        const status=await invoke<UpdateStatus>("get_update_status");if(status)accept(status);
      } catch {accept({...get().status,phase:"error",error:"check_failed"});}
    })(),
    check:async()=>{
      if(applying(get().status.phase))return;
      set({status:{...get().status,phase:"checking",error:null}});
      try {await invoke("check_for_updates");}catch{accept({...get().status,phase:"error",error:"check_failed"});}
    },
    restartToUpdate:async()=>{
      if(!get().readyVersion||get().isRestarting)return;
      set({isRestarting:true,status:{...get().status,phase:"preparing",error:null}});
      try {await invoke("apply_update",{locale:useI18n.getState().locale,theme:useSettingsStore.getState().theme});}
      catch {if(applying(get().status.phase))accept({...get().status,phase:"error",error:"install_failed"});}
    },
  };
});
