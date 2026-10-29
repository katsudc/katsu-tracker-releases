import { useEffect, useRef } from "react";
import { useShallow } from "zustand/react/shallow";
import { useGameStore } from "../stores/gameStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useTrackerStore } from "../tracker/store";
import { useI18n } from "../lib/i18n";
import { usePrivateNames } from "../features/privacy";
import { invoke } from "./ipc";
import { useAccountStore } from "./accountStore";
import { accountId } from "./storage";
const currentPreferences = () => { const s = useGameStore.getState(); return { autoLockAgent:s.autoLockAgent,pausedAutoLockAgent:s.pausedAutoLockAgent,mapAgentPreferences:{...s.mapAgentPreferences},delay:useSettingsStore.getState().autoLockDelaySeconds }; };
export function useAccountLifecycle() {
  const { status, game } = useGameStore(useShallow(s => ({ status:s.status,game:s.gameState.state }))), profile = useTrackerStore(s => s.data.profile);
  const active = useAccountStore(s => s.active), separate = useAccountStore(s => s.separate);
  const applied = useRef<string | null>(null);
  useEffect(() => {
    let live = true;
    if (status !== "CONNECTED") { useAccountStore.getState().setActive(null); applied.current=null; return; }
    void invoke<string | null>("own_account_id").then(id => { if (!live) return; const value=accountId(id)?id:null; useAccountStore.getState().setActive(value); if(value)useAccountStore.getState().observe(value,profile?.puuid===value?profile.name:null); }).catch(()=>{});
    return () => { live=false; };
  }, [status,game,profile?.puuid,profile?.name]);
  useEffect(() => {
    if (!separate || !active || game!=="idle") { if(!separate)applied.current=null; return; }
    if(applied.current===active)return;
    const store=useAccountStore.getState(), prefs=store.accounts.find(a=>a.id===active)?.agents;
    applied.current=active;
    if (!prefs) store.save(active,currentPreferences());
    else {
      useGameStore.setState({autoLockAgent:prefs.autoLockAgent,pausedAutoLockAgent:prefs.pausedAutoLockAgent,mapAgentPreferences:{...prefs.mapAgentPreferences}});
      useSettingsStore.getState().setAutoLockDelaySeconds(prefs.delay);
      void invoke("set_auto_lock",{agent:prefs.pausedAutoLockAgent?null:prefs.autoLockAgent}).catch(()=>{});
      void invoke("set_map_preferences",{preferences:prefs.pausedAutoLockAgent?{}:prefs.mapAgentPreferences}).catch(()=>{});
    }
  },[active,separate,game]);
  useEffect(() => {
    const save=()=>{const s=useAccountStore.getState();if(s.separate&&s.active&&applied.current===s.active) s.save(s.active,currentPreferences());};
    const a=useGameStore.subscribe((s,p)=>{if(s.autoLockAgent!==p.autoLockAgent||s.pausedAutoLockAgent!==p.pausedAutoLockAgent||s.mapAgentPreferences!==p.mapAgentPreferences)save();});
    const b=useSettingsStore.subscribe((s,p)=>{if(s.autoLockDelaySeconds!==p.autoLockDelaySeconds)save();});
    return()=>{a();b();};
  },[]);
}
export function AccountProfiles(){ const {t,locale}=useI18n(), hidden=usePrivateNames(), {accounts,active,separate,setSeparate}=useAccountStore(), game=useGameStore(s=>s.gameState.state);
  return <section className="personal-card"><h3>{t("suite.accounts")}</h3><p>{t("suite.accountHint")}</p><label className="personal-check"><input type="checkbox" checked={separate} disabled={game!=="idle"||!active} onChange={e=>setSeparate(e.target.checked)}/>{t("suite.separateAgents")}</label><p className="personal-muted">{t("suite.separateHint")}</p>{!active&&<p className="personal-muted">{t("suite.noAccount")}</p>}<ul className="suite-account-list">{accounts.map((a,i)=><li key={a.id}><span>{hidden?t("suite.accountLabel",{n:i+1}):a.name??t("suite.accountLabel",{n:i+1})}</span><small>{a.id===active?t("suite.connected"):new Date(a.seen).toLocaleDateString(locale==="tr"?"tr-TR":"en-US")}</small></li>)}</ul></section>;
}
