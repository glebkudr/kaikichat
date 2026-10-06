import {useCallback,useEffect,useRef,useState} from 'react';
import {AgentPanel} from './AgentPanel';
import {SettingsPanel} from './SettingsPanel';
import {ThemeSwitch,applyTheme,savedTheme} from './theme';
import {ContactsPanel} from './ContactsPanel';
import {GroupPanel,NewGroupPanel} from './GroupPanel';
import {WalletPanel} from './WalletPanel';
import {DiscoverPanel} from './DiscoverPanel';
import {KeychainScreen,NoKeychainScreen,Onboarding,UnlockScreen} from './Onboarding';
import {StartHere} from './StartHere';
import {RecommendedPending} from './Recommended';
import {MoveNotice} from './MoveApp';
import {DeliveryBadge} from './DeliveryBadge';
import {NavMenu} from './NavMenu';
import {NetworkNotice} from './NetworkPreset';
import {ReleaseNotice} from './Release';
import {useMessageHistory} from './useMessageHistory';
import {CoreError,isCode} from './core-error';
import {shortId} from './group-roles';
import {LanguageSelect,LocaleProvider,useDescribe,useT,type Locale} from './i18n';
import type {DesktopApi,Follow,Group,ProfileStatus,Snapshot} from './types';

type Panel='chat'|'contacts'|'new-group'|'group'|'agents'|'wallet'|'network'|'discover';
type Gate={kind:'loading'}|{kind:'locked';status:ProfileStatus}|{kind:'keychain'}|{kind:'no-keychain';reason:string}|{kind:'stopped';reason?:string}|{kind:'failed'}|{kind:'ready'};

/** The screen of a node that does not run: stopped by someone, did not
 * start (with the reason), or no keychain for the profile's key. */
function idleGate(error:unknown):Gate|undefined {
  if(!(error instanceof CoreError))return undefined;
  if(error.code==='keychain_unavailable')return {kind:'no-keychain',reason:error.message};
  if(error.code==='start_failed')return {kind:'stopped',reason:error.message};
  if(error.code==='daemon_unavailable')return {kind:'stopped'};
  return undefined;
}

/** The owner's window, in the language the owner chose (English at first). */
export function ChatShell({api,locale}:{api:DesktopApi;locale?:Locale}) {
  useEffect(()=>applyTheme(savedTheme()),[]);
  return <LocaleProvider initial={locale}><Shell api={api}/></LocaleProvider>;
}

function Shell({api}:{api:DesktopApi}) {
  const t=useT();const errorText=useDescribe();
  const [panel,setPanel]=useState<Panel>('chat');
  const [gate,setGate]=useState<Gate>({kind:'loading'});
  const [snapshot,setSnapshot]=useState<Snapshot>();
  const [groups,setGroups]=useState<Group[]>([]);const [requests,setRequests]=useState(0);
  const [follows,setFollows]=useState<Follow[]>([]);
  const [error,setError]=useState(''); const [selected,setSelected]=useState('');
  const [draft,setDraft]=useState(''); const [busy,setBusy]=useState(false);
  // The first run keeps its own screens until the owner finishes it.
  const [firstRun,setFirstRun]=useState(false);
  const pending=useRef<{conversationId:string;text:string;operationId:string}|null>(null);
  const generation=useRef(0);
  const reload=useCallback(async()=>{
    const current=++generation.current;
    try {
      const state=await api.snapshot();
      if(current!==generation.current)return;
      setSnapshot(state);setGate({kind:'ready'});
      if(!state.identity)return;
      // Groups and requests decorate the list; the chats work without them.
      const [listed,waiting,read]=await Promise.allSettled([api.groups(),api.introRequests(),api.follows()]);
      if(current!==generation.current)return;
      if(listed.status==='fulfilled')setGroups(listed.value);
      if(waiting.status==='fulfilled')setRequests(waiting.value.length);
      if(read.status==='fulfilled')setFollows(read.value);
    } catch(err) {
      if(current!==generation.current)return;
      if(isCode(err,'profile_locked')) {
        try {const status=await api.profileStatus();if(current===generation.current)setGate({kind:'locked',status});}
        catch(inner){setError(errorText(inner));setGate({kind:'failed'});}
      } else if(isCode(err,'keychain_consent'))setGate({kind:'keychain'});
      else {
        const idle=idleGate(err);
        if(idle)setGate(idle);
        else {setError(errorText(err));setGate(previous=>previous.kind==='ready'?previous:{kind:'failed'});}
      }
    }
  },[api,errorText]);
  useEffect(()=>{void reload(); const stop=api.subscribe(()=>{void reload();}); return ()=>{++generation.current;stop();};},[api,reload]);
  const conversation=snapshot?.conversations.find(c=>c.id===selected)??snapshot?.conversations[0];
  const group=groups.find(g=>g.id===conversation?.id);
  const follow=follows.find(f=>f.id===conversation?.id);
  const history=useMessageHistory(api,conversation?.id,snapshot,panel==='chat');
  const own=snapshot?.identity?.networkId??'';
  function open(conversationId:string) {
    if(busy)return;
    setPanel('chat');setSelected(conversationId);setDraft('');pending.current=null;setError('');void reload();
  }
  function show(next:Panel){if(busy)return;setPanel(next);setError('');}
  async function submit() {
    if(!conversation||busy||!draft.trim())return;
    const request=pending.current??{conversationId:conversation.id,text:draft,operationId:crypto.randomUUID()};
    pending.current=request;setBusy(true);setError('');
    try {
      const message=await api.sendMessage(request);
      history.append(request.conversationId,message);
      pending.current=null;setDraft('');
    } catch(err){setError(errorText(err));} finally {setBusy(false);}
  }
  async function unfollow(groupId:string) {
    setBusy(true);setError('');
    try {await api.unfollowGroup({groupId});setSelected('');await reload();}
    catch(err){setError(errorText(err));}
    finally {setBusy(false);}
  }
  async function restart() {
    setBusy(true);setError('');
    try {await api.reconnect();await reload();}
    catch(err){const idle=idleGate(err);if(idle)setGate(idle);else setError(errorText(err));}
    finally {setBusy(false);}
  }
  const identity=snapshot?.identity;
  const ready=gate.kind==='ready'&&snapshot;
  useEffect(()=>{if(ready&&!identity)setFirstRun(true);},[ready,identity]);
  const notice=<><MoveNotice api={api}/>{ready&&<NetworkNotice api={api} onChanged={()=>void reload()}/>}<ReleaseNotice api={api}/></>;
  if(ready&&(!identity||firstRun))return <><RecommendedPending api={api}/><Onboarding api={api} notice={notice} identity={identity??null} onCreated={()=>void reload()} onFinish={()=>{setFirstRun(false);setPanel('chat');}} onTopUp={()=>{setFirstRun(false);setPanel('wallet');}}/></>;
  const time=(seconds:number)=>new Date(seconds*1000).toLocaleTimeString(t.tag,{hour:'2-digit',minute:'2-digit'});
  const nav=(id:string,target:Panel,label:string,badge?:number)=>({id,label,badge,current:panel===target,onSelect:()=>show(target)});
  return <div className="app-shell">
    {ready&&<RecommendedPending api={api}/>}
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark" aria-hidden="true">k<span>·</span>c</span><div>Kaiki Chat<small>{t.shell.tagline}</small></div></div>
      <div className="sidebar-heading"><h1>{t.shell.messages}</h1>{ready&&identity&&<div className="sidebar-actions"><button className="icon-button" aria-label={t.shell.addContact} onClick={()=>show('contacts')}>+</button><NavMenu disabled={busy} items={[
        nav('contacts','contacts',t.shell.navContacts,requests),
        nav('discover','discover',t.shell.navDiscover),
        nav('new-group','new-group',t.shell.navNewGroup),
        nav('agents','agents',t.shell.navAgents),
        nav('wallet','wallet',t.shell.navWallet),
        nav('settings','network',t.shell.navSettings),
      ]}/></div>}</div>
      <nav aria-label={t.shell.conversations} className="conversations">{ready&&snapshot.conversations.map(c=>{
        const isGroup=groups.some(g=>g.id===c.id)||follows.some(f=>f.id===c.id);
        const isChannel=groups.find(g=>g.id===c.id)?.kind==='channel'||follows.find(f=>f.id===c.id)?.kind==='channel';
        return <button key={c.id} className={`conversation ${panel==='chat'&&conversation?.id===c.id?'selected':''}`} onClick={()=>open(c.id)}>
          <span className={`avatar ${isGroup?'group-avatar':''}`} aria-hidden="true">{isGroup?'#':c.title.slice(0,1)}</span><span className="conversation-text"><strong dir="auto">{c.title}</strong><span dir="auto">{c.messages.at(-1)?.text??(isChannel?t.shell.channelPreview:isGroup?t.shell.groupPreview:t.shell.startConversation)}</span></span>{c.unread>0&&<span className="unread">{c.unread}</span>}
        </button>;
      })}</nav>
      <div className="sidebar-footer"><div className={`network-dot ${ready&&snapshot.network.state==='online'?'online':''}`}/><div><strong>{!ready?t.shell.offline:snapshot.network.state==='online'?t.shell.peers(snapshot.network.connectedPeers):snapshot.network.state==='connecting'?t.shell.connecting:t.shell.offline}</strong><small title={own} dir="auto">{identity?.name??t.shell.localProfile}</small></div><div className="footer-controls"><ThemeSwitch compact/><LanguageSelect compact/></div></div>
    </aside>
    <main>
      {error&&<div className="error-banner" role="alert">{error}</div>}
      {notice}
      {gate.kind==='locked'?<UnlockScreen api={api} status={gate.status} onOpened={()=>void reload()}/>:
      gate.kind==='keychain'?<KeychainScreen api={api} onOpened={()=>void reload()}/>:
      gate.kind==='no-keychain'?<NoKeychainScreen reason={gate.reason} busy={busy} onRetry={()=>void restart()}/>:
      gate.kind==='stopped'?<section className="empty"><span className="eyebrow">{t.shell.stoppedEyebrow}</span><h2>{gate.reason?t.shell.startFailedTitle:t.shell.stoppedTitle}</h2>{gate.reason?<><p>{t.shell.startFailedText}</p><p className="failure-reason" dir="auto">{gate.reason}</p></>:<p>{t.shell.stoppedText}</p>}<button disabled={busy} onClick={()=>void restart()}>{busy?t.shell.starting:gate.reason?t.shell.retryStart:t.shell.startNode}</button></section>:
      !ready?<section className="empty"><span className="eyebrow">{t.shell.coreEyebrow}</span><h2>{gate.kind==='failed'?t.shell.coreUnavailable:t.shell.coreConnecting}</h2><p>{t.shell.coreHint}</p>{gate.kind==='failed'&&<button onClick={()=>{setError('');void reload();}}>{t.shell.retryConnect}</button>}</section>:
      !identity?null:
      panel==='network'?<SettingsPanel api={api} onBack={()=>setPanel('chat')}/>:
      panel==='agents'?<AgentPanel api={api} conversations={snapshot.conversations} onBack={()=>setPanel('chat')}/>:
      panel==='wallet'?<WalletPanel api={api} onBack={()=>setPanel('chat')}/>:
      panel==='discover'?<DiscoverPanel api={api} onOpen={open} onBack={()=>setPanel('chat')}/>:
      panel==='contacts'?<ContactsPanel api={api} identity={identity} onOpen={open} onBack={()=>setPanel('chat')}/>:
      panel==='new-group'?<NewGroupPanel api={api} onCreated={open} onBack={()=>setPanel('chat')}/>:
      panel==='group'&&group?<GroupPanel api={api} groupId={group.id} ownId={own} onBack={()=>setPanel('chat')}/>:
      conversation?<>
        <header className="chat-header"><span className={`avatar ${group||follow?'group-avatar':''}`} aria-hidden="true">{group||follow?'#':conversation.title.slice(0,1)}</span><div><h2 dir="auto">{conversation.title}</h2><p>{follow?(follow.closed?t.shell.followClosed:follow.kind==='channel'?t.shell.followChannel:t.shell.followSubtitle):group?(group.kind==='channel'?t.shell.channelSubtitle(t.roles[group.role]):group.access==='public'?t.shell.publicSubtitle(group.members.length,t.roles[group.role]):t.shell.groupSubtitle(group.members.length,t.roles[group.role])):t.shell.directSubtitle}</p></div>{follow?<button className="secondary header-action" disabled={busy} onClick={()=>void unfollow(follow.id)}>{t.shell.unfollow}</button>:group?<button className="secondary header-action" onClick={()=>show('group')}>{t.shell.members}</button>:<span className="private-label">{t.shell.privateLabel}</span>}</header>
        <section ref={history.list} className="message-list" aria-label={t.shell.history} aria-live="polite">
          {history.error&&<div className="history-error" role="alert">{history.error}</div>}
          {history.error&&history.page&&<button className="secondary history-control" disabled={history.loading} onClick={history.retry}>{t.shell.retryHistory}</button>}
          {!history.page&&<button className="secondary history-control" disabled={history.loading} onClick={history.retry}>{history.loading?t.shell.loadingHistory:t.shell.retryHistory}</button>}
          {history.page?.nextBefore&&<button className="secondary history-control" disabled={history.olderBusy} onClick={()=>void history.older()}>{history.olderBusy?t.shell.loadingOlder:t.shell.loadOlder}</button>}
          {history.page?.messages.length===0&&<div className="conversation-empty">{t.shell.noMessages}</div>}
          {history.page?.messages.map(message=><article className={`message ${message.own?'own':'received'}`} key={message.id} data-message-id={message.id}>{(group||follow)&&!message.own&&<span className="message-author" title={message.author}>{shortId(message.author)}</span>}<p dir="auto">{message.text}</p><footer><time dateTime={new Date(message.createdAt*1000).toISOString()}>{time(message.createdAt)}</time>{message.own&&<DeliveryBadge delivery={message.delivery}/>}{!message.own&&message.lowTrust&&<span className="trust-low" title={t.trust.lowHint}>{t.trust.low}</span>}</footer></article>)}
        </section>
        {follow?<p className="composer read-only-note">{follow.kind==='channel'?t.shell.readOnlyChannel:t.shell.readOnly}</p>:<form className="composer" onSubmit={e=>{e.preventDefault();void submit();}}><label className="sr-only" htmlFor="message">{t.shell.message}</label><textarea id="message" value={draft} disabled={busy} onChange={e=>{setDraft(e.target.value);pending.current=null;}} placeholder={group?(group.access==='public'?t.shell.writePublic:t.shell.writeGroup):t.shell.writeMessage} rows={2} maxLength={12000} onKeyDown={e=>{if(e.key==='Enter'&&!e.shiftKey&&!e.nativeEvent.isComposing){e.preventDefault();void submit();}}}/><button type="submit" aria-label={t.shell.send} disabled={busy||!draft.trim()}>{t.shell.sendButton} <span aria-hidden="true">↗</span></button><div className="composer-hint">{t.shell.sendHint}</div></form>}
      </>:<StartHere api={api} identity={identity} onOpen={open} onGroup={()=>show('new-group')} onDiscover={()=>show('discover')}/>}
    </main>
  </div>;
}
