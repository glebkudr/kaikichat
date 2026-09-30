import {useCallback,useEffect,useRef,useState} from 'react';
import type {ChannelStorage,DesktopApi,DoorRequest,Group,GroupAccess,GroupChange,GroupKind,Retention} from './types';
import {groupRights,roleOf,shortId,validNetworkId} from './group-roles';
import {CardForm} from './DiscoverPanel';
import {useDescribe,useT} from './i18n';

type Change=Omit<GroupChange,'operationId'|'groupId'>;

/** One commit at a time; a retried change keeps its operation id. */
function useGroupChange(api:DesktopApi,groupId:string,done:()=>Promise<void>) {
  const describe=useDescribe();
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const pending=useRef<{key:string;operationId:string}|null>(null);
  async function change(what:Change) {
    if(busy)return;
    const key=JSON.stringify(what);
    if(pending.current?.key!==key)pending.current={key,operationId:crypto.randomUUID()};
    setBusy(true);setError('');
    try {await api.changeGroup({...what,groupId,operationId:pending.current.operationId});pending.current=null;await done();return true;}
    catch(e){setError(describe(e));return false;}
    finally {setBusy(false);}
  }
  return {busy,error,change};
}

/** Applications at a group's door, for its owner and admins. */
function DoorRequests({api,groupId}:{api:DesktopApi;groupId:string}) {
  const t=useT();const describe=useDescribe();
  const [waiting,setWaiting]=useState<DoorRequest[]>([]);const [error,setError]=useState('');const [busy,setBusy]=useState(false);
  const load=useCallback(async()=>{
    try {setWaiting(await api.doorRequests({groupId}));setError('');}catch(e){setError(describe(e));}
  },[api,groupId,describe]);
  useEffect(()=>{void load();const stop=api.subscribe(()=>{void load();});return stop;},[api,load]);
  async function decide(request:DoorRequest,accept:boolean) {
    setBusy(true);setError('');
    try {await api.doorDecide({groupId,requestId:request.requestId,accept});await load();}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  return <section className="agent-form" aria-label={t.groups.door}><h3>{t.groups.door}</h3>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    {waiting.length===0?<p className="agent-muted">{t.groups.doorEmpty}</p>:<ul className="member-list">{waiting.map(request=><li key={request.requestId} className="member-row door-row">
      <span className="member-id" title={request.networkId}>{shortId(request.networkId)}</span>
      {request.rejoin&&<span className="role-badge">{t.groups.rejoin}</span>}
      {request.note&&<span className="door-note" dir="auto">{request.note}</span>}
      <button className="secondary member-remove" disabled={busy} aria-label={t.groups.letInLabel(shortId(request.networkId))} onClick={()=>void decide(request,true)}>{t.groups.letIn}</button>
      <button className="secondary member-remove" disabled={busy} aria-label={t.groups.turnAwayLabel(shortId(request.networkId))} onClick={()=>void decide(request,false)}>{t.groups.turnAway}</button>
    </li>)}</ul>}
  </section>;
}

const RETENTIONS:Retention[]=[30,90,180,365,'forever'];

/** How long a public channel keeps its history, and what that costs. */
function ChannelHistory({api,group,busy,change}:{api:DesktopApi;group:Group;busy:boolean;change:(what:Change)=>Promise<boolean|undefined>}) {
  const t=useT();const describe=useDescribe();
  const current:Retention=group.retention===null?'forever':(group.retention as Retention);
  const [kept,setKept]=useState<Retention>(current);
  const [storage,setStorage]=useState<ChannelStorage>();const [error,setError]=useState('');
  useEffect(()=>{setKept(current);},[current]);
  useEffect(()=>{void api.channelStorage({groupId:group.id}).then(setStorage,e=>setError(describe(e)));},[api,group.id,group.epoch,describe]);
  return <section className="agent-form" aria-label={t.groups.history}><h3>{t.groups.history}</h3>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    {storage&&(storage.parts>0?<p className="agent-muted">{t.groups.storageParts(storage.parts)} · {t.groups.storageStamps(storage.stampsPerMonth)}</p>:<p className="agent-muted">{t.groups.noArchive}</p>)}
    <label>{t.groups.keep}<select value={String(kept)} disabled={busy} onChange={e=>setKept(e.target.value==='forever'?'forever':Number(e.target.value) as Retention)}>
      {RETENTIONS.map(days=><option key={String(days)} value={String(days)}>{days==='forever'?t.groups.forever:t.groups.days(days)}</option>)}
    </select></label>
    {kept==='forever'&&<p className="network-validation">{t.groups.foreverWarning}</p>}
    <button disabled={busy||kept===current} onClick={()=>void change({retention:kept})}>{t.groups.save}</button>
  </section>;
}

/** Keys of a closed channel, given to one subscriber at a time. */
function ChannelSubscribers({api,groupId,busy,change}:{api:DesktopApi;groupId:string;busy:boolean;change:(what:Change)=>Promise<boolean|undefined>}) {
  const t=useT();const describe=useDescribe();
  const [id,setId]=useState('');const [error,setError]=useState('');const [giving,setGiving]=useState(false);
  const pending=useRef<{key:string;operationId:string}|null>(null);
  const candidate=id.trim();const valid=validNetworkId(candidate);
  async function give() {
    if(!valid||giving)return;
    if(pending.current?.key!==candidate)pending.current={key:candidate,operationId:crypto.randomUUID()};
    setGiving(true);setError('');
    try {await api.channelSubscribe({groupId,members:[candidate],operationId:pending.current.operationId});pending.current=null;setId('');}
    catch(e){setError(describe(e));}
    finally {setGiving(false);}
  }
  return <section className="agent-form" aria-label={t.groups.subscribers}><h3>{t.groups.subscribers}</h3>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    <label>{t.groups.subscriberId}<input value={id} disabled={busy||giving} spellCheck={false} placeholder="ain1…" onChange={e=>setId(e.target.value)}/></label>
    {candidate&&!valid&&<p className="network-validation">{t.common.idHint}</p>}
    <div className="group-add-actions">
      <button disabled={busy||giving||!valid} onClick={()=>void give()}>{t.groups.giveKeys}</button>
      <button className="secondary" disabled={busy||giving||!valid} onClick={()=>void change({unsubscribe:[candidate]}).then(ok=>{if(ok)setId('');})}>{t.groups.takeKeys}</button>
    </div>
    <button className="secondary" disabled={busy||giving} onClick={()=>void change({reseed:true})}>{t.groups.reseed}</button>
    <small>{t.groups.subscribersHelp}</small>
  </section>;
}

export function GroupPanel({api,groupId,ownId,onBack}:{api:DesktopApi;groupId:string;ownId:string;onBack:()=>void}) {
  const t=useT();const describe=useDescribe();
  const [group,setGroup]=useState<Group>();const [loadError,setLoadError]=useState('');
  const [member,setMember]=useState('');const [admins,setAdmins]=useState<string[]>();
  const [confirmOpen,setConfirmOpen]=useState(false);
  const load=useCallback(async()=>{
    try {const current=await api.group({groupId});setGroup(current);setAdmins(current.admins);setLoadError('');}
    catch(e){setLoadError(describe(e));}
  },[api,groupId,describe]);
  useEffect(()=>{void load();const stop=api.subscribe(()=>{void load();});return stop;},[api,load]);
  const {busy,error,change}=useGroupChange(api,groupId,load);
  if(!group)return <section className="agent-panel" aria-label={t.groups.panel}>{loadError?<div className="error-banner" role="alert">{loadError}</div>:<p className="agent-muted">{t.groups.loading}</p>}<button className="secondary" onClick={onBack}>{t.common.back}</button></section>;
  const rights=groupRights(group);
  const channel=group.kind==='channel';const team=group.role!=='member';
  const addTitle=!channel?t.groups.addTitle:rights.canAdd?t.groups.teamAddTitle:t.groups.banTitle;
  const closedChannel=channel&&group.access!=='public';
  const candidate=member.trim();
  const candidateValid=validNetworkId(candidate)&&!group.members.includes(candidate);
  const banValid=validNetworkId(candidate)&&candidate!==ownId&&rights.canBan(candidate)&&!group.banned.some(ban=>ban.id===candidate);
  const adminsChanged=admins!==undefined&&[...admins].sort().join()!==[...group.admins].sort().join();
  return <section className="agent-panel" aria-label={t.groups.panel}>
    <header className="agent-header"><div><span className="eyebrow">{channel?t.groups.channelEyebrow(group.epoch):t.groups.eyebrow(group.epoch)}</span><h2 dir="auto">{group.name}</h2><p>{channel?t.groups.channelText(t.roles[group.role]):t.groups.text(t.roles[group.role])}</p></div><button className="secondary" disabled={busy} onClick={onBack}>{t.common.back}</button></header>
    {(error||loadError)&&<div className="error-banner" role="alert">{error||loadError}</div>}
    <div className="agent-columns group-columns">
      <section className="agent-form" aria-label={t.groups.membersLabel}><h3>{channel?t.groups.team(group.members.length):t.groups.members(group.members.length)}</h3>
        <ul className="member-list">{group.members.map(id=>{
          const role=roleOf(group,id);
          return <li key={id} className="member-row">
            <span className="member-id" title={id}>{id===ownId?t.groups.you:shortId(id)}</span>
            <span className={`role-badge role-${role}`}>{t.roles[role]}</span>
            {rights.canSetAdmins&&!channel&&role!=='owner'&&<label className="check-row admin-toggle"><input type="checkbox" aria-label={t.groups.adminLabel(shortId(id))} checked={admins?.includes(id)??false} disabled={busy} onChange={e=>setAdmins(list=>e.target.checked?[...(list??[]),id]:(list??[]).filter(a=>a!==id))}/>{t.groups.admin}</label>}
            {rights.canRemove(id)&&id!==ownId&&<button className="secondary member-remove" disabled={busy} aria-label={t.groups.removeLabel(shortId(id))} onClick={()=>void change({remove:[id]})}>{t.groups.remove}</button>}
            {rights.canBan(id)&&id!==ownId&&<button className="secondary member-remove" disabled={busy} aria-label={t.groups.banLabel(shortId(id))} onClick={()=>void change({ban:[id]})}>{t.groups.ban}</button>}
          </li>;
        })}</ul>
        {rights.canSetAdmins&&!channel&&<button disabled={busy||!adminsChanged} onClick={()=>void change({admins:admins??[]})}>{t.groups.saveAdmins}</button>}
        {group.banned.length>0&&<><h3>{t.groups.banned(group.banned.length)}</h3>
          <ul className="member-list" aria-label={t.groups.bannedLabel}>{group.banned.map(ban=><li key={ban.id} className="member-row">
            <span className="member-id" title={ban.id}>{shortId(ban.id)}</span>
            <span className="role-badge">{ban.byOwner?t.groups.byOwner:t.groups.byAdmin}</span>
            {rights.canUnban(ban)&&<button className="secondary member-remove" disabled={busy} aria-label={t.groups.unbanLabel(shortId(ban.id))} onClick={()=>void change({unban:[ban.id]})}>{t.groups.unban}</button>}
          </li>)}</ul></>}
      </section>
      <div className="panel-stack">
      <section className="agent-form" aria-label={t.groups.readingTitle}><h3>{t.groups.readingTitle}</h3>
        <p className={group.access==='public'?'claim-outcome':'agent-muted'}>{readingText(t,group)}</p>
        {group.role==='owner'&&!channel&&<>
          {group.access!=='public'&&<><label className="check-row"><input type="checkbox" checked={confirmOpen} disabled={busy} onChange={e=>setConfirmOpen(e.target.checked)}/>{t.groups.openConfirm}</label>
          <button disabled={busy||!confirmOpen} onClick={()=>void change({access:'public'}).then(ok=>{if(ok)setConfirmOpen(false);})}>{t.groups.open}</button></>}
          {group.access!=='request'&&<button className="secondary" disabled={busy} onClick={()=>void change({access:'request'})}>{t.groups.byRequest}</button>}
          {group.access!=='private'&&<button className="secondary" disabled={busy} onClick={()=>void change({access:'private'})}>{t.groups.close}</button>}
        </>}
        {group.role==='owner'&&closedChannel&&(group.access==='private'
          ?<button className="secondary" disabled={busy} onClick={()=>void change({access:'request'})}>{t.groups.byRequest}</button>
          :<button className="secondary" disabled={busy} onClick={()=>void change({access:'private'})}>{t.groups.keysOnly}</button>)}
      </section>
      {team&&group.access==='request'&&<DoorRequests api={api} groupId={group.id}/>}
      {team&&channel&&group.access==='public'&&<ChannelHistory api={api} group={group} busy={busy} change={change}/>}
      {team&&closedChannel&&<ChannelSubscribers api={api} groupId={group.id} busy={busy} change={change}/>}
      {group.role==='owner'&&group.access==='public'&&<CardForm api={api} draft={{kind:'group',groupId:group.id}} title={channel?t.groups.channelCardTitle:t.groups.cardTitle} text={channel?t.groups.channelCardText:t.groups.cardText}/>}
      {rights.canBanById?<form className="agent-form" aria-label={addTitle} onSubmit={e=>{e.preventDefault();if(rights.canAdd)void change({add:[candidate]}).then(ok=>{if(ok)setMember('');});}}>
        <h3>{addTitle}</h3>
        <label>{channel?t.groups.networkId:t.groups.memberId}<input id="group-member" value={member} disabled={busy} spellCheck={false} placeholder="ain1…" onChange={e=>setMember(e.target.value)}/></label>
        {candidate&&!validNetworkId(candidate)&&<p className="network-validation">{t.common.idHint}</p>}
        <div className="group-add-actions">
          {rights.canAdd&&<button disabled={busy||!candidateValid}>{busy?t.groups.confirming:t.groups.add}</button>}
          <button type="button" className="secondary" disabled={busy||!banValid} onClick={()=>void change({ban:[candidate]}).then(ok=>{if(ok)setMember('');})}>{t.groups.banId}</button>
        </div>
        {rights.canAdd&&<small>{channel?t.groups.teamAddHelp:t.groups.addHelp}</small>}
        <small>{channel?t.groups.channelBanHelp:t.groups.banHelp}</small>
      </form>:<section className="agent-form"><h3>{t.groups.rosterTitle}</h3><p className="agent-muted">{t.groups.rosterHelp}</p></section>}
      </div>
    </div>
  </section>;
}

/** What a group's readers are told of who reads it. */
function readingText(t:ReturnType<typeof useT>,group:Group) {
  if(group.kind==='channel')return group.access==='public'?t.groups.channelPublic:group.access==='request'?t.groups.channelRequest:t.groups.channelPrivate;
  return group.access==='public'?t.groups.readingPublic:group.access==='request'?t.groups.readingRequest:t.groups.readingPrivate;
}

export function NewGroupPanel({api,onCreated,onBack}:{api:DesktopApi;onCreated:(groupId:string)=>void;onBack:()=>void}) {
  const t=useT();const describe=useDescribe();
  const [kind,setKind]=useState<GroupKind>('group');const [access,setAccess]=useState<GroupAccess>('private');
  const [confirmPublic,setConfirmPublic]=useState(false);
  const [name,setName]=useState('');const [members,setMembers]=useState('');
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const pending=useRef<{key:string;operationId:string}|null>(null);
  const ids=members.split(/\s+/).map(id=>id.trim()).filter(Boolean);
  const valid=ids.every(validNetworkId)&&new Set(ids).size===ids.length;
  const channel=kind==='channel';
  const ready=!busy&&!!name.trim()&&valid&&(!channel||access!=='public'||confirmPublic);
  async function create() {
    if(!ready)return;
    const request=channel?{name:name.trim(),members:ids,kind,access}:{name:name.trim(),members:ids};
    const key=JSON.stringify(request);
    if(pending.current?.key!==key)pending.current={key,operationId:crypto.randomUUID()};
    setBusy(true);setError('');
    try {const made=await api.createGroup({...request,operationId:pending.current.operationId});pending.current=null;onCreated(made.id);}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  return <section className="empty onboarding" aria-label={t.groups.newLabel}><span className="eyebrow">{t.groups.newEyebrow}</span><h2>{channel?t.groups.newChannelTitle:t.groups.newTitle}</h2><p>{channel?t.groups.newChannelText:t.groups.newText}</p>
    {error&&<div className="error-banner inline-error" role="alert">{error}</div>}
    <fieldset className="choice-row" disabled={busy}><legend>{t.groups.kind}</legend>
      <label className="check-row"><input type="radio" name="group-kind" checked={!channel} onChange={()=>setKind('group')}/>{t.groups.kindGroup}</label>
      <label className="check-row"><input type="radio" name="group-kind" checked={channel} onChange={()=>setKind('channel')}/>{t.groups.kindChannel}</label>
    </fieldset>
    <label>{channel?t.groups.channelName:t.groups.name}<input id="group-name" value={name} disabled={busy} maxLength={80} onChange={e=>setName(e.target.value)}/></label>
    <label>{channel?t.groups.teamIds:t.groups.memberIds}<textarea id="group-members" rows={4} spellCheck={false} value={members} disabled={busy} onChange={e=>setMembers(e.target.value)}/></label>
    {!valid&&<p className="network-validation">{t.groups.idsInvalid}</p>}
    {channel&&<fieldset className="choice-row" disabled={busy}><legend>{t.groups.readingTitle}</legend>
      {(['public','request','private'] as GroupAccess[]).map(option=><label key={option} className="check-row"><input type="radio" name="channel-access" checked={access===option} onChange={()=>{setAccess(option);setConfirmPublic(false);}}/>{option==='public'?t.groups.accessPublic:option==='request'?t.groups.accessRequest:t.groups.accessPrivate}</label>)}
      {access==='public'&&<label className="check-row"><input type="checkbox" checked={confirmPublic} onChange={e=>setConfirmPublic(e.target.checked)}/>{t.groups.openConfirm}</label>}
    </fieldset>}
    <div className="actions"><button className="secondary" disabled={busy} onClick={onBack}>{t.common.cancel}</button><button disabled={!ready} onClick={()=>void create()}>{busy?(channel?t.groups.creatingChannel:t.groups.creating):(channel?t.groups.makeChannel:t.groups.create)}</button></div>
  </section>;
}
