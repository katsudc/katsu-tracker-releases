import {useEffect,useState} from 'react';
import {create} from 'zustand';
import {CachedImage} from '../components/CachedImage';
import {Shield} from 'lucide-react';
function loadCached():Record<string,string>{try{const saved=JSON.parse(localStorage.getItem('katsu-rank-icons-v1')??'{}');if(!saved||typeof saved!=='object'||Array.isArray(saved))return{};return Object.fromEntries(Object.entries(saved).filter((row):row is [string,string]=>typeof row[1]==='string'&&row[1].startsWith('https://media.valorant-api.com/')));}catch{return{};}}
const useRanks=create<{icons:Record<string,string>;loaded:boolean;load:()=>Promise<void>}>((set,get)=>({icons:loadCached(),loaded:false,load:async()=>{
 if(get().loaded)return;set({loaded:true});try{const result=await fetch('https://valorant-api.com/v1/competitivetiers');if(!result.ok)return;const data=await result.json();const tiers=data.data?.at(-1)?.tiers;if(!Array.isArray(tiers))return;const icons:Record<string,string>={};for(const tier of tiers){if(Number.isInteger(tier.tier)&&typeof tier.smallIcon==='string'&&tier.smallIcon.startsWith('https://media.valorant-api.com/'))icons[tier.tier]=tier.smallIcon;}set({icons});try{localStorage.setItem('katsu-rank-icons-v1',JSON.stringify(icons));}catch{}}catch{}
}}));
export function RankIcon({tier,compact=false}:{tier:number|null|undefined;compact?:boolean}){
 const {icons,load}=useRanks();const [failed,setFailed]=useState(false);const source=tier!=null?icons[tier]:null;
 useEffect(()=>{void load();},[load]);useEffect(()=>setFailed(false),[source]);
 return <span className={`tracker-rank-icon ${compact?'rank-icon-compact':''}`} aria-hidden>{source&&!failed?<CachedImage src={source} alt="" className="rank-icon-art" onError={()=>setFailed(true)}/>:<Shield className="rank-icon-art" strokeWidth={1.2}/>}</span>;
}
