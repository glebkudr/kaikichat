import {useEffect,useRef,useState} from 'react';
import type {Card,CardDraft,CardKind,DesktopApi,Found,Handle,LinkOpen,LoginProvider,Published} from './types';
import {ProviderButtons} from './ProviderButtons';
import {useDescribe,useT} from './i18n';

/** A field's words as a card takes them: lowercase, each once, at most `limit`. */
export function cardWords(text:string,limit:number):string[] {
  return [...new Set(text.toLowerCase().split(/[\s,]+/).filter(Boolean))].slice(0,limit);
}

/** Asking someone found for a conversation; a retry keeps its operation. */
function useContactRequest(api:DesktopApi,onOpen:(conversationId:string)=>void) {
  const describe=useDescribe();
  const operations=useRef(new Map<string,string>());
  const [busy,setBusy]=useState('');const [error,setError]=useState('');
  async function ask(networkId:string,name:string) {
    if(busy)return;
    const operationId=operations.current.get(networkId)??crypto.randomUUID();
    operations.current.set(networkId,operationId);
    setBusy(networkId);setError('');
    try {const made=await api.requestContact({networkId,name,operationId});operations.current.delete(networkId);onOpen(made.conversationId);}
    catch(e){setError(describe(e));}
    finally {setBusy('');}
  }
  return {busy,error,ask};
}

/** People the owner knows by address: counted first, then checked for a
 * coin each. */
function KnownPeople({api,onOpen}:{api:DesktopApi;onOpen:(conversationId:string)=>void}) {
  const t=useT();const describe=useDescribe();
  const [text,setText]=useState('');const [handles,setHandles]=useState<Handle[]|null>(null);
  const [found,setFound]=useState<Found[]|null>(null);
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const contact=useContactRequest(api,onOpen);
  function edit(next:string){setText(next);setHandles(null);setFound(null);}
  async function count() {
    setBusy(true);setError('');
    try {setHandles((await api.discoverHandles({text})).handles);}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  async function check() {
    if(!handles?.length)return;
    setBusy(true);setError('');
    try {setFound((await api.discoverLookup({handles})).found);}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  async function open(file:File|undefined) {
    if(!file)return;
    const read=await file.text();
    edit(text.trim()?`${text}\n${read}`:read);
  }
  return <section className="agent-form" aria-label={t.discover.knownTitle}>
    <h3>{t.discover.knownTitle}</h3><p className="agent-muted">{t.discover.knownText}</p>
    {(error||contact.error)&&<p className="network-validation" role="alert">{error||contact.error}</p>}
    <label>{t.discover.addresses}<textarea rows={4} spellCheck={false} value={text} disabled={busy} onChange={e=>edit(e.target.value)}/></label>
    <label className="file-pick">{t.discover.openFile}<input type="file" accept=".vcf,.csv,.txt,text/vcard,text/csv,text/plain" disabled={busy} onChange={e=>void open(e.target.files?.[0])}/></label>
    {handles===null?<button type="button" disabled={busy||!text.trim()} onClick={()=>void count()}>{t.discover.count}</button>:
      handles.length===0?<p className="agent-muted" role="status">{t.discover.noAddresses}</p>:
      found===null&&<><p className="claim-outcome" role="status">{t.discover.counted(handles.length)}</p><button type="button" disabled={busy} onClick={()=>void check()}>{busy?t.discover.checking:t.discover.check(handles.length)}</button></>}
    {found&&<ul className="member-list" aria-label={t.discover.results}>{found.map(item=><li key={`${item.kind}:${item.handle}`} className="member-row">
      <span className="member-id" dir="auto">{item.handle}</span>
      {item.networkId?<button type="button" className="secondary member-remove" disabled={contact.busy!==''} onClick={()=>void contact.ask(item.networkId!,item.kind==='github'?item.handle:item.handle.split('@')[0])}>{t.discover.addContact}</button>:<span className="role-badge">{t.discover.notFound}</span>}
    </li>)}</ul>}
  </section>;
}

/** Cards of open groups, channels and people, found by what they wrote; free
 * with an active book. */
function Explore({api,onOpen}:{api:DesktopApi;onOpen:(conversationId:string)=>void}) {
  const t=useT();const describe=useDescribe();
  const [query,setQuery]=useState('');const [kind,setKind]=useState<CardKind|''>('');
  const [cards,setCards]=useState<Card[]|null>(null);
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const contact=useContactRequest(api,onOpen);
  async function search() {
    setBusy(true);setError('');
    try {setCards((await api.discoverSearch({query:query.trim(),...(kind?{kind}:{})})).cards);}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  async function follow(card:Card) {
    if(!card.groupRef||busy)return;
    setBusy(true);setError('');
    try {onOpen((await api.followGroup({group:card.groupRef,owner:card.owner,name:card.name})).id);}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  return <section className="agent-form" aria-label={t.discover.searchTitle}>
    <h3>{t.discover.searchTitle}</h3><p className="agent-muted">{t.discover.searchText}</p>
    {(error||contact.error)&&<p className="network-validation" role="alert">{error||contact.error}</p>}
    <form className="discover-search" onSubmit={e=>{e.preventDefault();void search();}}>
      <label>{t.discover.query}<input value={query} maxLength={200} disabled={busy} onChange={e=>setQuery(e.target.value)}/></label>
      <label>{t.discover.kind}<select value={kind} disabled={busy} onChange={e=>setKind(e.target.value as CardKind|'')}>
        <option value="">{t.discover.kindAll}</option><option value="group">{t.discover.kindGroups}</option><option value="channel">{t.discover.kindChannels}</option><option value="profile">{t.discover.kindPeople}</option>
      </select></label>
      <button disabled={busy}>{busy?t.discover.searching:t.discover.search}</button>
    </form>
    {cards&&cards.length===0&&<p className="agent-muted" role="status">{t.discover.nothingFound}</p>}
    {cards&&cards.length>0&&<><ul className="discover-cards" aria-label={t.discover.results}>{cards.map(card=><li key={card.id} className="discover-card">
      <div className="discover-card-head"><strong dir="auto">{card.name}</strong><span className="role-badge">{card.kind==='group'?t.discover.group:card.kind==='channel'?t.discover.channel:t.discover.person}</span></div>
      <p dir="auto">{card.about}</p>
      {card.tags.length>0&&<small dir="auto">{card.tags.map(tag=>`#${tag}`).join(' ')}</small>}
      {card.kind!=='profile'?<button type="button" className="secondary" disabled={busy} onClick={()=>void follow(card)}>{card.kind==='channel'?t.discover.followChannel:t.discover.follow}</button>:
        <button type="button" className="secondary" disabled={contact.busy!==''} onClick={()=>void contact.ask(card.owner,card.name)}>{t.discover.addContact}</button>}
    </li>)}</ul><small className="agent-muted">{t.discover.untrusted}</small></>}
  </section>;
}

/** A card for 30 days, for ten coins: this profile's, or an open group's. */
export function CardForm({api,draft,title,text}:{api:DesktopApi;draft:Pick<CardDraft,'kind'|'groupId'>;title:string;text:string}) {
  const t=useT();const describe=useDescribe();
  const [about,setAbout]=useState('');const [tags,setTags]=useState('');const [langs,setLangs]=useState('');
  const [published,setPublished]=useState<Published|null>(null);
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');const [notice,setNotice]=useState('');
  const date=(seconds:number)=>new Date(seconds*1000).toLocaleDateString(t.tag,{day:'2-digit',month:'2-digit',year:'numeric'});
  async function publish() {
    setBusy(true);setError('');setNotice('');
    try {setPublished(await api.discoverPublish({...draft,about:about.trim(),tags:cardWords(tags,8),langs:cardWords(langs,4)}));}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  async function withdraw() {
    if(!published)return;
    setBusy(true);setError('');
    try {await api.discoverWithdraw({cardId:published.id});setPublished(null);setNotice(t.discover.withdrawn);}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  return <section className="agent-form" aria-label={title}>
    <h3>{title}</h3><p className="agent-muted">{text}</p>
    {error&&<p className="network-validation" role="alert">{error}</p>}
    {notice&&<p className="claim-outcome" role="status">{notice}</p>}
    <label>{t.discover.about}<textarea rows={3} maxLength={500} value={about} disabled={busy} onChange={e=>setAbout(e.target.value)}/></label>
    <label>{t.discover.tags}<input value={tags} disabled={busy} spellCheck={false} onChange={e=>setTags(e.target.value)}/></label>
    <label>{t.discover.langs}<input value={langs} disabled={busy} spellCheck={false} onChange={e=>setLangs(e.target.value)}/></label>
    {published&&<p className="claim-outcome" role="status">{t.discover.published(date(published.expiresAt))}</p>}
    <div className="actions">
      {published&&<button type="button" className="secondary" disabled={busy} onClick={()=>void withdraw()}>{t.discover.withdraw}</button>}
      <button type="button" disabled={busy||!about.trim()} onClick={()=>void publish()}>{busy?t.discover.publishing:t.discover.publish}</button>
    </div>
  </section>;
}

/** Signing in with an account makes this profile findable by it: the page
 * opens in the browser, and the window asks the service until it decides. */
function Findable({api,poll}:{api:DesktopApi;poll:number}) {
  const t=useT();const describe=useDescribe();
  const [link,setLink]=useState<LinkOpen|null>(null);const [state,setState]=useState('');
  const [busy,setBusy]=useState<LoginProvider|''>('');const [error,setError]=useState('');
  useEffect(()=>{
    if(!link)return;
    let stopped=false;let timer:ReturnType<typeof setTimeout>|undefined;
    const ask=async()=>{
      try {
        const status=await api.discoverStatus({linkId:link.linkId});
        if(stopped)return;
        if(status.status==='linked'){setState(t.discover.linked);setLink(null);return;}
        if(status.status==='denied'){setState(t.discover.denied);setLink(null);return;}
      } catch(e){if(!stopped){setError(describe(e));setLink(null);}return;}
      if(Date.now()/1000<link.expiresAt)timer=setTimeout(()=>void ask(),poll);else setLink(null);
    };
    timer=setTimeout(()=>void ask(),poll);
    return ()=>{stopped=true;clearTimeout(timer);};
  },[api,link,poll,t,describe]);
  async function open(kind:LoginProvider) {
    setBusy(kind);setError('');setState('');
    try {setLink(await api.discoverLink({kind}));}
    catch(e){setError(describe(e));}
    finally {setBusy('');}
  }
  async function unlink(kind:LoginProvider) {
    setBusy(kind);setError('');setState('');
    try {await api.discoverUnlink({kind});setState(t.discover.unlinked);}
    catch(e){setError(describe(e));}
    finally {setBusy('');}
  }
  return <section className="agent-form" aria-label={t.discover.findableTitle}>
    <h3>{t.discover.findableTitle}</h3><p className="agent-muted">{t.discover.findableText}</p>
    {error&&<p className="network-validation" role="alert">{error}</p>}
    {link&&<p className="claim-outcome" role="status">{t.discover.code(link.code)}<span className="claim-note">{t.discover.waiting}</span></p>}
    {state&&<p className="claim-outcome" role="status">{state}</p>}
    <ProviderButtons busy={busy} disabled={link!==null} onPick={kind=>void open(kind)}/>
    <div className="actions">
      <button type="button" className="secondary" disabled={busy!==''} onClick={()=>void unlink('google')}>{t.discover.unlinkGoogle}</button>
      <button type="button" className="secondary" disabled={busy!==''} onClick={()=>void unlink('github')}>{t.discover.unlinkGithub}</button>
    </div>
  </section>;
}

/** Finding people and groups: by an address the owner knows, or by
 * interest; and being found. Nothing lists who is there. */
export function DiscoverPanel({api,onOpen,onBack,poll=2000}:{api:DesktopApi;onOpen:(conversationId:string)=>void;onBack:()=>void;poll?:number}) {
  const t=useT();
  return <section className="agent-panel" aria-label={t.discover.label}>
    <header className="agent-header"><div><span className="eyebrow">{t.discover.eyebrow}</span><h2>{t.discover.title}</h2><p>{t.discover.text}</p></div><button className="secondary" onClick={onBack}>{t.common.back}</button></header>
    <div className="agent-columns">
      <div className="panel-stack">
        <KnownPeople api={api} onOpen={onOpen}/>
        <Explore api={api} onOpen={onOpen}/>
      </div>
      <div className="panel-stack">
        <Findable api={api} poll={poll}/>
        <CardForm api={api} draft={{kind:'profile'}} title={t.discover.cardTitle} text={t.discover.cardText}/>
      </div>
    </div>
  </section>;
}
