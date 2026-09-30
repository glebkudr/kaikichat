import {useCallback,useEffect,useState} from 'react';
import type {Balance,DesktopApi,Payment} from './types';
import {CoreError} from './core-error';
import {useDescribe,useT} from './i18n';
import {ProviderButtons} from './ProviderButtons';
import type {LoginProvider,PaymentStep} from './types';

const sleep=(ms:number)=>new Promise(resolve=>setTimeout(resolve,ms));
/** Asks again while the node waits for the chain or the identity server. */
export async function askAgain<T>(call:()=>Promise<T>,code:string,attempts=20,wait=sleep):Promise<T> {
  for(let attempt=1;;attempt++) {
    try {return await call();}
    catch(error){if(!(error instanceof CoreError)||error.code!==code||attempt>=attempts)throw error;await wait(1000);}
  }
}
/** Wei as ether, without rounding a small price to zero. */
export function ether(wei:string,decimal='.') {
  const value=BigInt(wei);const unit=10n**18n;
  const fraction=(value%unit).toString().padStart(18,'0').replace(/0+$/,'');
  return `${value/unit}${fraction?`${decimal}${fraction}`:''} ETH`;
}

/** Coins through a login: the login page opens in the browser, the node
 * collects the grant when the server decides. */
export function useClaim(api:DesktopApi,onBalance:(balance:Balance)=>void) {
  const describe=useDescribe();
  const [busy,setBusy]=useState<LoginProvider|''>('');const [error,setError]=useState('');const [code,setCode]=useState('');const [link,setLink]=useState('');
  async function claim(provider:LoginProvider) {
    if(busy)return;setBusy(provider);setError('');setCode('');
    try {const opened=await askAgain(()=>api.claimCoins({provider}),'claim_pending');setLink(`${opened.loginUrl}/${provider}`);onBalance(await api.coinsBalance());}
    catch(e){setError(describe(e));setCode(e instanceof CoreError?e.code:'');}
    finally {setBusy('');}
  }
  return {busy,error,code,link,claim};
}
export type ClaimNote={text:string;note:string};
/** What became of the login. The owner takes the login for signing in, so
 * an account that already got its coins is explained by how signing in
 * works: this device already holds the key, or (holding none of the coins)
 * the key is on another device and this one is a separate account. */
export function useClaimOutcome(balance:Balance|undefined):ClaimNote {
  const t=useT();
  const say=(text:string,note='')=>({text,note});
  const last=balance?.lastClaim;
  if(!balance)return say('');
  if(balance.claim?.status==='open')return say(t.wallet.claimOpen);
  if(balance.claim?.status==='starting')return say(t.wallet.claimStarting);
  if(last?.status==='granted')return say(t.wallet.claimGranted);
  if(last?.status==='denied'&&last.reason==='already_claimed')return balance.books.some(book=>book.kind==='granted')?say(t.wallet.claimAgain,t.wallet.claimAgainNote):say(t.wallet.claimElsewhere,t.wallet.claimElsewhereNote);
  if(last?.status==='denied')return say(t.wallet.claimDenied(t.wallet.reasons[last.reason]??last.reason));
  return say('');
}
export function ClaimOutcome({text,note}:ClaimNote) {
  return <p className="claim-outcome" role="status">{text}{note&&<span className="claim-note">{note}</span>}</p>;
}

export function WalletPanel({api,onBack}:{api:DesktopApi;onBack:()=>void}) {
  const t=useT();const describe=useDescribe();
  const [balance,setBalance]=useState<Balance>();const [payment,setPayment]=useState<Payment>();
  const [busy,setBusy]=useState(false);const [error,setError]=useState('');const [notice,setNotice]=useState('');
  const load=useCallback(async()=>{try {setBalance(await api.coinsBalance());}catch(e){setError(describe(e));}},[api,describe]);
  const claim=useClaim(api,setBalance);
  const waiting=Boolean(balance?.pending.length||balance?.claim);
  useEffect(()=>{void load();const stop=api.subscribe(()=>{void load();});return stop;},[api,load]);
  // A purchase or a claim is decided outside the app: look again meanwhile.
  useEffect(()=>{if(!waiting)return;const timer=setInterval(()=>{void load();},3000);return ()=>clearInterval(timer);},[waiting,load]);
  async function topUp() {
    if(busy)return;setBusy(true);setError('');setNotice('');
    try {const made=await askAgain(()=>api.coinsBuy(),'chain_pending',15);setPayment(made);await load();}
    catch(e){setError(describe(e));}
    finally {setBusy(false);}
  }
  async function openPayment(step:PaymentStep) {
    if(!payment||busy)return;setError('');
    try {await api.openPayment({book:payment.book,step});setNotice(t.wallet.opened);}
    catch(e){setError(describe(e));}
  }
  const outcome=useClaimOutcome(balance);
  const paid=Boolean(payment&&balance?.books.some(book=>book.book===payment.book));
  const usd=(units:string)=>new Intl.NumberFormat(t.tag,{style:'currency',currency:'USD'}).format(Number(units)/1e6);
  const date=(seconds:number)=>new Date(seconds*1000).toLocaleDateString(t.tag,{day:'2-digit',month:'2-digit',year:'numeric'});
  return <section className="agent-panel" aria-label={t.wallet.label}>
    <header className="agent-header"><div><span className="eyebrow">{t.wallet.eyebrow}</span><h2>{t.wallet.title}</h2><p>{t.wallet.text}</p></div><button className="secondary" disabled={busy} onClick={onBack}>{t.common.back}</button></header>
    {(error||claim.error)&&<div className="error-banner" role="alert">{error||claim.error}</div>}
    {notice&&<div className="network-notice" role="status">{notice}</div>}
    <div className="agent-columns">
      <div className="panel-stack">
        <section className="network-status wallet-balance" aria-label={t.wallet.balance}><p className="network-route-count"><strong data-testid="coins-left">{balance?balance.remaining.toLocaleString(t.tag):'…'}</strong> {t.wallet.left}</p>
          {balance&&balance.books.length===0&&<p>{t.wallet.noBooks}</p>}
          {balance?.books.map(book=><div className="book-row" key={book.book}><span>{book.kind==='granted'?t.wallet.granted:t.wallet.bought}</span><span>{t.wallet.spent(book.used,book.count)}</span><small>{t.wallet.until(date(book.validUntil))}</small></div>)}
          {balance?.pending.map(item=><div className="book-row pending" key={item.book}><span>{t.wallet.awaitingPayment}</span><small>{t.wallet.created(date(item.createdAt))}</small></div>)}
        </section>
        <section className="agent-form"><h3>{t.wallet.loginTitle}</h3><p className="agent-muted">{t.wallet.loginText}</p>
          {outcome.text&&<ClaimOutcome {...outcome}/>}
          {claim.link&&balance?.claim&&<label>{t.wallet.loginLink}<input readOnly value={claim.link} onFocus={e=>e.currentTarget.select()}/></label>}
          <ProviderButtons busy={claim.busy} disabled={false} onPick={provider=>void claim.claim(provider)}/>
        </section>
      </div>
      <section className="agent-form"><h3>{t.wallet.cryptoTitle}</h3><p className="agent-muted">{t.wallet.cryptoText}</p>
        {payment&&paid&&<p className="claim-outcome" role="status">{t.wallet.paid(payment.count.toLocaleString(t.tag))}</p>}
        {payment&&!paid&&<div className="payment" aria-label={t.wallet.payment}>
          <dl><dt>{t.wallet.price}</dt><dd>{usd(payment.priceUsdc)}</dd><dt>{t.wallet.count}</dt><dd>{payment.count.toLocaleString(t.tag)}</dd><dt>{t.wallet.network}</dt><dd>{t.wallet.chain(payment.chainId)}</dd><dt>{t.wallet.shop}</dt><dd className="mono">{payment.shop}</dd></dl>
          <section className="pay-way" aria-label={t.wallet.ethTitle}><h4>{t.wallet.ethTitle}</h4>
            {payment.eth?<><p>{t.wallet.ethAmount(ether(payment.eth.value,t.decimal))}</p>
              <label>{t.wallet.paymentLink}<textarea aria-label={t.wallet.paymentLink} className="invitation-code" rows={3} readOnly value={payment.eth.uri} onFocus={e=>e.currentTarget.select()}/></label>
              <button type="button" onClick={()=>void openPayment('eth')}>{t.wallet.payEth}</button></>:<p className="network-validation">{t.wallet.ethStale}</p>}
          </section>
          <section className="pay-way" aria-label={t.wallet.usdcTitle}><h4>{t.wallet.usdcTitle}</h4><p>{t.wallet.usdcAmount((Number(payment.usdc.amount)/1e6).toLocaleString(t.tag,{minimumFractionDigits:2}))}</p>
            <div className="actions"><button type="button" className="secondary" onClick={()=>void openPayment('approve')}>{t.wallet.approveUsdc}</button><button type="button" className="secondary" onClick={()=>void openPayment('buy')}>{t.wallet.buyUsdc}</button></div>
          </section>
        </div>}
        <button disabled={busy} onClick={()=>void topUp()}>{busy?t.wallet.preparing:payment?t.wallet.topUpAgain:t.wallet.topUp}</button>
      </section>
    </div>
  </section>;
}
