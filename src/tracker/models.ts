export interface TrackerProfile {puuid:string;name:string|null;region:string;level:number|null}
export interface RankInfo {tier:number|null;rr:number|null;peak:number|null}
export interface PlayerMatchStats {puuid:string;name:string|null;agent:string;team:string;tier:number|null;is_me:boolean;kills:number|null;deaths:number|null;assists:number|null;score:number|null;rounds:number|null;headshots:number|null;bodyshots:number|null;legshots:number|null;damage:number|null}
export interface TrackerMatch {id:string;map:string;queue:string;started_at:number|null;duration_ms:number|null;won:boolean|null;ally_score:number|null;enemy_score:number|null;me:PlayerMatchStats;players:PlayerMatchStats[];season_id?:string|null;first_kills?:number|null;first_deaths?:number|null;opening_rounds?:number|null;rr_delta?:number|null;rr_after?:number|null;tier_after?:number|null}
export interface TrackerSnapshot {schema:number;provider:string;profile:TrackerProfile|null;rank:RankInfo|null;matches:TrackerMatch[];updated_at:number|null;partial:boolean;status:string;retry_at:number|null;latest_update?:{match_id:string|null;season_id:string|null;tier_before:number|null;tier_after:number|null;rr_before:number|null;rr_after:number|null;earned:number|null;at:number|null}|null}
export const EMPTY_SNAPSHOT:TrackerSnapshot={schema:1,provider:'session',profile:null,rank:null,matches:[],updated_at:null,partial:false,status:'no_profile',retry_at:null};
export const divide=(a:number|null|undefined,b:number|null|undefined)=>a!=null&&b!=null&&b>0?a/b:null;
export const acs=(p:PlayerMatchStats)=>divide(p.score,p.rounds);
export const kd=(p:PlayerMatchStats)=>divide(p.kills,p.deaths);
export const hs=(p:PlayerMatchStats)=>p.headshots!=null&&p.bodyshots!=null&&p.legshots!=null?divide(100*p.headshots,p.headshots+p.bodyshots+p.legshots):null;
export const adr=(p:PlayerMatchStats)=>divide(p.damage,p.rounds);
export function outcome(m:TrackerMatch){return m.won===true?'win':m.won===false?'loss':m.ally_score!=null&&m.enemy_score!=null&&m.ally_score===m.enemy_score?'draw':'unknown';}
export function aggregate(matches:TrackerMatch[]){
 const players=matches.map(m=>m.me);
 const total=(field:keyof PlayerMatchStats)=>{const rows=players.map(p=>p[field]);return rows.length&&rows.every(v=>typeof v==='number')?(rows as number[]).reduce((a,b)=>a+b,0):null;};
 const ratio=(a:keyof PlayerMatchStats,b:keyof PlayerMatchStats)=>divide(total(a),total(b));
 const wins=matches.filter(m=>m.won===true).length,losses=matches.filter(m=>m.won===false).length;
 const hits=players.filter(p=>p.headshots!=null&&p.bodyshots!=null&&p.legshots!=null);
 const heads=hits.reduce((n,p)=>n+p.headshots!,0),shots=hits.reduce((n,p)=>n+p.headshots!+p.bodyshots!+p.legshots!,0);
 return {matches:matches.length,wins,losses,draws:matches.filter(m=>outcome(m)==='draw').length,winrate:divide(100*wins,wins+losses),kd:ratio('kills','deaths'),acs:ratio('score','rounds'),adr:ratio('damage','rounds'),hs:hits.length===players.length?divide(heads*100,shots):null,kills:divide(total('kills'),matches.length),deaths:divide(total('deaths'),matches.length),assists:divide(total('assists'),matches.length)};
}
export function groups(matches:TrackerMatch[],kind:'agent'|'map'){
 const grouped=new Map<string,TrackerMatch[]>();for(const match of matches){const key=kind==='map'?match.map:match.me.agent;if(key)grouped.set(key,[...(grouped.get(key)??[]),match]);}
 return [...grouped].map(([name,rows])=>({name,...aggregate(rows)})).sort((a,b)=>b.matches-a.matches||a.name.localeCompare(b.name));
}
