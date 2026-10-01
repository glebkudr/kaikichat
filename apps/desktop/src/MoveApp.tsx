import {useEffect,useState} from 'react';
import {useDescribe,useT} from './i18n';
import type {DesktopApi} from './types';

/** An app opened outside the Applications folder (a download, a disk
 * image): one press moves it there and opens it from there, where it opens
 * at login and its command line stays. */
export function MoveNotice({api}:{api:DesktopApi}) {
  const t=useT();const describe=useDescribe();
  const [offered,setOffered]=useState(false);
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');
  useEffect(()=>{
    let open=true;
    api.moveOffer().then(next=>{if(open)setOffered(next);},()=>{/* the window works without it */});
    return ()=>{open=false;};
  },[api]);
  if(!offered)return null;
  /** The app quits and opens from its new place, so the button stays busy. */
  async function move() {
    if(busy)return;setBusy(true);setError('');
    try {await api.moveToApplications();}
    catch(e){setError(describe(e));setBusy(false);}
  }
  return <section className="network-notice preset-notice move-notice" aria-label={t.move.notice}>
    <p>{t.move.text}</p>
    {error&&<p className="network-validation" role="alert">{error}</p>}
    <span className="preset-actions"><button disabled={busy} onClick={()=>void move()}>{busy?t.move.moving:t.move.go}</button></span>
  </section>;
}
