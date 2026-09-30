import {useEffect,useRef,useState} from 'react';
import {CoreError} from './core-error';
import type {DesktopApi,Recommended} from './types';

/** The kinds this window offers: a channel is followed, a group joined. */
export const known=(list:Recommended[]|undefined)=>(list??[]).filter(r=>r.kind==='channel'||r.kind==='group');

/** One join per group: a retry is the same request to the node. */
const joinOperation=(r:Recommended)=>`recommended-join-${r.ref.slice(0,32)}`;

/** Follows a recommended channel or joins a recommended group. */
export async function take(api:DesktopApi,r:Recommended) {
  if(r.kind==='channel')await api.followGroup({group:r.ref,owner:r.owner,name:r.name});
  else await api.joinGroup({groupRef:r.ref,note:'',operationId:joinOperation(r)});
}

// Refusals that pass: no book yet (the free messages are on their way), the
// network or the node away for now.
const waiting=new Set(['book_required','network_unavailable','card_pending','unavailable','daemon_unavailable']);
const shouldWait=(error:unknown)=>error instanceof CoreError&&(error.retryable||waiting.has(error.code));

const storageKey='agentic.recommended.pending';
// Tells the window a choice was kept, so it starts watching for its book.
const keptEvent='agentic-recommended-kept';
function pending():Recommended[] {
  try {const kept=JSON.parse(localStorage.getItem(storageKey)??'[]') as unknown;return Array.isArray(kept)?known(kept as Recommended[]):[];}
  catch {return [];}
}
function keep(list:Recommended[]) {
  try {if(list.length)localStorage.setItem(storageKey,JSON.stringify(list));else localStorage.removeItem(storageKey);}
  catch { /* the choice lasts for this window only */ }
}

/** Takes what the owner chose; what has to wait for the free messages (or the
 * network) is kept and taken later by useRecommendedPending. */
export async function takeChosen(api:DesktopApi,chosen:Recommended[]) {
  const later:Recommended[]=[];
  for(const r of chosen) {
    try {await take(api,r);}
    catch(error) {if(shouldWait(error))later.push(r);}
  }
  if(later.length) {
    keep([...pending().filter(p=>!later.some(l=>l.ref===p.ref)),...later]);
    window.dispatchEvent(new Event(keptEvent));
  }
}

/** Takes the kept choice whenever the node says something changed (a book
 * arriving, the network coming back); a final refusal drops it. */
export function useRecommendedPending(api:DesktopApi) {
  const running=useRef(false);
  // Bumped when a choice is kept or the last one is taken: the watch below
  // runs only while something waits.
  const [round,setRound]=useState(0);
  // A choice kept just now failed a moment ago: wait for the node to say
  // something changed instead of asking again at once.
  const justKept=useRef(false);
  useEffect(()=>{
    const kept=()=>{justKept.current=true;setRound(n=>n+1);};
    window.addEventListener(keptEvent,kept);
    return ()=>window.removeEventListener(keptEvent,kept);
  },[]);
  useEffect(()=>{
    if(!pending().length)return;
    let live=true;
    const attempt=async()=>{
      if(running.current)return;
      const list=pending();if(!list.length)return;
      running.current=true;
      try {
        const left:Recommended[]=[];
        for(const r of list) {
          try {await take(api,r);}
          catch(error) {if(shouldWait(error))left.push(r);}
        }
        keep(left);
        if(!left.length&&live)setRound(n=>n+1);
      } finally {running.current=false;}
    };
    if(!justKept.current)void attempt();
    justKept.current=false;
    const stop=api.subscribe(()=>{void attempt();});
    return ()=>{live=false;stop();};
  },[api,round]);
}

/** Renders nothing: takes the kept choice while the profile is open. */
export function RecommendedPending({api}:{api:DesktopApi}) {
  useRecommendedPending(api);
  return null;
}
