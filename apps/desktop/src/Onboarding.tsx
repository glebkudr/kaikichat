import {useEffect,useState,type ReactNode} from 'react';
import type {Balance,DesktopApi,Identity,ProfileStatus} from './types';
import {isCode} from './core-error';
import {LanguageSelect,useDescribe,useT} from './i18n';
import {ClaimOutcome,useClaim,useClaimOutcome} from './WalletPanel';
import {ProviderButtons} from './ProviderButtons';
import {CopyStep,useAgentInstruction} from './Connect';

/** A password-sealed profile: the password opens it (or seals a new one). */
export function UnlockScreen({api,status,onOpened}:{api:DesktopApi;status:ProfileStatus;onOpened:()=>void}) {
  const t=useT();const describe=useDescribe();
  const [password,setPassword]=useState('');const [repeat,setRepeat]=useState('');
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const fresh=status.newProfile===true;
  const ready=password.length>=8&&(!fresh||password===repeat);
  async function unlock() {
    if(busy||!ready)return;setBusy(true);setError('');
    try {await api.unlockProfile({password});setPassword('');setRepeat('');onOpened();}
    catch(e){setError(isCode(e,'secrets_locked')?t.unlock.wrong:describe(e));}
    finally {setBusy(false);}
  }
  return <form className="empty onboarding" aria-label={fresh?t.unlock.newTitle:t.unlock.lockedTitle} onSubmit={e=>{e.preventDefault();void unlock();}}>
    <span className="eyebrow">{fresh?t.unlock.newEyebrow:t.unlock.lockedEyebrow}</span>
    <h2>{fresh?t.unlock.newTitle:t.unlock.lockedTitle}</h2>
    <p>{fresh?t.unlock.newText:t.unlock.lockedText}</p>
    {error&&<div className="error-banner inline-error" role="alert">{error}</div>}
    <label>{t.unlock.password}<input type="password" autoComplete={fresh?'new-password':'current-password'} value={password} disabled={busy} onChange={e=>setPassword(e.target.value)}/></label>
    {fresh&&<label>{t.unlock.repeat}<input type="password" autoComplete="new-password" value={repeat} disabled={busy} onChange={e=>setRepeat(e.target.value)}/></label>}
    {fresh&&password.length>0&&password.length<8&&<p className="network-validation">{t.unlock.tooShort}</p>}
    <button disabled={busy||!ready}>{busy?t.unlock.opening:fresh?t.unlock.save:t.unlock.open}</button>
  </form>;
}

type Step='welcome'|'name'|'login'|'agent'|'invite';
const steps:Step[]=['welcome','name','login','agent','invite'];

/** The first run, one action per screen: what this is, a name, free
 * messages through a login, the agent's instructions, an invitation for
 * friends. Only the name is required. */
export function Onboarding({api,identity,notice,onCreated,onFinish,onTopUp}:{api:DesktopApi;identity:Identity|null;notice?:ReactNode;onCreated:()=>void;onFinish:()=>void;onTopUp:()=>void}) {
  const t=useT();
  const [step,setStep]=useState<Step>(identity?'login':'welcome');
  // A profile made meanwhile elsewhere (the CLI) needs no name from here.
  useEffect(()=>{if(identity&&(step==='welcome'||step==='name'))setStep('login');},[identity,step]);
  const at=steps.indexOf(step);
  const next=()=>{const following=steps[at+1];if(following)setStep(following);else onFinish();};
  return <div className="wizard">
    <header className="wizard-bar"><div className="brand"><span className="brand-mark" aria-hidden="true">k<span>·</span>c</span><div>Kaiki Chat<small>{t.shell.tagline}</small></div></div><LanguageSelect/></header>
    {notice&&<div className="wizard-notice">{notice}</div>}
    <div className="wizard-main">
      {step==='welcome'?<Welcome onNext={next}/>:
      step==='name'?<NameStep api={api} onCreated={()=>{onCreated();next();}}/>:
      step==='login'?<LoginStep api={api} onNext={next} onTopUp={onTopUp}/>:
      step==='agent'?<AgentStep api={api} onNext={next}/>:
      identity&&<InviteStep identity={identity} onNext={next}/>}
    </div>
    <footer className="wizard-progress"><span className="sr-only">{t.wizard.step(at+1,steps.length)}</span>{steps.map((s,i)=><span key={s} aria-hidden="true" className={i<=at?'done':''}/>)}</footer>
  </div>;
}

function Screen({title,text,eyebrow,label,children}:{title:string;text:string;eyebrow?:string;label?:string;children:ReactNode}) {
  return <section className="wizard-screen" aria-label={label??title}>
    {eyebrow&&<span className="eyebrow">{eyebrow}</span>}<h2>{title}</h2><p>{text}</p>{children}
  </section>;
}

function Welcome({onNext}:{onNext:()=>void}) {
  const t=useT();
  return <Screen eyebrow={t.welcome.eyebrow} title={t.welcome.title} text={t.welcome.text}>
    <div className="flow-diagram" aria-hidden="true"><span>{t.welcome.you}</span><i>→</i><span className="agent">{t.welcome.yourAgent}</span><i>⇄</i><span className="agent">{t.welcome.theirAgent}</span><i>←</i><span>{t.welcome.friend}</span></div>
    <button type="button" autoFocus onClick={onNext}>{t.welcome.start}</button>
  </Screen>;
}

/** The keys are made on this device and never leave it. */
function NameStep({api,onCreated}:{api:DesktopApi;onCreated:()=>void}) {
  const t=useT();const describe=useDescribe();
  const [name,setName]=useState('');const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  async function create() {
    if(busy||!name.trim())return;setBusy(true);setError('');
    try {await api.createIdentity({name:name.trim()});onCreated();}
    catch(e){setError(describe(e));setBusy(false);}
  }
  return <Screen title={t.create.title} text={t.create.text}>
    <form className="wizard-form" onSubmit={e=>{e.preventDefault();void create();}}>
      {error&&<div className="error-banner inline-error" role="alert">{error}</div>}
      <label>{t.create.name}<input autoFocus value={name} onChange={e=>setName(e.target.value)} maxLength={80} autoComplete="nickname" disabled={busy}/></label>
      <button disabled={busy||!name.trim()}>{t.create.submit}</button>
      <small className="wizard-note">{t.create.keys}</small>
    </form>
  </Screen>;
}

/** Free messages through a login in the browser; the app goes on once the
 * grant arrives. Crypto is the anonymous way, and the step can wait. */
function LoginStep({api,onNext,onTopUp}:{api:DesktopApi;onNext:()=>void;onTopUp:()=>void}) {
  const t=useT();
  const [balance,setBalance]=useState<Balance>();const [asked,setAsked]=useState(false);
  const claim=useClaim(api,setBalance);
  useEffect(()=>{
    const load=()=>{void api.coinsBalance().then(setBalance,()=>{});};
    load();return api.subscribe(load);
  },[api]);
  // The login is decided in the browser: look again meanwhile.
  useEffect(()=>{
    if(!balance?.claim)return;
    const timer=setInterval(()=>{void api.coinsBalance().then(setBalance,()=>{});},3000);
    return ()=>clearInterval(timer);
  },[api,balance?.claim]);
  const granted=asked&&!balance?.claim&&balance?.lastClaim?.status==='granted';
  // Once, when the grant arrives: a second call would skip the next step.
  useEffect(()=>{if(granted)onNext();},[granted]);
  const outcome=useClaimOutcome(balance);
  const unavailable=claim.code==='identity_not_configured';
  return <Screen title={t.starter.title} text={t.starter.text}>
    {claim.error&&<div className={unavailable?'network-notice':'error-banner inline-error'} role={unavailable?'status':'alert'}>{claim.error}</div>}
    {asked&&outcome.text&&<ClaimOutcome {...(balance?.claim?{text:t.starter.waiting,note:''}:outcome)}/>}
    <ProviderButtons busy={claim.busy} disabled={unavailable} onPick={provider=>{setAsked(true);void claim.claim(provider);}}/>
    <div className="wizard-links"><button type="button" className="link" onClick={onTopUp}>{t.starter.crypto}</button><button type="button" className="link" onClick={onNext}>{t.starter.skip}</button></div>
  </Screen>;
}

function AgentStep({api,onNext}:{api:DesktopApi;onNext:()=>void}) {
  const t=useT();const {text,error,move}=useAgentInstruction(api);
  return <Screen title={t.agentSetup.title} text={t.agentSetup.text}>
    {error&&<div className="error-banner inline-error" role="alert">{error}</div>}
    {move&&<div className="network-notice" role="alert">{t.agentSetup.moveApp}</div>}
    <CopyStep label={t.agentSetup.label} text={text} placeholder={t.agentSetup.loading} rows={12} copyLabel={t.agentSetup.copy} copied={t.agentSetup.copied} manual={t.agentSetup.copyManually} next={t.agentSetup.next} skip={t.agentSetup.later} onNext={onNext}/>
  </Screen>;
}

function InviteStep({identity,onNext}:{identity:Identity;onNext:()=>void}) {
  const t=useT();
  return <Screen title={t.invite.title} text={t.invite.text}>
    <CopyStep label={t.invite.label} text={t.invite.message(identity.name,identity.networkId)} rows={6} copyLabel={t.invite.copy} copied={t.invite.copied} manual={t.agentSetup.copyManually} next={t.invite.done} skip={t.invite.skip} onNext={onNext}/>
  </Screen>;
}
