import {useEffect,useState} from 'react';
import {useDescribe,useT} from './i18n';
import type {DesktopApi,Release} from './types';

/** How often the window looks at the latest release; the app itself asks
 * kaikichat.com at most every twelve hours. */
const LOOK_EVERY=60*60*1000;

/** The latest release as the app knows it, and what the owner does with it. */
function useRelease(api:DesktopApi) {
  const describe=useDescribe();
  const [release,setRelease]=useState<Release>();
  const [busy,setBusy]=useState<''|'check'|'skip'|'install'>('');
  const [error,setError]=useState('');
  useEffect(()=>{
    let open=true;
    const look=async()=>{try{const next=await api.release();if(open)setRelease(next);}catch{/* the window works without it */}};
    void look();
    const timer=setInterval(()=>void look(),LOOK_EVERY);
    return ()=>{open=false;clearInterval(timer);};
  },[api]);
  async function act(kind:'check'|'skip',run:()=>Promise<Release>) {
    if(busy)return;setBusy(kind);setError('');
    try {setRelease(await run());}
    catch(e){setError(describe(e));}
    finally {setBusy('');}
  }
  /** Replaces the app; it starts again, so the button stays busy. */
  async function install() {
    if(busy)return;setBusy('install');setError('');
    try {await api.installUpdate();}
    catch(e){setError(describe(e));setBusy('');}
  }
  async function download() {
    setError('');
    try {await api.openDownloads();} catch(e){setError(describe(e));}
  }
  return {release,busy,error,check:()=>act('check',()=>api.checkRelease()),skip:(version:string)=>act('skip',()=>api.skipRelease({version})),install,download};
}

function UpdateButton({release,busy,install,download}:{release:Release;busy:string;install:()=>void;download:()=>void}) {
  const t=useT();
  return release.installable?<button disabled={!!busy} onClick={install}>{busy==='install'?t.release.updating:t.release.update}</button>:
    <button disabled={!!busy} onClick={download}>{t.release.download}</button>;
}

/** A newer release the owner has not skipped: update now, or skip it. */
export function ReleaseNotice({api}:{api:DesktopApi}) {
  const t=useT();const {release,busy,error,skip,install,download}=useRelease(api);
  if(!release?.available||release.skipped||!release.latest)return null;
  const latest=release.latest;
  return <section className="network-notice preset-notice release-notice" aria-label={t.release.notice}>
    <p>{t.release.available(latest,release.current)}</p>
    {error&&<p className="network-validation" role="alert">{error}</p>}
    <span className="preset-actions">
      <button className="secondary" disabled={!!busy} onClick={()=>void skip(latest)}>{t.release.skip}</button>
      <UpdateButton release={release} busy={busy} install={()=>void install()} download={()=>void download()}/>
    </span>
  </section>;
}

/** This app's version in the settings: the latest release, when it was
 * looked for, and a look now. A skipped release can be installed here. */
export function ReleaseSection({api}:{api:DesktopApi}) {
  const t=useT();const {release,busy,error,check,install,download}=useRelease(api);
  if(!release)return null;
  const newer=release.available&&release.latest;
  return <section className="agent-form release-section" aria-label={t.release.title}>
    <h3>{t.release.title}</h3>
    <div className="setting-row"><strong>{t.release.version(release.current)}</strong><span>{newer?t.release.available(newer,release.current):t.release.upToDate}</span></div>
    {release.checkedAt!==null&&<small>{t.preset.checked(new Date(release.checkedAt*1000).toLocaleString(t.tag))}</small>}
    {release.error&&<p className="network-help">{t.release.failed}</p>}
    {error&&<p className="network-validation" role="alert">{error}</p>}
    <span className="preset-actions">
      <button className="secondary" disabled={!!busy} onClick={()=>void check()}>{busy==='check'?t.preset.checking:t.release.check}</button>
      {newer&&<UpdateButton release={release} busy={busy} install={()=>void install()} download={()=>void download()}/>}
    </span>
  </section>;
}
