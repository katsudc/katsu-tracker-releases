import {useEffect,useRef} from 'react';
import {useGameStore} from '../stores/gameStore';
import {useLastMatchStore} from '../stores/lastMatchStore';
import {useTrackerStore} from './store';
export function useTrackerLifecycle(){
 const status=useGameStore(s=>s.status),game=useGameStore(s=>s.gameState),last=useLastMatchStore(s=>s.match);
 const previous=useRef({state:game.state,id:game.match_id});const ended=useRef<string|null>(null);const refreshTimer=useRef<ReturnType<typeof setTimeout>|null>(null);
 useEffect(()=>{void useTrackerStore.getState().start().then(()=>useTrackerStore.getState().refresh());},[]);
 useEffect(()=>{if(status==='CONNECTED')void useTrackerStore.getState().refresh();},[status]);
 useEffect(()=>{const old=previous.current;if(old.state==='ingame'&&game.state==='idle'&&old.id)ended.current=old.id;previous.current={state:game.state,id:game.match_id};},[game.state,game.match_id]);
 useEffect(()=>()=>{if(refreshTimer.current)clearTimeout(refreshTimer.current);},[]);
 useEffect(()=>{if(!last||last.match_id!==ended.current)return;ended.current=null;useTrackerStore.getState().setPost(last.match_id);if(refreshTimer.current)clearTimeout(refreshTimer.current);refreshTimer.current=setTimeout(()=>void useTrackerStore.getState().refresh(true),6000);},[last,game.state,game.match_id]);
}
