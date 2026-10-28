import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowDownToLine, Check, LoaderCircle, RefreshCw, ShieldCheck, X } from "lucide-react";
import { applying, useUpdateStore, type UpdateStatus } from "../stores/updateStore";
import { useI18n } from "../lib/i18n";
import "./updates.css";

const ERRORS=new Set(["game_running","signature","check_failed","download_failed","install_failed","restart_failed","helper_failed","close_failed","storage","invalid_release","package_size"]);
function errorKey(code:string|null) { return ERRORS.has(code??"")?`update.error.${code}`:"update.error.install_failed"; }
function Progress({status:s}:{status:UpdateStatus}) {
  const {t}=useI18n();
  const percent=s.total&&s.total>0?Math.min(100,Math.floor(s.downloaded/s.total*100)):null;
  const phase=["verifying","preparing","installing","restarting","complete","error"].includes(s.phase)?s.phase:"downloading";
  return <div className="update-progress-card">
    <div className="update-mark">{s.phase==="complete"?<Check/>:s.phase==="verifying"?<ShieldCheck/>:<LoaderCircle className={s.phase==="error"?"":"update-spin"}/>}</div>
    <div className="update-overline">katsu’s Tracker {s.version?`· v${s.version}`:""}</div>
    <h2>{t(`update.${phase}`)}</h2>
    <p>{t(s.phase==="error"?errorKey(s.error):s.phase==="downloading"?"update.downloadHint":s.phase==="restarting"?"update.restartHint":"update.installHint")}</p>
    {s.phase==="downloading"&&<><div role="progressbar" aria-label={t("update.downloading")} aria-valuenow={percent??undefined} aria-valuemin={0} aria-valuemax={100} className={`update-track ${percent===null?"update-indeterminate":""}`}><i style={{width:percent===null?"35%":`${percent}%`}}/></div>
      <div className="update-transfer"><span>{(s.downloaded/1048576).toFixed(1)} MB{s.total?` / ${(s.total/1048576).toFixed(1)} MB`:""}</span><strong>{percent===null?t("update.downloading"):`${percent}%`}</strong></div></>}
    {!['downloading','error','complete'].includes(s.phase)&&<div role="progressbar" aria-label={t(`update.${phase}`)} className="update-track update-indeterminate"><i/></div>}
    {s.phase!=="error"&&<div className="update-steps"><span className={s.phase==="downloading"?"active":""}>1 · {t("update.downloadStep")}</span><span className={['verifying','preparing','installing'].includes(s.phase)?"active":""}>2 · {t("update.installStep")}</span><span className={s.phase==="restarting"||s.phase==="complete"?"active":""}>3 · {t("update.restartStep")}</span></div>}
  </div>;
}
export function UpdateNotice() {
  const s=useUpdateStore(v=>v.status),check=useUpdateStore(v=>v.check),apply=useUpdateStore(v=>v.restartToUpdate);
  const {t}=useI18n();
  const [dismissed,setDismissed]=useState<string|null>(null),[checked,setChecked]=useState(false);
  useEffect(()=>{setChecked(false);if(s.phase==="up_to_date"){const id=setTimeout(()=>setChecked(true),4000);return()=>clearTimeout(id);}},[s.phase]);
  if(applying(s.phase))return <div className="update-modal" role="dialog" aria-modal="true" aria-label={t("update.installing")}><Progress status={s}/></div>;
  if(checked||dismissed===`${s.phase}:${s.version}`)return null;
  return <div className={`update-notice update-${s.phase}`} role="status">
    {s.phase==="checking"?<LoaderCircle className="update-spin" size={16}/>:s.phase==="up_to_date"?<Check size={16}/>:<ArrowDownToLine size={16}/>}
    <span>{t(s.phase==="checking"?"update.checking":s.phase==="up_to_date"?"update.upToDate":s.phase==="error"?errorKey(s.error):"update.available",{version:s.version??""})}</span>
    {s.phase==="available"&&<button className="update-primary" onClick={()=>void apply()}>{t("update.now")}</button>}
    {s.phase==="error"&&<button onClick={()=>void(s.version?apply():check())}><RefreshCw size={12}/>{t("update.retry")}</button>}
    {s.phase!=="checking"&&<button className="update-dismiss" aria-label={t("common.close")} onClick={()=>setDismissed(`${s.phase}:${s.version}`)}><X size={14}/></button>}
  </div>;
}
export function UpdateWorker() {
  const s=useUpdateStore(v=>v.status),init=useUpdateStore(v=>v.init);
  const {t}=useI18n();
  useEffect(()=>{void init().then(()=>invoke("update_worker_ready")).catch(()=>{});},[init]);
  return <main className="update-worker"><div data-tauri-drag-region className="update-worker-drag"/><Progress status={s}/>
    {s.phase==="error"&&<div className="update-worker-actions"><button onClick={()=>void invoke("retry_update_worker")}>{t("update.retry")}</button><button onClick={()=>void invoke("reopen_after_update")}>{t("update.openTracker")}</button><button onClick={()=>void getCurrentWindow().close()}>{t("common.close")}</button></div>}
  </main>;
}
