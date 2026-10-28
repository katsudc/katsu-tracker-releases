import { useEffect, useRef, useState } from "react";
import type { PointerEvent } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { Check, Move, RotateCcw, X } from "lucide-react";
import previewHud from "../assets/spike-editor-hud.png";
import { useI18n } from "../lib/i18n";
import { DEFAULT_APPEARANCE, type SpikeAppearance } from "./spikeAppearance";

type Context={width:number;height:number;scale:number;game_found:boolean};
const FALLBACK:Context={width:1920,height:1080,scale:1,game_found:false};
export function SpikeOverlayEditor({onClose}:{onClose:()=>void}) {
  const {t}=useI18n();
  const [value,setValue]=useState<SpikeAppearance>(DEFAULT_APPEARANCE);
  const [context,setContext]=useState(FALLBACK);
  const [ready,setReady]=useState(false);
  const [saving,setSaving]=useState(false);
  const [error,setError]=useState(false);
  const stage=useRef<HTMLDivElement>(null);
  const [stageWidth,setStageWidth]=useState(760);
  const latest=useRef(value), queue=useRef(Promise.resolve()), debounce=useRef<ReturnType<typeof setTimeout>|undefined>(undefined);
  const mounted=useRef(true), dirty=useRef(false), generation=useRef(0);
  const drag=useRef<{kind:"move"|"size";x:number;y:number;value:SpikeAppearance}|null>(null);
  const doneButton=useRef<HTMLButtonElement>(null);
  function persist(next:SpikeAppearance) {
    const id=++generation.current;
    setSaving(true);setError(false);
    queue.current=queue.current.catch(()=>{}).then(async()=>{
      try {await invoke("set_spike_appearance",{appearance:next});if(id===generation.current){dirty.current=false;if(mounted.current)setError(false);}}
      catch {if(mounted.current)setError(true);}
      finally {if(mounted.current&&id===generation.current)setSaving(false);}
    });
    return queue.current;
  }
  function change(next:SpikeAppearance) {
    if(next.position) {
      const minX=Math.min(0.5,Math.max(200,next.font_size*4.2+12)*context.scale/context.width/2);
      const minY=Math.min(0.5,(next.font_size*1.2+10)*context.scale/context.height/2);
      next={...next,position:{x:Math.max(minX,Math.min(1-minX,next.position.x)),y:Math.max(minY,Math.min(1-minY,next.position.y))}};
    }
    latest.current=next;setValue(next);dirty.current=true;setSaving(true);setError(false);
    clearTimeout(debounce.current);debounce.current=setTimeout(()=>void persist(next),180);
  }
  async function close() {
    clearTimeout(debounce.current);
    if(dirty.current)await persist(latest.current);else await queue.current;
    // Keep the editor open if storage failed, so the user can retry.
    try { if(dirty.current){setError(true);return;}onClose(); } catch {setError(true);}
  }
  useEffect(()=>{
    mounted.current=true;
    void Promise.all([invoke<SpikeAppearance>("get_spike_appearance"),invoke<Context>("get_spike_editor_context")]).then(([a,c])=>{
      if(!mounted.current)return;if(a){setValue(a);latest.current=a;}if(c)setContext(c);setReady(true);doneButton.current?.focus();
    }).catch(()=>{if(mounted.current)setError(true);});
    return()=>{mounted.current=false;clearTimeout(debounce.current);if(dirty.current)void persist(latest.current);};
  },[]);
  useEffect(()=>{
    const element=stage.current;if(!element)return;
    const observer=new ResizeObserver(()=>setStageWidth(element.clientWidth));observer.observe(element);
    return()=>observer.disconnect();
  },[]);
  const ratio=context.scale*stageWidth/context.width;
  const height=value.font_size*1.2+10;
  const position=value.position??{x:0.5,y:(28+height/2)*context.scale/context.height};
  function move(e:PointerEvent<HTMLDivElement>) {
    const item=drag.current,box=stage.current?.getBoundingClientRect();if(!item||!box)return;
    const base=item.value.position??{x:0.5,y:(28+(item.value.font_size*1.2+10)/2)*context.scale/context.height};
    if(item.kind==="size")change({...item.value,font_size:Math.round(Math.max(14,Math.min(84,item.value.font_size+(e.clientX-item.x)/Math.max(ratio*2,0.1))))});
    else change({...item.value,position:{x:Math.max(0,Math.min(1,base.x+(e.clientX-item.x)/box.width)),y:Math.max(0,Math.min(1,base.y+(e.clientY-item.y)/box.height))}});
  }
  function start(e:PointerEvent<HTMLElement>,kind:"move"|"size") {
    if(!ready)return;e.preventDefault();e.stopPropagation();e.currentTarget.setPointerCapture(e.pointerId);
    drag.current={kind,x:e.clientX,y:e.clientY,value:latest.current};
  }
  return createPortal(<div className="spike-editor-backdrop" onKeyDown={e=>{
    if(e.key==="Escape"){e.stopPropagation();void close();}
    if(e.key==="Tab") {const controls=Array.from(e.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),[tabindex="0"]'));const first=controls[0],last=controls[controls.length-1];if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus();}}
  }}>
    <section role="dialog" aria-modal="true" aria-labelledby="spike-editor-title" className="spike-editor">
      <header><div><span className="spike-editor-eyebrow">SPIKE TIMER</span><h2 id="spike-editor-title">{t("spike.editTitle")}</h2></div>
        <button aria-label={t("common.close")} onClick={()=>void close()}><X size={18}/></button></header>
      <p className="spike-editor-hint"><Move size={14}/>{t("spike.dragHint")}</p>
      <div ref={stage} className="spike-editor-stage" style={{aspectRatio:`${context.width}/${context.height}`,width:`min(100%, ${51*context.width/context.height}vh)`}} onPointerMove={move} onPointerUp={()=>{drag.current=null;}} onPointerCancel={()=>{drag.current=null;}}>
        <img className="spike-preview-hud" src={previewHud} alt="" draggable={false}/>
        <div className="spike-preview-timer" tabIndex={0} role="slider" aria-label={t("spike.position")} aria-valuetext={`${Math.round(position.x*100)}%, ${Math.round(position.y*100)}%`} aria-valuenow={Math.round(position.x*100)} aria-valuemin={0} aria-valuemax={100}
          style={{left:`${position.x*100}%`,top:`${position.y*100}%`,fontSize:value.font_size*ratio,lineHeight:1.2,padding:`${2*ratio}px ${6*ratio}px`,background:`rgba(8,14,18,${value.background_opacity})`}}
          onPointerDown={e=>start(e,"move")} onKeyDown={e=>{if(["ArrowUp","ArrowDown","ArrowLeft","ArrowRight"].includes(e.key)){e.preventDefault();const d=e.shiftKey?0.02:0.002;change({...value,position:{x:Math.max(0,Math.min(1,position.x+(e.key==="ArrowRight"?d:e.key==="ArrowLeft"?-d:0))),y:Math.max(0,Math.min(1,position.y+(e.key==="ArrowDown"?d:e.key==="ArrowUp"?-d:0)))}});}}}>
          32.0<button className="spike-resize-handle" aria-label={t("spike.resize")} onPointerDown={e=>start(e,"size")}/>
        </div>
      </div>
      <div className="spike-editor-controls">
        <label>{t("spike.size")}<output>{value.font_size} px</output><input aria-label={t("spike.size")} disabled={!ready} type="range" min="14" max="84" step="1" value={value.font_size} onChange={e=>change({...value,font_size:Number(e.target.value)})}/></label>
        <label>{t("spike.background")}<output>{Math.round(value.background_opacity*100)}%</output><input aria-label={t("spike.background")} disabled={!ready} type="range" min="0" max="1" step="0.01" value={value.background_opacity} onChange={e=>change({...value,background_opacity:Number(e.target.value)})}/></label>
      </div>
      <p className="spike-editor-note">{context.width} × {context.height} · {t(context.game_found?"spike.gamePreview":"spike.samplePreview")} · {t("spike.colorHint")}</p>
      <footer><button disabled={!ready} onClick={()=>change({...DEFAULT_APPEARANCE,game_bar:latest.current.game_bar})}><RotateCcw size={14}/>{t("spike.reset")}</button>
        <span role="status" className={error?"spike-editor-error":""}>{t(error?"spike.saveError":saving?"spike.saving":"spike.saved")}</span>
        <button ref={doneButton} disabled={!ready} className="spike-editor-done" onClick={()=>void close()}><Check size={14}/>{t("spike.done")}</button></footer>
    </section>
  </div>,document.body);
}
