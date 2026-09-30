import {useCallback,useEffect,useRef,useState} from 'react';
import type {DesktopApi,NetworkPreferences,NetworkSettings} from './types';
import {useDescribe,useT} from './i18n';
const addresses=(text:string)=>text.split('\n').map(value=>value.trim()).filter(Boolean);

export function NetworkPanel({api,onBack,embedded=false}:{api:DesktopApi;onBack:()=>void;embedded?:boolean}) {
  const t=useT();const errorText=useDescribe();
  const [observed,setObserved]=useState<NetworkSettings>();
  const [baseline,setBaseline]=useState<NetworkSettings>();
  const baselineRef=useRef<NetworkSettings|undefined>(undefined);
  const [relays,setRelays]=useState('');const [verifiers,setVerifiers]=useState('');
  const [bootstrap,setBootstrap]=useState('');
  const [lanDiscovery,setLanDiscovery]=useState(false);
  const [dhtServer,setDhtServer]=useState(false);
  const [relayOnly,setRelayOnly]=useState(false);
  const [error,setError]=useState('');const [statusError,setStatusError]=useState('');
  const [notice,setNotice]=useState('');const [busy,setBusy]=useState(false);
  const saving=useRef(false);const mounted=useRef(false);const generation=useRef(0);
  const adopt=useCallback((value:NetworkSettings)=>{
    baselineRef.current=value;setBaseline(value);
    setRelays(value.preferences.relays.join('\n'));setVerifiers(value.preferences.autoNatPeers.join('\n'));
    setBootstrap((value.preferences.bootstrapPeers??[]).join('\n'));
    setLanDiscovery(value.preferences.lanDiscovery);
    setDhtServer(value.preferences.dhtServer);
    setRelayOnly(value.preferences.relayOnly);
  },[]);
  const load=useCallback(async(replaceForm=false)=>{
    if(saving.current)return;
    const current=++generation.current;
    try {
      const value=await api.networkSettings();
      if(!mounted.current||current!==generation.current)return;
      setObserved(value);setStatusError('');
      if(replaceForm||!baselineRef.current){adopt(value);setError('');setNotice('');}
    } catch(err) {
      if(mounted.current&&current===generation.current){setStatusError(errorText(err));}
    }
  },[api,adopt,errorText]);
  useEffect(()=>{
    mounted.current=true;void load();
    const stop=api.subscribe(()=>void load());
    const timer=setInterval(()=>void load(),1500);
    return()=>{mounted.current=false;++generation.current;clearInterval(timer);stop();};
  },[api,load]);
  // An empty field leaves the routes to the network: the node takes its preset's.
  const bootstrapPeers=addresses(bootstrap);
  const preferences:NetworkPreferences={relays:addresses(relays),relayOnly,autoNatPeers:addresses(verifiers),...(bootstrapPeers.length?{bootstrapPeers}:{}),lanDiscovery,dhtServer};
  const tooMany=preferences.relays.length>4||preferences.autoNatPeers.length>4||bootstrapPeers.length>4;
  const missingRelay=relayOnly&&preferences.relays.length===0;
  const changed=Boolean(baseline&&(
    preferences.lanDiscovery!==baseline.preferences.lanDiscovery||
    preferences.dhtServer!==baseline.preferences.dhtServer||
    preferences.relayOnly!==baseline.preferences.relayOnly||
    preferences.relays.join('\n')!==baseline.preferences.relays.join('\n')||
    preferences.autoNatPeers.join('\n')!==baseline.preferences.autoNatPeers.join('\n')||
    bootstrapPeers.join('\n')!==(baseline.preferences.bootstrapPeers??[]).join('\n')
  ));
  async function save() {
    if(!baseline||busy||!changed||tooMany||missingRelay)return;
    // Keep the form's original revision until a save succeeds or the owner explicitly reloads.
    // A background status refresh must not silently change a retry after a lost response.
    const request={expectedRevision:baseline.revision,preferences};
    saving.current=true;++generation.current;setBusy(true);setError('');setNotice('');
    try {
      const value=await api.configureNetwork(request);
      if(mounted.current){adopt(value);setObserved(value);setStatusError('');setNotice(t.network.saved);}
    } catch(err) {if(mounted.current)setError(errorText(err));}
    finally {saving.current=false;if(mounted.current)setBusy(false);}
  }
  const status=observed?.status;
  const unavailable=statusError||error;
  return <section className={embedded?'network-panel embedded':'network-panel agent-panel'} aria-label={t.network.title}>
    {embedded?<header className="embedded-header"><h3>{t.network.title}</h3><p>{t.network.text}</p></header>:<header className="agent-header"><div><span className="eyebrow">{t.network.eyebrow}</span><h2>{t.network.title}</h2><p>{t.network.text}</p></div><button className="secondary" disabled={busy} onClick={onBack}>{t.common.back}</button></header>}
    {unavailable&&<div className="error-banner" role="alert">{unavailable}</div>}
    {notice&&<p className="network-notice" role="status">{notice}</p>}
    {!baseline&&!statusError&&<p className="agent-muted">{t.network.loading}</p>}
    {baseline&&<div className="network-columns">
      <form className="agent-form network-form" onSubmit={event=>{event.preventDefault();void save();}}>
        <h3>{t.network.routes}</h3>
        <label className="check-row" htmlFor="network-dht"><input id="network-dht" type="checkbox" checked={dhtServer} disabled={busy} onChange={e=>{setDhtServer(e.target.checked);setNotice('');}}/>{t.network.dht}</label>
        <p className="network-help">{t.network.dhtHelp}</p>
        <label className="check-row" htmlFor="network-lan"><input id="network-lan" type="checkbox" checked={lanDiscovery} disabled={busy} onChange={e=>{setLanDiscovery(e.target.checked);setNotice('');}}/>{t.network.lan}</label>
        <p className="network-help">{t.network.lanHelp}</p>
        <label htmlFor="network-bootstrap">{t.network.bootstrap}<textarea id="network-bootstrap" aria-label={t.network.bootstrap} value={bootstrap} placeholder={observed?.status.bootstrap.routes.join('\n')} disabled={busy} onChange={e=>{setBootstrap(e.target.value);setNotice('');}} rows={3} maxLength={1100} spellCheck={false} dir="ltr" aria-describedby="network-bootstrap-help"/><small id="network-bootstrap-help">{t.network.bootstrapHelp}</small></label>
        <label htmlFor="network-relays">{t.network.relays}<textarea id="network-relays" aria-label={t.network.relays} value={relays} disabled={busy} onChange={e=>{setRelays(e.target.value);setNotice('');}} rows={3} maxLength={1100} spellCheck={false} dir="ltr" aria-describedby="network-relays-help"/><small id="network-relays-help">{t.network.relaysHelp}</small></label>
        <label className="check-row" htmlFor="network-relay-only"><input id="network-relay-only" type="checkbox" checked={relayOnly} disabled={busy} onChange={e=>{setRelayOnly(e.target.checked);setNotice('');}}/>{t.network.relayOnly}</label>
        <p className="network-help">{t.network.relayOnlyHelp}</p>
        <label htmlFor="network-verifiers">{t.network.verifiers}<textarea id="network-verifiers" aria-label={t.network.verifiers} value={verifiers} disabled={busy} onChange={e=>{setVerifiers(e.target.value);setNotice('');}} rows={3} maxLength={1100} spellCheck={false} dir="ltr" aria-describedby="network-verifiers-help"/><small id="network-verifiers-help">{t.network.verifiersHelp}</small></label>
        {tooMany&&<p className="network-validation">{t.network.tooMany}</p>}
        {missingRelay&&<p className="network-validation">{t.network.needsRelay}</p>}
        {observed&&observed.revision!==baseline.revision&&<p className="network-validation">{t.network.changedElsewhere}</p>}
        <button type="submit" disabled={busy||!changed||tooMany||missingRelay}>{busy?t.network.reconnecting:t.network.save}</button>
        <small>{t.network.saveHelp}</small>
      </form>
      <aside className="network-status" aria-label={t.network.statusLabel}>
        <h3>{t.network.now}</h3>
        {status&&!statusError?<>
          <p className="network-help">{status.routing.blockedByPolicy?t.network.dhtBlocked:status.routing.mode==='server'?t.network.dhtServer:t.network.dhtClient}</p>
          <p className="network-help">{status.lanDiscovery.blockedByPolicy?t.network.lanBlocked:status.lanDiscovery.active?t.network.lanOn:t.network.lanOff}</p>
          <p className="network-bootstrap-state">{status.bootstrap.verifiedPeers.length?t.network.verified(status.bootstrap.verifiedPeers.length):t.network.noVerified}</p>
          {!status.bootstrap.verifiedPeers.length&&<p className="network-help">{t.network.joinHint}</p>}
          {status.bootstrap.policyBlockedHints>0&&<p className="network-help">{t.network.blockedHints(status.bootstrap.policyBlockedHints)}</p>}
          <p className="network-route-count">{t.network.relayCount(status.relayRoutes.length,observed.preferences.relays.length)}</p>
          <p>{status.autoNat.status==='public'?t.network.public:status.autoNat.status==='private'?t.network.private:t.network.unknown}</p>
          <p className="network-help">{status.listening?t.network.peers(status.connectedPeers):t.network.waitingPorts}</p>
          {observed.preferences.relays.length>status.relayRoutes.length&&<p className="network-help">{t.network.waitingRelay}</p>}
          <details><summary>{t.network.diagnostics}</summary><dl><dt>Peer ID</dt><dd>{status.peerId}</dd><dt>{t.network.invitationAddresses}</dt><dd>{status.advertisedAddresses.length?status.advertisedAddresses.map(address=><div key={address}>{address}</div>):t.network.noRoutes}</dd><dt>{t.network.probes}</dt><dd>{t.network.probeCounts(status.autoNat.successfulProbes,status.autoNat.failedProbes)}</dd><dt>{t.network.joining}</dt><dd>{t.network.joinCounts(status.bootstrap.candidateHints,status.bootstrap.inFlight,status.bootstrap.failedAttempts)}</dd><dt>{t.network.signed}</dt><dd>{status.bootstrap.verifiedPeers.length?status.bootstrap.verifiedPeers.map(peer=><div key={peer.peerId}>{peer.rootId}<br/>{peer.peerId}</div>):t.network.notYet}</dd></dl><p className="network-help">{t.network.signatureNote}</p></details>
        </>:<p className="network-help">{t.network.unavailable}</p>}
      </aside>
    </div>}
    <button className="secondary network-reload" disabled={busy} onClick={()=>void load(true)}>{t.network.reload}</button>
  </section>;
}
