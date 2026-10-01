import {useCallback,useEffect,useRef,useState} from 'react';
import type {DesktopApi,Identity,OwnerCli} from './types';
import {shortId} from './group-roles';
import {locales,useDescribe,useT} from './i18n';

/** One word for a POSIX shell. Paths are always quoted, so the full stop
 * that may follow one in a sentence is not read as part of it. */
export const shellWord=(word:string)=>/^[A-Za-z0-9_.,:=@%+-]+$/.test(word)?word:`'${word.replaceAll("'","'\\''")}'`;

const escape=(text:string)=>text.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
/** An id on its own: not a part of a longer word or hex string. */
const idPattern='(?<![0-9A-Za-z])ain1[0-9a-fA-F]{64}(?![0-9A-Za-z])';
/** Direction marks that messengers put around text. */
const marks=/[\u200e\u200f\u202a-\u202e\u2066-\u2069]/g;
/** The friend's id and name from an invitation in any of the app's
 * languages (`…, name: NAME, ID: ain1…`), or a bare id; null when the text
 * holds no id. */
export function parseInvitation(pasted:string):{networkId:string;name:string}|null {
  const text=pasted.replace(marks,'');
  for(const table of Object.values(locales)) {
    const line=new RegExp(`,\\s*${escape(table.invite.nameLabel)}:\\s*(.+?),\\s*ID:\\s*(${idPattern})`,'iu').exec(text);
    if(line)return {networkId:line[2].toLowerCase(),name:Array.from(line[1].trim()).slice(0,80).join('')};
  }
  const id=new RegExp(idPattern).exec(text);
  return id?{networkId:id[0].toLowerCase(),name:''}:null;
}

/** A copy of the app that macOS runs from a disk image or a quarantine
 * folder: its path is gone once the app quits. */
export const unsettled=(command:string)=>command.includes('/AppTranslocation/')||/^\/Volumes\/[^/]+\/[^/]+\.app\//.test(command);

/** Copies a text; where the clipboard is closed, copies the selected box
 * the old way, or leaves the text selected for the owner's own ⌘C. */
export function useCopy() {
  const [state,setState]=useState<''|'copied'|'manual'>('');
  const box=useRef<HTMLTextAreaElement>(null);
  const copy=useCallback(async(text:string)=>{
    try {await navigator.clipboard.writeText(text);setState('copied');return;}
    catch { /* the selected box below */ }
    const shown=box.current;
    if(shown) {
      shown.closest('details')?.setAttribute('open','');
      shown.focus();shown.select();
      try {if(document.execCommand('copy')){setState('copied');return;}}
      catch { /* left selected */ }
    }
    setState('manual');
  },[]);
  return {state,copy,box};
}

/** The text the owner gives an agent: the CLI of this profile, as shell words. */
export function useAgentInstruction(api:DesktopApi) {
  const t=useT();const describe=useDescribe();
  const [cli,setCli]=useState<OwnerCli>();const [error,setError]=useState('');
  useEffect(()=>{
    let live=true;
    api.ownerCli().then(found=>{if(live)setCli(found);},e=>{if(live)setError(describe(e));});
    return ()=>{live=false;};
  },[api,describe]);
  return {text:cli?t.agentSetup.instruction([cli.command,...cli.args].map(shellWord).join(' ')):'',error,move:cli!==undefined&&unsettled(cli.command)};
}

const select=(e:{currentTarget:HTMLTextAreaElement})=>e.currentTarget.select();

/** A text to copy, with a short line that says what happened. */
function CopyBlock({label,text,placeholder,copyLabel,copied,manual,rows,folded,foldLabel}:{label:string;text:string;placeholder?:string;copyLabel:string;copied:string;manual:string;rows:number;folded?:boolean;foldLabel?:string}) {
  const copy=useCopy();
  const box=<label className="copy-text">{label}<textarea ref={copy.box} readOnly rows={rows} value={text||placeholder||''} onFocus={select}/></label>;
  return <>
    {folded?<details className="show-text"><summary>{foldLabel}</summary>{box}</details>:box}
    {copy.state&&<p className="claim-outcome" role="status">{copy.state==='copied'?copied:manual}</p>}
    <button type="button" className={copy.state?'secondary':''} disabled={!text} onClick={()=>void copy.copy(text)}>{copyLabel}</button>
  </>;
}

/** The agent's instructions in a card: the start screen and the agents screen. */
export function AgentCard({api}:{api:DesktopApi}) {
  const t=useT();const {text,error,move}=useAgentInstruction(api);
  return <section className="agent-form connect-card" aria-label={t.agentSetup.title}>
    <h3>{t.agentSetup.title}</h3><p>{t.home.agentText}</p>
    {error&&<p className="network-validation" role="alert">{error}</p>}
    {move&&<p className="network-validation" role="alert">{t.agentSetup.moveApp}</p>}
    <CopyBlock label={t.agentSetup.label} text={text} placeholder={t.agentSetup.loading} rows={10} copyLabel={t.agentSetup.copy} copied={t.agentSetup.copied} manual={t.agentSetup.copyManually} folded foldLabel={t.home.showText}/>
  </section>;
}

/** The invitation for friends in a card. */
export function InviteCard({identity}:{identity:Identity}) {
  const t=useT();
  return <section className="agent-form connect-card" aria-label={t.invite.title}>
    <h3>{t.invite.title}</h3><p>{t.home.inviteText}</p>
    <CopyBlock label={t.invite.label} text={t.invite.message(identity.name,identity.networkId)} rows={6} copyLabel={t.invite.copy} copied={t.invite.copied} manual={t.agentSetup.copyManually} folded foldLabel={t.home.showText}/>
  </section>;
}

/** The first screens' copy step: the text in full, then one button; once
 * copied, the way on, and the copy button stays for another copy. */
export function CopyStep({label,text,placeholder,rows,copyLabel,copied,manual,next,skip,onNext}:{label:string;text:string;placeholder?:string;rows:number;copyLabel:string;copied:string;manual:string;next:string;skip:string;onNext:()=>void}) {
  const copy=useCopy();
  return <>
    <label className="copy-text">{label}<textarea ref={copy.box} readOnly rows={rows} value={text||placeholder||''} onFocus={select}/></label>
    {copy.state&&<p className="claim-outcome" role="status">{copy.state==='copied'?copied:manual}</p>}
    <button type="button" className={copy.state?'secondary':''} disabled={!text} onClick={()=>void copy.copy(text)}>{copyLabel}</button>
    {copy.state?<button type="button" onClick={onNext}>{next}</button>:
      <div className="wizard-links"><button type="button" className="link" onClick={onNext}>{skip}</button></div>}
  </>;
}

/** A friend from the invitation they sent, or from a bare id. */
export function AddFriend({api,identity,onOpen}:{api:DesktopApi;identity:Identity;onOpen:(conversationId:string)=>void}) {
  const t=useT();const describe=useDescribe();
  const [text,setText]=useState('');const [name,setName]=useState('');const [named,setNamed]=useState(false);
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const asking=useRef<{networkId:string;name:string;operationId:string}|null>(null);
  const found=parseInvitation(text);
  const own=found?.networkId===identity.networkId;
  function edit(value:string) {
    setText(value);setError('');
    const next=parseInvitation(value);
    if(!named&&next?.name)setName(next.name);
  }
  async function add() {
    if(busy||!found||own||!name.trim())return;
    const wanted={networkId:found.networkId,name:name.trim()};
    // The same request keeps its operation id across retries.
    if(asking.current?.networkId!==wanted.networkId||asking.current.name!==wanted.name)asking.current={...wanted,operationId:crypto.randomUUID()};
    setBusy(true);setError('');
    try {
      // The native side waits while the node looks up the card.
      const made=await api.requestContact(asking.current);
      asking.current=null;setText('');setName('');setNamed(false);onOpen(made.conversationId);
    } catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  return <form className="agent-form connect-card" aria-label={t.addFriend.title} onSubmit={e=>{e.preventDefault();void add();}}>
    <h3>{t.addFriend.title}</h3><p>{t.addFriend.text}</p>
    {error&&<div className="error-banner inline-error" role="alert">{error}</div>}
    <label>{t.addFriend.paste}<textarea id="friend-invitation" rows={3} spellCheck={false} value={text} disabled={busy} onChange={e=>edit(e.target.value)}/></label>
    {text.trim()&&!found&&<p className="network-validation">{t.addFriend.noId}</p>}
    {own&&<p className="network-validation">{t.addFriend.own}</p>}
    {found&&!own&&<p className="agent-muted found-id" title={found.networkId}>{t.addFriend.found(shortId(found.networkId))}</p>}
    <label>{t.addFriend.name}<input id="friend-name" value={name} disabled={busy} maxLength={80} onChange={e=>{setName(e.target.value);setNamed(true);}}/></label>
    <button disabled={busy||!found||own||!name.trim()}>{busy?t.addFriend.searching:t.addFriend.add}</button>
    <small>{t.addFriend.help}</small>
  </form>;
}
