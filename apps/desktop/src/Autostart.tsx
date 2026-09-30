import {useEffect,useState} from 'react';
import {useDescribe,useT} from './i18n';
import type {Autostart,DesktopApi} from './types';

/** Whether the app opens when the owner logs in; when the system's login
 * items block it, a way to them to allow it. */
export function AutostartSection({api}:{api:DesktopApi}) {
  const t=useT();const describe=useDescribe();
  const [autostart,setAutostart]=useState<Autostart|null>(null);
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  useEffect(()=>{
    let open=true;
    api.autostart().then(next=>{if(open)setAutostart(next);}).catch(()=>{/* the window works without it */});
    return ()=>{open=false;};
  },[api]);
  if(!autostart)return null;
  async function change(on:boolean) {
    if(busy)return;setBusy(true);setError('');
    try {setAutostart(await api.setAutostart({on}));}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  async function allow() {
    setError('');
    try {await api.openLoginItems();} catch(e){setError(describe(e));}
  }
  return <section className="agent-form autostart-section" aria-label={t.autostart.title}>
    <h3>{t.autostart.title}</h3>
    <label className="check-row"><input type="checkbox" checked={autostart.state!=='off'} disabled={busy} onChange={e=>void change(e.target.checked)}/>{t.autostart.open}</label>
    {autostart.state==='blocked'&&<>
      <p className="network-help">{t.autostart.blocked}</p>
      <span className="preset-actions"><button className="secondary" onClick={()=>void allow()}>{t.autostart.allow}</button></span>
    </>}
    {error&&<p className="network-validation" role="alert">{error}</p>}
  </section>;
}
