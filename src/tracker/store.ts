import {create} from 'zustand';
import {listen} from '@tauri-apps/api/event';
import { invoke } from "@tauri-apps/api/core";
import {EMPTY_SNAPSHOT,type TrackerSnapshot} from './models';
let startPromise:Promise<void>|null=null,queued:boolean|null=null;
interface TrackerStore {data:TrackerSnapshot;busy:boolean;selectedId:string|null;postId:string|null;limit:5|10|20;start:()=>Promise<void>;refresh:(force?:boolean)=>Promise<void>;select:(id:string|null)=>void;setLimit:(n:5|10|20)=>void;setPost:(id:string|null)=>void}
export const useTrackerStore=create<TrackerStore>((set,get)=>({
 data:EMPTY_SNAPSHOT,busy:false,selectedId:null,postId:null,limit:20,
 start:()=>{
  if(startPromise)return startPromise;
  startPromise=(async()=>{
   let receivedUpdate=false;
   try{
    await listen<TrackerSnapshot>('tracker_updated',event=>{if(event.payload){receivedUpdate=true;set({data:event.payload});}});
    const data=await invoke<TrackerSnapshot>('tracker_load');
    if(data&&!receivedUpdate)set({data});
   }catch{set({data:{...get().data,status:'unavailable'}});}
  })();
  return startPromise;
 },
 refresh:async(force=false)=>{
  if(get().busy){queued=(queued??false)||force;return;}if(get().data.retry_at && get().data.retry_at!*1000>Date.now())return;
  set({busy:true});try{const data=await invoke<TrackerSnapshot>('tracker_refresh',{force});if(data)set({data});}catch{set({data:{...get().data,status:'unavailable'}});}finally{set({busy:false});if(queued!==null){const force=queued;queued=null;void get().refresh(force);}}
 },
 select:selectedId=>set({selectedId}),setLimit:limit=>set({limit}),setPost:postId=>set({postId}),
}));
