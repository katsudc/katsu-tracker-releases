import { useEffect, useState } from "react";
import { useI18n } from "../lib/i18n";
import { useGameStore } from "../stores/gameStore";
import { useTrackerStore } from "../tracker/store";
import { useSettingsStore } from "../stores/settingsStore";
import { usePersonalStore, DEFAULT_STREAM } from "../features/personalStore";
import { useLibraryStore } from "../features/libraryStore";
import { useLineupsStore } from "../stores/lineupsStore";
import { Dialog } from "../features/Dialog";
import { applyPreferences, capturePreferences } from "../features/preferenceTransfer";
import { DEFAULT_APPEARANCE } from "../components/spikeAppearance";
import { invoke } from "./ipc";
import { AccountProfiles } from "./AccountProfiles";
import { useAccountStore } from "./accountStore";
import { useMarkerStore } from "./markerStore";
import { useMarkerConfig } from "./markerConfig";
import { useNoteStore } from "./noteStore";
import { useProfileTools } from "./profileStore";
import { useGoalStore } from "./goalStore";
import { useRuntimeStore } from "./runtimeStore";
type ClearKind="cache"|"notes"|"lineups"|"all";
const keys=["valorant-tracker-settings-v3","valorant-tracker-game","valorant-tracker-locale","katsu-personal-tools-v1","katsu-lineups-v1","katsu-lineup-library-v1","katsu-marker-config-v1","katsu-moments-v1","katsu-notes-v1","katsu-player-tools-v1","katsu-session-goals-v1","katsu-accounts-v1"];
export function PrivacyDashboard(){
 const {t}=useI18n(),status=useGameStore(s=>s.status),game=useGameStore(s=>s.gameState.state),refreshing=useTrackerStore(s=>s.busy),discord=useSettingsStore(s=>s.discordRpcEnabled),stream=usePersonalStore(s=>s.stream.enabled),normal=useRuntimeStore(s=>s.mode==="normal"),[integration,setIntegration]=useState<{spotify:boolean|null;widget:boolean|null}>({spotify:null,widget:null});
 const [confirm,setConfirm]=useState<ClearKind|null>(null),[busy,setBusy]=useState(false),[message,setMessage]=useState(""),[error,setError]=useState(false),[size,setSize]=useState(0);
 useEffect(()=>{let live=true;let bytes=0;for(const k of keys)bytes+=(localStorage.getItem(k)?.length??0)*2;setSize(bytes);void Promise.allSettled([invoke<{connected:boolean}>("media_status"),invoke<{widget_installed:boolean}>("get_spike_output_status")]).then(([a,b])=>{if(live)setIntegration({spotify:a.status==="fulfilled"?a.value.connected:null,widget:b.status==="fulfilled"?b.value.widget_installed:null});});return()=>{live=false;};},[]);
 const blocked=game==="ingame"||game==="pregame"||refreshing;
 async function clear(){if(!confirm)return;const kind=confirm;if((kind==="all"||kind==="cache")&&(blocked||useTrackerStore.getState().busy||["ingame","pregame"].includes(useGameStore.getState().gameState.state)))return;setBusy(true);setError(false);setMessage("");try{
   if(kind==="cache"||kind==="all")await invoke("tracker_clear_cache");
   if(kind==="all"){
     if(!normal)throw Error("safe");
     const file=await capturePreferences();file.preferences={...file.preferences,theme:"rose",locale:"en",hotkey:"F2",windowStyle:"free",autoLockDelaySeconds:5,discordRpcEnabled:false,minimizeToTray:false,closeToTray:false,startMinimized:false,autoLockAgent:null,pausedAutoLockAgent:null,mapAgentPreferences:{},spikeEnabled:false,appearance:{...DEFAULT_APPEARANCE,game_bar:{font_size:30,background_opacity:0.78}},stream:{...DEFAULT_STREAM}};
     await applyPreferences(file);
     useAccountStore.getState().clear();useProfileTools.getState().clear();useGoalStore.setState({byAccount:{}});useMarkerConfig.getState().set(false,"F8");usePersonalStore.setState({sessionStarts:{},setupComplete:true});
     useLineupsStore.setState({agent:"brimstone",map:"haven",side:"attack"});
     useGameStore.setState({pausedByUser:false});
   }
   if(kind==="notes"||kind==="all"){useNoteStore.getState().clear();useMarkerStore.getState().clear();}
   if(kind==="lineups"||kind==="all")useLibraryStore.setState({items:[]});
   if(kind==="all")for(const key of keys)localStorage.removeItem(key+".recovery");
   setMessage(t("suite.cleared"));setConfirm(null);
 }catch{setError(true);}finally{setBusy(false);}}
 const state=(v:boolean|null)=>v===null?"—":t(v?"spike.on":"spike.off");
 return <section className="suite-tool"><h3>{t("suite.privacy")}</h3><p className="personal-muted">{t("suite.privacyHint")}</p><h4>{t("suite.localData")}</h4><p className="personal-muted">{[t("settings.title"),t("tracker.history"),t("library.title"),t("suite.notes"),t("suite.crosshairs"),t("suite.sensitivity"),t("suite.goals"),t("suite.accounts")].join(" · ")}</p><p className="personal-muted">{(size/1024).toFixed(1)} KiB · {t("suite.preferenceSize")}</p><h4>{t("suite.services")}</h4><dl className="suite-privacy-services">{[["Riot",status==="CONNECTED"?t("suite.connected"):t("suite.notConnected"),"suite.riotPrivacy"],["Spotify",normal?state(integration.spotify):t("suite.paused"),"suite.spotifyPrivacy"],["Discord",state(discord&&normal&&!stream),"suite.discordPrivacy"],["Game Bar",!normal?t("suite.paused"):integration.widget===null?"—":t(integration.widget?"suite.installed":"suite.notInstalled"),"suite.gamebarPrivacy"]].map(([name,state,key])=><div key={name}><dt>{name}<small>{state}</small></dt><dd>{t(key)}</dd></div>)}</dl><p className="personal-muted">{t("suite.otherNetwork")}</p><AccountProfiles/><div className="personal-actions">{(["cache","notes","lineups","all"] as ClearKind[]).map(kind=><button key={kind} disabled={busy||((kind==="cache"||kind==="all")&&blocked)||(kind==="all"&&!normal)} onClick={()=>{setConfirm(kind);setError(false);}}>{t(kind==="cache"?"suite.clearCache":kind==="notes"?"suite.clearNotes":kind==="lineups"?"suite.clearLineups":"suite.clearAll")}</button>)}</div>{blocked&&<p className="personal-muted">{t("suite.clearBusy")}</p>}{message&&<p role="status" className="personal-success">{message}</p>}{error&&<p role="alert" className="personal-error">{t("tools.error")}</p>}{confirm&&<Dialog title={t("suite.clearConfirm")} onClose={()=>{if(!busy)setConfirm(null);}}><p>{t(confirm==="all"?"suite.clearAllHint":"suite.clearConfirm")}</p><div className="personal-actions"><button disabled={busy} onClick={()=>setConfirm(null)}>{t("tools.cancel")}</button><button className="personal-primary" disabled={busy||((confirm==="cache"||confirm==="all")&&blocked)} onClick={()=>void clear()}>{t("tools.apply")}</button></div>{error&&<p role="alert">{t("tools.error")}</p>}</Dialog>}</section>;
}
