import {useEffect,useState} from 'react';
import type {Balance,DesktopApi,Identity} from './types';
import {AddFriend,AgentCard,InviteCard} from './Connect';
import {ProviderButtons} from './ProviderButtons';
import {ClaimOutcome,useClaim,useClaimOutcome} from './WalletPanel';
import {useT} from './i18n';

/** Free messages through a login, offered while none are left. */
function CoinsCard({api}:{api:DesktopApi}) {
  const t=useT();
  const [balance,setBalance]=useState<Balance>();
  const claim=useClaim(api,setBalance);
  useEffect(()=>{
    const load=()=>{void api.coinsBalance().then(setBalance,()=>{});};
    load();return api.subscribe(load);
  },[api]);
  useEffect(()=>{
    if(!balance?.claim)return;
    const timer=setInterval(()=>{void api.coinsBalance().then(setBalance,()=>{});},3000);
    return ()=>clearInterval(timer);
  },[api,balance?.claim]);
  const outcome=useClaimOutcome(balance);
  if(!balance||balance.remaining>0||balance.pending.length>0)return null;
  return <section className="agent-form connect-card" aria-label={t.home.coinsTitle}>
    <h3>{t.home.coinsTitle}</h3><p>{t.starter.text}</p>
    {claim.error&&<p className="network-validation" role="alert">{claim.error}</p>}
    {outcome.text&&<ClaimOutcome {...outcome}/>}
    <ProviderButtons busy={claim.busy} disabled={claim.code==='identity_not_configured'} onPick={provider=>void claim.claim(provider)}/>
  </section>;
}

/** Where an owner without conversations lands: the agent, friends in and
 * out, and free messages when none are left. */
export function StartHere({api,identity,onOpen,onGroup,onDiscover}:{api:DesktopApi;identity:Identity;onOpen:(conversationId:string)=>void;onGroup:()=>void;onDiscover:()=>void}) {
  const t=useT();
  return <section className="start" aria-label={t.home.title}>
    <span className="eyebrow">{t.home.eyebrow}</span><h2>{t.home.title}</h2><p>{t.home.text}</p>
    <div className="start-cards">
      <CoinsCard api={api}/>
      <AgentCard api={api}/>
      <InviteCard identity={identity}/>
      <AddFriend api={api} identity={identity} onOpen={onOpen}/>
    </div>
    <div className="actions start-actions">
      <button type="button" className="secondary start-group" onClick={onDiscover}>{t.home.discover}</button>
      <button type="button" className="secondary start-group" onClick={onGroup}>{t.home.group}</button>
    </div>
  </section>;
}
