import {useCallback,useEffect,useState} from 'react';
import type {DesktopApi,Identity,IntroRequest} from './types';
import {shortId} from './group-roles';
import {LocalTime} from './LocalTime';
import {useDescribe,useT} from './i18n';
import {AddFriend,InviteCard} from './Connect';

export function ContactsPanel({api,identity,onOpen,onBack}:{api:DesktopApi;identity:Identity;onOpen:(conversationId:string)=>void;onBack:()=>void}) {
  const t=useT();const describe=useDescribe();
  const [requests,setRequests]=useState<IntroRequest[]>([]);
  const [error,setError]=useState('');const [notice,setNotice]=useState('');const [busy,setBusy]=useState(false);
  const [invitation,setInvitation]=useState('');const [invitationName,setInvitationName]=useState('');
  const [ownInvitation,setOwnInvitation]=useState('');
  const load=useCallback(async()=>{
    try {
      setRequests(await api.introRequests());
    } catch(e){setError(describe(e));}
  },[api,describe]);
  useEffect(()=>{void load();const stop=api.subscribe(()=>{void load();});return stop;},[api,load]);
  async function act(work:()=>Promise<void>) {
    if(busy)return;setBusy(true);setError('');setNotice('');
    try {await work();} catch(e){setError(describe(e));} finally {setBusy(false);}
  }
  const decide=(item:IntroRequest,accept:boolean)=>act(async()=>{
    if(accept){const joined=await api.acceptIntroRequest({requestId:item.requestId});await load();onOpen(joined.conversationId);}
    else {await api.rejectIntroRequest({requestId:item.requestId});setNotice(t.contacts.rejected(item.name));await load();}
  });
  const addByInvitation=()=>act(async()=>{
    await api.addContact({name:invitationName.trim(),invitation:invitation.trim()});
    setInvitation('');setInvitationName('');setNotice(t.contacts.added);
  });
  const shareInvitation=()=>act(async()=>{setOwnInvitation(await api.createInvitation());});
  return <section className="agent-panel" aria-label={t.contacts.label}>
    <header className="agent-header"><div><span className="eyebrow">{t.contacts.eyebrow}</span><h2>{t.contacts.title}</h2><p>{t.contacts.text}</p></div><button className="secondary" disabled={busy} onClick={onBack}>{t.common.back}</button></header>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    {notice&&<div className="network-notice" role="status">{notice}</div>}
    <div className="agent-columns">
      <div className="panel-stack">
        <InviteCard identity={identity}/>
        <AddFriend api={api} identity={identity} onOpen={onOpen}/>
      </div>
      <div className="panel-stack">
        <section className="agent-form"><h3>{t.contacts.waiting}</h3>
          {requests.length===0?<p className="agent-muted">{t.contacts.none}</p>:requests.map(item=><article className="agent-card request-card" key={item.requestId}>
            <div className="agent-card-heading"><strong>{item.name}</strong><span className="agent-status">{item.group?t.contacts.groupInvitation:t.contacts.conversationRequest}</span></div>
            <p title={item.networkId}>{shortId(item.networkId)}</p><small><LocalTime value={item.receivedAt}/></small>
            <div className="actions"><button className="secondary" disabled={busy} aria-label={t.contacts.rejectLabel(item.name)} onClick={()=>void decide(item,false)}>{t.contacts.reject}</button><button disabled={busy} aria-label={t.contacts.acceptLabel(item.name)} onClick={()=>void decide(item,true)}>{t.contacts.accept}</button></div>
          </article>)}
        </section>
        <details className="agent-config"><summary>{t.contacts.invitationTitle}</summary>
          <label>{t.contacts.myIdHelp}<textarea aria-label={t.contacts.myId} className="invitation-code" rows={2} readOnly value={identity.networkId} onFocus={e=>e.currentTarget.select()}/></label><button type="button" className="secondary" onClick={()=>void navigator.clipboard?.writeText(identity.networkId).then(()=>setNotice(t.contacts.copied),()=>setNotice(t.contacts.copyManually))}>{t.contacts.copyId}</button>
          <p>{t.contacts.invitationText}</p>
          {ownInvitation?<label>{t.contacts.yourInvitation}<textarea aria-label={t.contacts.yourInvitation} className="invitation-code" value={ownInvitation} readOnly rows={4} onFocus={e=>e.currentTarget.select()}/></label>:<button type="button" className="secondary" disabled={busy} onClick={()=>void shareInvitation()}>{t.contacts.createInvitation}</button>}
          <label>{t.contacts.contactName}<input id="invitation-name" value={invitationName} disabled={busy} maxLength={80} onChange={e=>setInvitationName(e.target.value)}/></label>
          <label>{t.contacts.receivedInvitation}<textarea id="invitation-code" rows={3} value={invitation} disabled={busy} onChange={e=>setInvitation(e.target.value)}/></label>
          <button type="button" disabled={busy||!invitationName.trim()||!invitation.trim()} onClick={()=>void addByInvitation()}>{t.contacts.saveContact}</button>
        </details>
      </div>
    </div>
  </section>;
}
