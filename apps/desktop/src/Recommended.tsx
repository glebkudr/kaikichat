import {useEffect,useState} from 'react';
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
const waiting=new Set(['book_required','network_unavailable','card_pending','unavailable','daemon_unavailable','keychain_unavailable']);
const shouldWait=(error:unknown)=>error instanceof CoreError&&(error.retryable||waiting.has(error.code));

const storageKey='agentic.recommended.pending';
// Tells the window a choice was kept, so it starts taking it.
const keptEvent='agentic-recommended-kept';
// The choice when the window cannot store it: it lasts for this window only.
let unsaved:Recommended[]=[];
function pending():Recommended[] {
  try {const kept=JSON.parse(localStorage.getItem(storageKey)??'[]') as unknown;return Array.isArray(kept)?known(kept as Recommended[]):[];}
  catch {return unsaved;}
}
function keep(list:Recommended[]) {
  unsaved=list;
  try {if(list.length)localStorage.setItem(storageKey,JSON.stringify(list));else localStorage.removeItem(storageKey);}
  catch { /* unsaved holds it */ }
}

/** Keeps what the owner chose and returns at once: the node may take a while
 * to follow and join, so useRecommendedPending takes the choice meanwhile and
 * keeps what has to wait for the free messages (or the network). */
export function choose(chosen:Recommended[]) {
  if(!chosen.length)return;
  keep([...pending().filter(p=>!chosen.some(c=>c.ref===p.ref)),...chosen]);
  window.dispatchEvent(new Event(keptEvent));
}

// One pass at a time for the whole window: the onboarding and the shell
// both watch, and a follow already under way is not asked for again.
let passing:Promise<void>|undefined;
/** Takes the kept choice, and what is chosen meanwhile; a final refusal drops
 * it, what has to wait stays kept. */
function takePending(api:DesktopApi):Promise<void> {
  return passing??=(async()=>{
    const tried=new Set<string>();
    const untried=()=>pending().filter(r=>!tried.has(r.ref));
    for(let list=untried();list.length;list=untried()) {
      const settled=new Set<string>();
      for(const r of list) {
        tried.add(r.ref);
        try {await take(api,r);settled.add(r.ref);}
        catch(error) {if(!shouldWait(error))settled.add(r.ref);}
      }
      keep(pending().filter(r=>!settled.has(r.ref)));
    }
  })().finally(()=>{passing=undefined;});
}

/** Takes the kept choice at once and again whenever the node says something
 * changed (a book arriving, the network coming back). */
export function useRecommendedPending(api:DesktopApi) {
  // Bumped when a choice is kept or the last one is taken: the watch below
  // runs only while something waits.
  const [round,setRound]=useState(0);
  useEffect(()=>{
    const kept=()=>setRound(n=>n+1);
    window.addEventListener(keptEvent,kept);
    return ()=>window.removeEventListener(keptEvent,kept);
  },[]);
  useEffect(()=>{
    if(!pending().length)return;
    let live=true;
    const attempt=()=>{void takePending(api).then(()=>{if(live&&!pending().length)setRound(n=>n+1);});};
    attempt();
    const stop=api.subscribe(attempt);
    return ()=>{live=false;stop();};
  },[api,round]);
}

/** Renders nothing: takes the kept choice while the profile is open. */
export function RecommendedPending({api}:{api:DesktopApi}) {
  useRecommendedPending(api);
  return null;
}
