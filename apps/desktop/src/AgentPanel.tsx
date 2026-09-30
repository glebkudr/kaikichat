import {useCallback,useEffect,useRef,useState} from 'react';
import type {Conversation,DesktopApi,ProvisionRuntimeRequest,RuntimeInfo,RuntimeSetup,RuntimeAction,SkillHost,SkillName} from './types';
import messagingSkill from '../../../integrations/agent-skill/agentic-messaging/SKILL.md?raw';
import {useDescribe,useT} from './i18n';
import {AgentCard} from './Connect';

const identifier=()=>Array.from(crypto.getRandomValues(new Uint8Array(32)));
const shellCommand=(args:string[])=>args.map(arg=>`'${arg.replaceAll("'","'\"'\"'")}'`).join(' ');
const hosts:Record<SkillHost,string>={claude:'Claude Code',codex:'Codex'};
const skills:SkillName[]=['agentic-messaging','kaiki'];

/** Installs a skill into an agent host's skills folder. */
function SkillInstall({api}:{api:DesktopApi}) {
  const t=useT();const describe=useDescribe();
  const [skill,setSkill]=useState<SkillName>('agentic-messaging');
  const [busy,setBusy]=useState(false);const [result,setResult]=useState('');const [error,setError]=useState('');
  async function install(host:SkillHost) {
    if(busy)return;setBusy(true);setError('');setResult('');
    try {const {path}=await api.installSkill({skill,host});setResult(t.agents.installed(hosts[host],path));}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  return <section className="agent-config" aria-label={t.agents.skillTitle}><h3>{t.agents.skillTitle}</h3><p>{t.agents.skillIntro}</p>
    {skills.map(name=><label className="check-row policy-mode" key={name}><input type="radio" name="skill" value={name} checked={skill===name} disabled={busy} onChange={()=>setSkill(name)}/><span><strong>{t.agents.skills[name].title}</strong><small>{t.agents.skills[name].help}</small></span></label>)}
    <div className="actions skill-actions">{(Object.keys(hosts) as SkillHost[]).map(host=><button key={host} type="button" className="secondary" disabled={busy} onClick={()=>void install(host)}>{t.agents.installIn(hosts[host])}</button>)}</div>
    {result&&<p className="claim-outcome" role="status">{result}</p>}
    {error&&<p className="network-validation" role="alert">{error}</p>}
  </section>;
}

export function AgentPanel({api,conversations,onBack}:{api:DesktopApi;conversations:Conversation[];onBack:()=>void}) {
  const t=useT();const describe=useDescribe();
  const [runtimes,setRuntimes]=useState<RuntimeInfo[]>([]);
  const [loading,setLoading]=useState(true);const [error,setError]=useState('');
  const [name,setName]=useState('');const [selected,setSelected]=useState<string[]>([]);
  const [allowSend,setAllowSend]=useState(false);const [lifetime,setLifetime]=useState(86400);
  const [maxBytes,setMaxBytes]=useState(4096);const [busy,setBusy]=useState(false);
  const [setup,setSetup]=useState<RuntimeSetup>();
  const pending=useRef<ProvisionRuntimeRequest|undefined>(undefined);
  const revision=useRef(0);
  const reload=useCallback(async()=>{
    const current=++revision.current;
    try {const rows=await api.listRuntimes();if(current===revision.current)setRuntimes(rows);}
    catch(e){if(current===revision.current)setError(describe(e));}
    finally {if(current===revision.current)setLoading(false);}
  },[api,describe]);
  useEffect(()=>{void reload();const stop=api.subscribe(()=>{void reload();});return ()=>{++revision.current;stop();};},[api,reload]);
  function edited(){pending.current=undefined;setSetup(undefined);setError('');}
  async function provision() {
    if(busy||!name.trim()||!selected.length||!Number.isInteger(maxBytes)||maxBytes<1||maxBytes>48000)return;
    const actions:RuntimeAction[]=['read_inbox'];
    if(allowSend)actions.push('send_message');
    const request=pending.current??{operationId:crypto.randomUUID(),name:name.trim(),agentId:identifier(),serviceId:identifier(),conversationIds:[...selected],actions,expiresAt:Math.floor(Date.now()/1000)+lifetime,maxDataBytes:maxBytes};
    // Save all approved fields, including expiry and identities, before the first attempt.
    pending.current={...request,actions:[...request.actions]};setBusy(true);setError('');setSetup(undefined);
    try {setSetup(await api.provisionRuntime(pending.current));}
    catch(e){setError(describe(e));}
    finally {await reload();setBusy(false);}
  }
  async function revoke(runtime:RuntimeInfo) {
    if(busy)return;setBusy(true);setError('');
    try {
      await api.revokeRuntime({grantId:runtime.grantId});
      if(setup?.runtime.grantId.join(',')===runtime.grantId.join(',')){setSetup(undefined);pending.current=undefined;}
    }catch(e){setError(describe(e));}
    finally{await reload();setBusy(false);}
  }
  return <section className="agent-panel" aria-label={t.agents.label}>
    <header className="agent-header"><div><span className="eyebrow">{t.agents.eyebrow}</span><h2>{t.agents.title}</h2><p>{t.agents.text}</p></div><button className="secondary" disabled={busy} onClick={onBack}>{t.common.back}</button></header>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    <div className="agent-connect"><AgentCard api={api}/></div>
    <div className="agent-columns">
      <form className="agent-form" onSubmit={e=>{e.preventDefault();void provision();}}>
        <h3>{t.agents.connect}</h3>
        <label>{t.agents.name}<input id="agent-name" value={name} disabled={busy} maxLength={80} onChange={e=>{edited();setName(e.target.value);}} placeholder={t.agents.namePlaceholder}/></label>
        <fieldset disabled={busy}><legend>{t.agents.conversations}</legend><p>{t.agents.conversationsHelp}</p>{conversations.length?conversations.map(c=><label className="check-row" key={c.id}><input type="checkbox" value={c.id} checked={selected.includes(c.id)} onChange={e=>{edited();setSelected(ids=>e.target.checked?[...ids,c.id]:ids.filter(id=>id!==c.id));}}/>{c.title}</label>):<p>{t.agents.addContactFirst}</p>}</fieldset>
        <label className="check-row"><input id="agent-allow-send" type="checkbox" disabled={busy} checked={allowSend} onChange={e=>{edited();setAllowSend(e.target.checked);}}/>{t.agents.allowSend}</label>
        <div className="agent-limits"><label>{t.agents.lifetime}<select disabled={busy} value={lifetime} onChange={e=>{edited();setLifetime(Number(e.target.value));}}><option value={3600}>{t.agents.hour}</option><option value={86400}>{t.agents.day}</option><option value={604800}>{t.agents.week}</option></select></label><label>{t.agents.maxBytes}<input type="number" disabled={busy} value={maxBytes} min={1} max={48000} onChange={e=>{edited();setMaxBytes(Number(e.target.value));}}/></label></div>
        <button disabled={busy||!name.trim()||!selected.length||!Number.isInteger(maxBytes)||maxBytes<1||maxBytes>48000}>{t.agents.grant}</button>
        <small>{t.agents.keyHelp}</small>
      </form>
      <div className="agent-records">
        {setup&&<section className="agent-config"><h3>{t.agents.ready}</h3><p>{t.agents.readyText}</p><label>{t.agents.mcpConfig}<textarea aria-label={t.agents.mcpConfig} rows={9} value={JSON.stringify(setup.mcpConfig,null,2)} readOnly onFocus={e=>e.currentTarget.select()}/></label><label>{t.agents.cliCommand}<textarea className="agent-cli-command" aria-label={t.agents.cliCommand} rows={4} value={shellCommand([setup.cliConfig.command,...setup.cliConfig.args,'context'])} readOnly onFocus={e=>e.currentTarget.select()}/></label><small>{t.agents.cliHelp}</small><label>{t.agents.credentials}<input aria-label={t.agents.credentials} readOnly value={setup.credentialsPath} onFocus={e=>e.currentTarget.select()}/></label><small>{t.agents.credentialsHelp}</small></section>}
        {setup&&<details className="agent-config"><summary>{t.agents.skillText}</summary><p>{t.agents.skillTextHelp}</p><label>{t.agents.skillLabel}<textarea aria-label={t.agents.skillLabel} rows={8} value={messagingSkill} readOnly onFocus={e=>e.currentTarget.select()}/></label></details>}
        <SkillInstall api={api}/>
        <h3>{t.agents.grants}</h3>
        {loading?<p className="agent-muted">{t.agents.loading}</p>:runtimes.length===0?<p className="agent-muted">{t.agents.none}</p>:runtimes.map(runtime=><article className="agent-card" key={runtime.grantId.join(',')}>
          <div className="agent-card-heading"><strong>{runtime.name}</strong><span className={`agent-status ${runtime.status}`}>{t.agents.status[runtime.status]}</span></div>
          <p>{runtime.conversationIds.map(id=>conversations.find(c=>c.id===id)?.title??id).join(', ')}</p>
          <p>{runtime.actions.map(action=>t.agents.actions[action]).join(' · ')}</p>
          <small>{t.agents.until(new Date(runtime.expiresAt*1000).toLocaleString(t.tag),runtime.maxDataBytes.toLocaleString(t.tag))}</small>
          {runtime.status==='active'&&<button className="secondary" disabled={busy} aria-label={t.agents.revokeLabel(runtime.name)} onClick={()=>void revoke(runtime)}>{t.agents.revoke}</button>}
        </article>)}
        <p className="agent-muted">{t.agents.runningNote}</p>
      </div>
    </div>
  </section>;
}
