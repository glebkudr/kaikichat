import {useCallback,useEffect,useState} from 'react';
import type {DesktopApi,IntroMode,IntroPolicy} from './types';
import {validNetworkId} from './group-roles';
import {useDescribe,useT} from './i18n';

const modes:IntroMode[]=['all','list','manual'];

/** Who may ask for a conversation by network id, kept by the node. */
export function IntroPolicyPanel({api}:{api:DesktopApi}) {
  const t=useT();const describe=useDescribe();
  const [policy,setPolicy]=useState<IntroPolicy>();const [draft,setDraft]=useState<IntroPolicy>();
  const [allowedText,setAllowedText]=useState('');
  const [error,setError]=useState('');const [notice,setNotice]=useState('');const [busy,setBusy]=useState(false);
  const load=useCallback(async()=>{
    try {
      const current=await api.introPolicy();
      setPolicy(current);setDraft(previous=>previous??current);setAllowedText(previous=>previous||current.allowed.join('\n'));
    } catch(e){setError(describe(e));}
  },[api,describe]);
  useEffect(()=>{void load();const stop=api.subscribe(()=>{void load();});return stop;},[api,load]);
  const allowed=allowedText.split(/\s+/).map(id=>id.trim()).filter(Boolean);
  const allowedValid=allowed.every(validNetworkId);
  const limitValid=draft!==undefined&&Number.isInteger(draft.dailyLimit)&&draft.dailyLimit>=0&&draft.dailyLimit<=1000;
  async function save() {
    if(busy||!draft)return;setBusy(true);setError('');setNotice('');
    try {
      const saved=await api.setIntroPolicy({...draft,allowed});
      setPolicy(saved);setDraft(saved);setAllowedText(saved.allowed.join('\n'));setNotice(t.introPolicy.saved);
    } catch(e){setError(describe(e));} finally {setBusy(false);}
  }
  return <form className="agent-form intro-policy" aria-label={t.introPolicy.title} onSubmit={e=>{e.preventDefault();void save();}}>
    <h3>{t.introPolicy.title}</h3>
    {error&&<div className="error-banner" role="alert">{error}</div>}
    {notice&&<p className="network-notice" role="status">{notice}</p>}
    {draft&&<>
      {modes.map(mode=><label className="check-row policy-mode" key={mode}><input type="radio" name="intro-mode" value={mode} checked={draft.mode===mode} disabled={busy} onChange={()=>{setDraft({...draft,mode});setNotice('');}}/><span><strong>{t.introPolicy.modes[mode].title}</strong><small>{t.introPolicy.modes[mode].help}</small></span></label>)}
      {draft.mode==='all'&&<label>{t.introPolicy.dailyLimit}<input type="number" min={0} max={1000} value={draft.dailyLimit} disabled={busy} onChange={e=>{setDraft({...draft,dailyLimit:Number(e.target.value)});setNotice('');}}/></label>}
      <label>{t.introPolicy.allowed}<textarea aria-label={t.introPolicy.allowedLabel} rows={3} spellCheck={false} value={allowedText} disabled={busy} onChange={e=>{setAllowedText(e.target.value);setNotice('');}}/></label>
      {!allowedValid&&<p className="network-validation">{t.introPolicy.allowedInvalid}</p>}
      <button disabled={busy||!allowedValid||!limitValid||(policy!==undefined&&JSON.stringify({...draft,allowed})===JSON.stringify(policy))}>{t.introPolicy.save}</button>
    </>}
  </form>;
}
