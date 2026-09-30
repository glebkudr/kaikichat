import {useCallback,useEffect,useState} from 'react';
import {useDescribe,useT} from './i18n';
import type {DesktopApi,NetworkPreset} from './types';

/** The profile's network as last checked, and a way to check it again. */
function usePreset(api:DesktopApi,onChanged?:()=>void) {
  const describe=useDescribe();
  const [preset,setPreset]=useState<NetworkPreset>();
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  const load=useCallback(async()=>{try{setPreset(await api.networkPreset());}catch{/* the window works without it */}},[api]);
  useEffect(()=>{void load();},[load]);
  async function refresh(move:boolean) {
    if(busy)return;setBusy(true);setError('');
    try {setPreset(await api.refreshNetwork({switch:move}));onChanged?.();}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  return {preset,busy,error,refresh};
}

/** What the owner must know or decide about the network: settings not
 * fetched, another network offered, or a newer app needed. */
export function NetworkNotice({api,onChanged}:{api:DesktopApi;onChanged?:()=>void}) {
  const t=useT();const {preset,busy,error,refresh}=usePreset(api,onChanged);
  // Moving is one way: the owner confirms it after reading why.
  const [confirming,setConfirming]=useState(false);
  if(!preset||preset.source!=='preset')return null;
  const offer=preset.state!=='update'?preset.offered:null;
  const text=preset.state==='update'&&preset.required?t.preset.update(preset.required):offer?t.preset.offered(offer.name):preset.state==='unavailable'?t.preset.unavailable:'';
  if(!text)return null;
  return <section className="network-notice preset-notice" aria-label={t.preset.notice}>
    <p dir="auto">{offer&&confirming?t.preset.confirmSwitch(offer.name):text}</p>
    {error&&<p className="network-validation" role="alert">{error}</p>}
    {offer?confirming?<span className="preset-actions"><button className="secondary" disabled={busy} onClick={()=>setConfirming(false)}>{t.common.cancel}</button><button disabled={busy} onClick={()=>void refresh(true)}>{t.preset.confirm}</button></span>:
    <button disabled={busy} onClick={()=>setConfirming(true)}>{t.preset.switch}</button>:
    preset.state==='unavailable'&&<button className="secondary" disabled={busy} onClick={()=>void refresh(false)}>{busy?t.preset.checking:t.preset.retry}</button>}
  </section>;
}

/** The network in the settings: its name, where it comes from, when it was
 * last checked. */
export function NetworkPresetSection({api}:{api:DesktopApi}) {
  const t=useT();const {preset,busy,error,refresh}=usePreset(api);
  if(!preset)return null;
  const source=preset.source==='manual'?t.preset.manual:preset.source==='off'?t.preset.off:t.preset.source;
  return <section className="agent-form preset-section" aria-label={t.preset.title}>
    <h3>{t.preset.title}</h3>
    <div className="setting-row"><strong dir="auto">{preset.name??t.preset.none}</strong><span>{source}</span></div>
    {preset.checkedAt!==null&&<small>{t.preset.checked(new Date(preset.checkedAt*1000).toLocaleString(t.tag))}</small>}
    {preset.state==='cached'&&<p className="network-help">{t.preset.stale}</p>}
    {error&&<p className="network-validation" role="alert">{error}</p>}
    {preset.source==='preset'&&<button className="secondary" disabled={busy} onClick={()=>void refresh(false)}>{busy?t.preset.checking:t.preset.check}</button>}
  </section>;
}
