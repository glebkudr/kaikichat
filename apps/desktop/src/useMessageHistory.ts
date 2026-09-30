import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import type {ConversationHistory,DesktopApi,Message,Snapshot} from './types';
import {useDescribe,useT} from './i18n';

export function useMessageHistory(api:DesktopApi,conversationId:string|undefined,revision:Snapshot|undefined,visible:boolean) {
  const [page,setPage]=useState<ConversationHistory&{expanded:boolean}>();
  const [error,setError]=useState('');
  const [loading,setLoading]=useState(false);
  const [olderBusy,setOlderBusy]=useState(false);
  const [retry,setRetry]=useState(0);
  const cached=useRef(page);cached.current=page;
  const active=useRef(conversationId);active.current=conversationId;
  const list=useRef<HTMLElement>(null);
  const scroll=useRef<'bottom'|{top:number;height:number}|null>(null);
  const loadingOlder=useRef(false);
  const generation=useRef(0);
  const t=useT();const describe=useDescribe();
  useEffect(()=>{
    let cancelled=false;
    const currentGeneration=++generation.current;
    const stale=()=>cancelled||generation.current!==currentGeneration;
    if(!conversationId)return;
    setLoading(true);setError('');
    void api.conversationHistory({conversationId,before:null}).then(async latest=>{
      if(stale())return;
      if(latest.conversationId!==conversationId)throw new Error(t.shell.otherConversation);
      // A receipt can arrive for an older queued message after its page was
      // loaded. Reuse bounded pages only where outstanding delivery needs refresh.
      const updates=new Map<string,Message>();
      const saved=cached.current;
      if(saved?.conversationId===conversationId&&saved.expanded) {
        const overlap=saved.messages.findIndex(m=>m.id===latest.messages[0]?.id);
        for(let i=overlap-1;i>=0;i--) {
          const item=saved.messages[i];
          if(!item.own||!['queued','stored'].includes(item.delivery.phase)||updates.has(item.id))continue;
          const batch=await api.conversationHistory({conversationId,before:saved.messages[i+1].id});
          if(stale())return;
          if(batch.conversationId!==conversationId)throw new Error(t.shell.otherConversation);
          for(const message of batch.messages)updates.set(message.id,message);
        }
      }
      const el=list.current;
      if(!el||el.scrollHeight-el.scrollTop-el.clientHeight<80)scroll.current='bottom';
      setPage(previous=>{
        if(previous?.conversationId!==conversationId){scroll.current='bottom';return {...latest,expanded:false};}
        if(!previous.expanded)return {...latest,expanded:false};
        const overlap=previous.messages.findIndex(m=>m.id===latest.messages[0]?.id);
        // A disjoint latest page starts a new contiguous window. Its cursor lets
        // the owner load every intervening message instead of hiding a gap.
        if(overlap<0){scroll.current='bottom';return {...latest,expanded:false};}
        return {...latest,expanded:true,messages:[...previous.messages.slice(0,overlap).map(m=>updates.get(m.id)??m),...latest.messages],nextBefore:previous.nextBefore};
      });
    }).catch(err=>{if(!stale())setError(describe(err));}).finally(()=>{if(!stale())setLoading(false);});
    return()=>{cancelled=true;};
  },[api,conversationId,revision,retry,t,describe]);
  useLayoutEffect(()=>{
    const el=list.current;
    if(!el||!visible)return;
    if(scroll.current==='bottom')el.scrollTop=el.scrollHeight;
    else if(scroll.current)el.scrollTop=scroll.current.top+el.scrollHeight-scroll.current.height;
    scroll.current=null;
  },[page,visible]);
  const current=page?.conversationId===conversationId?page:undefined;
  async function older() {
    if(!current?.nextBefore||loadingOlder.current)return;
    const {conversationId,before}={conversationId:current.conversationId,before:current.nextBefore};
    loadingOlder.current=true;setOlderBusy(true);setError('');
    try {
      const batch=await api.conversationHistory({conversationId,before});
      if(active.current!==conversationId)return;
      if(batch.conversationId!==conversationId)throw new Error(t.shell.otherConversation);
      setPage(previous=>{
        if(previous?.conversationId!==conversationId||previous.nextBefore!==before)return previous;
        const el=list.current;
        if(el)scroll.current={top:el.scrollTop,height:el.scrollHeight};
        const ids=new Set(previous.messages.map(m=>m.id));
        return {...previous,expanded:true,messages:[...batch.messages.filter(m=>!ids.has(m.id)),...previous.messages],nextBefore:batch.nextBefore};
      });
    } catch(err){if(active.current===conversationId)setError(describe(err));}
    finally{loadingOlder.current=false;setOlderBusy(false);}
  }
  function append(id:string,message:Message) {
    if(active.current!==id)return;
    ++generation.current;setLoading(false);
    scroll.current='bottom';
    setPage(previous=>previous?.conversationId===id?{...previous,messages:previous.messages.some(m=>m.id===message.id)?previous.messages.map(m=>m.id===message.id?message:m):[...previous.messages,message]}:previous);
  }
  return {page:current,error,loading,olderBusy,older,append,retry:()=>setRetry(n=>n+1),list};
}
