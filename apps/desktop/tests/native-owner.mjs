// V1-GF01 on real daemons: three hidden WKWebViews (the e2e build with its
// isolated vault) on a local chain with ten bonded holders and our identity
// server behind a stand-in Google. Everything is done through the window:
// onboarding with Google coins, buying a book, contact by id under a manual
// policy, direct and group chats with roles, grants and skills. Untrusted
// markup in names and messages must stay text.
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdtemp,mkdir,readFile,writeFile,chmod,stat} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import {createNativeHarness} from './native-harness.mjs';

assert.equal(process.platform,'darwin','This runner exercises the macOS WKWebView build');
const root=resolve(import.meta.dirname,'../../..');
const binary=process.env.AIN_DESKTOP_BINARY??join(root,'target/debug/agentic-desktop');
const temporary=await mkdtemp('/tmp/ain-gf01-');
const evidence=join(root,'output/native-owner',new Date().toISOString().replaceAll(':','-'));
await mkdir(evidence,{recursive:true});
const {Client,until,cleanup,clients:windows}=createNativeHarness({binary,temporary,evidence});
const outcomes=[];const timings={};const started=Date.now();
const mark=label=>{timings[label]=Math.round((Date.now()-started)/1000);};
/** Anvil's account 3: the GrantIssuer's first issuer in the native network. */
const ISSUER_KEY='0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6';
const DOMAIN='0xae2e3182ade817a3e726c29ef308eed15c6ec267ffc01a68da990e576fa0df18';
const hostile='<img src=x onerror="window.__pwned=1">';
/** Alice's own name: every peer who meets her renders it. */
const aliceName=`Alice ${hostile}`;
const services=[];
function service(label,args,env,dir) {
  const child=spawn('cargo',['test','--locked',...args],{cwd:root,env:{...process.env,...env},stdio:['ignore','pipe','pipe']});
  let log='';child.stdout.on('data',b=>log+=b);child.stderr.on('data',b=>log+=b);
  services.push({label,child,dir,log:()=>log});
  return child;
}
async function json(path,label,timeout) {
  const service=services.find(s=>path.startsWith(s.dir));
  return until(label,async()=>{
    if(service&&service.child.exitCode!==null)throw new Error(`${service.label} exited: ${service.log().slice(-3000)}`);
    return existsSync(path)?JSON.parse(await readFile(path,'utf8')):null;
  },timeout);
}
/** `daemon.json` from the network's `serve` flags. */
function daemonFlags(flags,identityServer) {
  const saved={listen:[],bootstrap:[],identityServer};
  const names={'--chain-rpc':'chainRpc','--chain-id':'chainId','--book-shop':'bookShop','--grant-issuer':'grantIssuer','--registry':'registry','--chain-confirmations':'chainConfirmations'};
  for(let i=0;i<flags.length;i+=2) {
    if(flags[i]==='--bootstrap')saved.bootstrap.push(flags[i+1]);
    else saved[names[flags[i]]]=['--chain-id','--chain-confirmations'].includes(flags[i])?Number(flags[i+1]):flags[i+1];
  }
  return saved;
}
const short=id=>`${id.slice(0,10)}…${id.slice(-6)}`;
const links=async client=>{try{return (await readFile(join(client.data,'opened-links.txt'),'utf8')).trim().split('\n').filter(Boolean);}catch{return [];}};
const headerIs=(client,title)=>client.read('return document.querySelector(".chat-header h2")?.textContent===arguments[0];',[title]);
const received=(client,text)=>client.read('return [...document.querySelectorAll(".message.received > p")].some(p=>p.textContent===arguments[0]);',[text]);
const inert=async client=>{
  const state=await client.read('return {pwned:window.__pwned??null,images:document.querySelectorAll("img").length,scripts:document.querySelectorAll("body script").length};');
  assert.deepEqual(state,{pwned:null,images:0,scripts:0},`${client.name}: untrusted markup must stay text`);
};
/** Follows the provider's login link like a browser; the stand-in provider
 * signs in at once. */
async function signIn(link,provider) {
  assert.ok(link.endsWith(`/login/${provider}`),`the ${provider} button opened ${link}`);
  const response=await fetch(link,{redirect:'follow',signal:AbortSignal.timeout(20000)});
  assert.equal(response.status,200,await response.text());
}
/** Clicks `text` whenever it is enabled until `done`, as an owner retries. */
async function retrying(client,label,text,done,timeout=240000) {
  return until(label,async()=>{
    if(await done())return true;
    const idle=await client.read('const b=[...document.querySelectorAll("button")].find(b=>b.textContent.trim()===arguments[0]);return Boolean(b&&!b.disabled);',[text]);
    if(idle)await client.button(text);
    await delay(1500);
    return done();
  },timeout);
}
/** Opens a conversation from the sidebar once the window lists it. */
async function openConversation(client,title) {
  const item=`//nav[@aria-label='Conversations']//button[.//strong[normalize-space(.)=${JSON.stringify(title)}]]`;
  await until(`${client.name} lists ${title}`,async()=>{await client.xclick(item);return true;},60000);
  await until(`${client.name} opens ${title}`,()=>headerIs(client,title));
}

try {
  // The network and the identity server.
  const net=join(temporary,'net'),id=join(temporary,'id');await mkdir(net);await mkdir(id);
  await writeFile(join(id,'issuer.key'),ISSUER_KEY);
  service('network',['-p','agentic-node','--test','processes','--','--ignored','af08_host_network','--nocapture'],{AIN_AF08_DIR:net},net);
  service('identity server',['-p','agentic-identity-server','--test','identity_server','--','--ignored','gui_host_identity_server','--nocapture'],{AIN_GF01_DIR:id,AIN_GF01_DOMAIN:DOMAIN,AIN_GF01_ISSUER_KEY_FILE:join(id,'issuer.key')},id);
  const identity=await json(join(id,'identity.json'),'identity server',600000);
  const network=await json(join(net,'network.json'),'native network with ten bonded holders',900000);
  mark('network ready');
  const saved=daemonFlags(network.flags,identity.url);
  const clients={};
  for(const name of ['alice','bob','carol']) {
    const data=join(temporary,name);await mkdir(data,{mode:0o700});await chmod(data,0o700);
    await writeFile(join(data,'daemon.json'),JSON.stringify(saved));
    clients[name]=await new Client(name).launch();
  }
  const {alice:a,bob:b,carol:c}=clients;
  outcomes.push('Each window started its profile daemon with the flags saved in daemon.json (chain, bootstrap, identity server), as the CLI does');

  // Onboarding: own keys, then monthly coins through a login.
  for(const [client,name,provider] of [[a,aliceName,'google'],[c,'Carol','github']]) {
    await client.createProfile(name,{coins:true});
    if(client===a)await a.screenshot('alice-onboarding-choice');
    await client.button(provider==='google'?'Log in with Google':'Log in with GitHub');
    const link=await until(`${client.name} login link opened`,async()=>(await links(client)).find(l=>l.startsWith(identity.url)),60000);
    await signIn(link,provider);
    // The app goes on by itself once the grant arrives.
    await until(`${client.name} granted coins`,()=>client.textIncludes('Connect your agent'),180000);
    const instruction=await until(`${client.name} agent instructions`,()=>client.read('return document.querySelector("textarea[readonly]")?.value.includes("kaiki")&&document.querySelector("textarea[readonly]").value;'));
    assert.match(instruction,/command-line tool is (?:'[^']*\/kaiki'|\S*\/kaiki) --data-dir /);
    if(client===a)await a.screenshot('alice-onboarding-agent');
    await client.finishOnboarding({skipLogin:false});
  }
  mark('login coins');
  outcomes.push('Onboarding made each profile on its device and got monthly coins through the identity server, Alice with Google and Carol with GitHub, then gave the agent instructions naming the CLI beside the app; the app opened only the server’s login link');

  // Bob buys a book: the wallet opens the daemon's payment URI, paid on chain.
  await b.createProfile(hostile);
  await b.nav('wallet');await b.button('Top up with crypto');
  await until('payment URI shown',()=>b.read('return Boolean(document.querySelector("textarea[aria-label=\\"Payment link\\"]")?.value);'),60000);
  await b.button('Pay with ETH');
  const uri=await until('payment opened in the wallet',async()=>(await links(b)).find(l=>l.startsWith('ethereum:')),30000);
  const pay=new URL(uri.replace('ethereum:','ethereum://'));
  const shop=uri.slice('ethereum:'.length,uri.indexOf('@'));
  assert.equal(shop.toLowerCase(),network.shop.toLowerCase());
  execFileSync('cast',['send',shop,'buy(address,bytes32)',pay.searchParams.get('address'),pay.searchParams.get('bytes32'),'--value',pay.searchParams.get('value'),'--private-key',network.payer,'--rpc-url',network.chain],{stdio:'ignore'});
  // The paid book arrives; the profile has already spent a coin on its intro card.
  await until('purchase noticed in the wallet',()=>b.textIncludes('Balance topped up: 1,000 coins added.'),180000);
  const left=Number((await b.read('return document.querySelector("[data-testid=coins-left]").textContent;')).replaceAll(',',''));
  assert.ok(left>=990&&left<=1000,`coins left after the purchase: ${left}`);
  await b.screenshot('bob-wallet-bought');
  mark('book bought');
  outcomes.push('Wallet: coins buy issued a payment URI, the app opened exactly it, the on-chain purchase appeared as 1000 coins');

  // Contact by id under Bob's manual policy.
  const ids={a:(await a.snapshot()).identity.networkId,b:(await b.snapshot()).identity.networkId,c:(await c.snapshot()).identity.networkId};
  await b.nav('settings');
  await until('policy form',()=>b.textIncludes('Who can write by ID'));
  await b.click('input[name="intro-mode"][value="manual"]');await b.button('Save rules');
  await until('manual policy saved',()=>b.textIncludes('Rules saved.'));
  await b.button('Back to chat');
  await a.nav('contacts');
  await a.fill('#friend-invitation',ids.b);await a.fill('#friend-name','Bob');
  await retrying(a,'Alice asks Bob by id','Add friend',()=>headerIs(a,'Bob'));
  mark('request sent');
  await b.nav('contacts');
  await until('Bob sees the waiting request',()=>b.read('return [...document.querySelectorAll(".request-card strong")].some(s=>s.textContent===arguments[0]);',[aliceName]),180000);
  await inert(b);await b.screenshot('bob-contact-request');
  await b.xclick("//article[contains(@class,'request-card')][.//span[normalize-space(.)='Conversation request']]//button[normalize-space(.)='Accept']");
  await until('Bob joined Alice',()=>headerIs(b,aliceName),60000);
  // Carol asks too; Bob rejects her.
  await c.nav('contacts');
  await c.fill('#friend-invitation',ids.b);await c.fill('#friend-name','Bob');
  await retrying(c,'Carol asks Bob by id','Add friend',()=>headerIs(c,'Bob'));
  await b.nav('contacts');
  await until('Bob sees Carol’s request',()=>b.read('return [...document.querySelectorAll(".request-card strong")].some(s=>s.textContent===arguments[0]);',['Carol']),180000);
  await b.xclick("//article[contains(@class,'request-card')][.//strong[normalize-space(.)='Carol']]//button[normalize-space(.)='Reject']");
  await until('Carol’s request rejected',()=>b.textIncludes('Request from “Carol” rejected.'),30000);
  assert.equal((await b.invoke('intro_requests',{})).length,0);
  assert.equal((await b.snapshot()).conversations.some(x=>x.title==='Carol'),false,'a rejected request makes no conversation');
  await b.button('Back to chat');
  outcomes.push('Contact by network id: Alice’s paid request waited under Bob’s manual policy and Bob accepted it; Carol’s request he rejected, and no conversation came of it');

  // A direct message with markup, and delivery statuses.
  const first=`${hostile} Hi, Bob! This is a GUI on top of the daemon.`;
  await a.fill('#message',first);await a.click('button[aria-label="Send message"]');
  await until('Bob reads Alice',()=>received(b,first),120000);
  await until('delivery shown to Alice',()=>a.read('return [...document.querySelectorAll(".message.own .delivery")].some(d=>/Delivered|Read|Stored/.test(d.textContent));'),120000);
  const reply='Hi! I see the markup as text.';
  await b.fill('#message',reply);await b.click('button[aria-label="Send message"]');
  await until('Alice reads Bob',()=>received(a,reply),120000);
  await inert(a);await inert(b);
  await a.screenshot('alice-direct-chat');await b.screenshot('bob-direct-chat');
  mark('direct chat');
  outcomes.push('Direct chat: markup in a name and a message rendered as text on both sides (no element, no script run); delivery status shown');

  // A group with roles. Carol decides on invitations herself.
  await c.nav('settings');
  await until('Carol’s policy form',()=>c.textIncludes('Who can write by ID'));
  await c.click('input[name="intro-mode"][value="manual"]');await c.button('Save rules');
  await until('Carol’s manual policy saved',()=>c.textIncludes('Rules saved.'));
  await c.button('Back to chat');
  const groupName=`Release <b>GF01</b>`;
  await a.nav('new-group');
  await a.fill('#group-name',groupName);await a.fill('#group-members',`${ids.b}\n${ids.c}`);
  await retrying(a,'Alice makes the group','Make group',()=>headerIs(a,groupName));
  mark('group created');
  // Bob is Alice's contact, so her invitation lets him in at once; Carol
  // accepts hers in the window.
  await until('Bob in the group',async()=>(await b.snapshot()).conversations.some(x=>x.title===groupName),180000);
  await openConversation(b,groupName);
  await c.nav('contacts');
  await until('Carol sees the group invitation',()=>c.textIncludes('Group invitation'),180000);
  await c.screenshot('carol-group-invitation');
  await c.xclick("//article[contains(@class,'request-card')][.//span[normalize-space(.)='Group invitation']]//button[normalize-space(.)='Accept']");
  await until('Carol in the group',()=>headerIs(c,groupName),60000);
  const hello='Hello everyone in the group.';
  await openConversation(a,groupName);
  await a.fill('#message',hello);await a.click('button[aria-label="Send message"]');
  await until('Bob reads the group',()=>received(b,hello),120000);
  await until('Carol reads the group',()=>received(c,hello),120000);
  assert.equal(await c.read('return document.querySelector(".message.received .message-author")?.textContent;'),short(ids.a));
  mark('group message');
  // The owner names Bob admin; Bob, as admin, removes Carol.
  await a.button('Members');
  await until('members shown',()=>a.textIncludes('Members · 3'),30000);
  await a.click(`input[aria-label="Admin ${short(ids.b)}"]`);await a.button('Save admins');
  await until('Bob made admin',async()=>(await b.invoke('group',{request:{groupId:(await b.invoke('groups',{}))[0].id}})).role==='admin',180000);
  await a.screenshot('alice-group-members');
  await until('Bob sees his role',()=>b.textIncludes('you are admin'),60000);
  await b.button('Members');
  await until('Bob may remove Carol',()=>b.read('return Boolean(document.querySelector(arguments[0]));',[`button[aria-label="Remove ${short(ids.c)}"]`]),30000);
  assert.equal(await b.read('return Boolean(document.querySelector(arguments[0]));',[`button[aria-label="Remove ${short(ids.a)}"]`]),false,'an admin cannot remove the owner');
  await b.click(`button[aria-label="Remove ${short(ids.c)}"]`);
  await until('Carol removed',()=>b.textIncludes('Members · 2'),180000);
  await b.screenshot('bob-group-admin');
  await b.button('Back to chat');
  // Alice writes once her own node has applied the removal: a message sent
  // in the old epoch before that is still readable by the old members.
  await until('Alice knows Carol is removed',async()=>(await a.invoke('groups',{})).find(x=>x.name===groupName)?.members.length===2,180000);
  const after='A message after removing Carol.';
  await openConversation(a,groupName);
  await a.fill('#message',after);await a.click('button[aria-label="Send message"]');
  await until('Bob reads after the removal',()=>received(b,after),120000);
  await delay(15000);
  assert.equal((await c.snapshot()).conversations.find(x=>x.title===groupName).messages.some(m=>m.text===after),false,'a removed member reads nothing new');
  await inert(a);await inert(b);await inert(c);
  await a.screenshot('alice-group-chat');await c.screenshot('carol-after-removal');
  mark('group roles');
  outcomes.push('Group: the owner made it by ids (Bob, her contact, joined at once; Carol accepted the invitation under her manual policy), named Bob admin, Bob as admin removed Carol but could not remove the owner, and Carol read nothing after');

  // Agents: a grant with commands and credentials, a skill, the revoke.
  await openConversation(a,'Bob');
  const direct=(await a.snapshot()).conversations.find(x=>x.title==='Bob').id;
  await a.nav('agents');await until('agents',()=>a.textIncludes('Agent access'));
  await a.fill('#agent-name','GF01 helper');
  await a.click(`input[type="checkbox"][value="${direct}"]`);await a.click('#agent-allow-send');
  await a.button('Grant access');
  const cli=await until('CLI command shown',()=>a.read('return document.querySelector("textarea[aria-label=\\"CLI command\\"]")?.value;'),30000);
  const credentials=await a.read('return document.querySelector("input[aria-label=\\"Agent credentials file\\"]")?.value;');
  assert.ok(cli.includes(credentials),'the command names the credentials file');
  assert.equal(((await stat(credentials)).mode&0o077),0,'the credentials file is private');
  await a.button('Install in Codex');
  await until('skill installed',()=>a.textIncludes('Installed for Codex'),10000);
  const skill=await readFile(join(a.data,'home/.codex/skills/agentic-messaging/SKILL.md'),'utf8');
  assert.equal(skill,await readFile(join(root,'integrations/agent-skill/agentic-messaging/SKILL.md'),'utf8'));
  await a.screenshot('alice-agents-grant');
  await a.click('button[aria-label="Revoke access GF01 helper"]');
  await until('grant revoked',()=>a.textIncludes('Revoked'),30000);
  outcomes.push('Agents: a grant with read and send showed its CLI and MCP settings and a private credentials file, the skill installed for Codex, the revoke took effect');
  await a.nav('wallet');await until('wallet',()=>a.textIncludes('Monthly coins for your login'),30000);
  await a.screenshot('alice-wallet-granted');
  // The same window in Russian.
  const language=code=>a.read('const list=document.querySelector(".sidebar-footer .language-select select");list.value=arguments[0];list.dispatchEvent(new Event("change",{bubbles:true}));return list.options.length;',[code]);
  assert.equal(await language('ru'),20);
  await until('Russian interface',()=>a.textIncludes('Ежемесячные монеты за вход'),10000);
  await a.screenshot('alice-wallet-ru');
  await language('ar');
  await until('Arabic interface, right to left',()=>a.read('return document.documentElement.dir==="rtl"&&document.body.innerText.includes(arguments[0]);',['العملات الشهرية مقابل تسجيل دخولك']),10000);
  await a.screenshot('alice-wallet-ar');
  await language('en');
  outcomes.push('The interface starts in English; the language list offers twenty languages and switches to Russian and to right-to-left Arabic');
  mark('done');

  await writeFile(join(evidence,'result.json'),JSON.stringify({passed:true,platform:'macOS WKWebView (e2e build)',timings,outcomes},null,2)+'\n');
  console.log(JSON.stringify({passed:true,timings,outcomes,evidence},null,2));
} catch(error) {
  for(const client of windows) {
    await writeFile(join(evidence,`${client.name}.log`),client.log??'');
    if(client.session)await client.screenshot(`${client.name}-failure`).catch(()=>{});
  }
  await writeFile(join(evidence,'result.json'),JSON.stringify({passed:false,timings,outcomes,error:error.stack},null,2)+'\n');
  for(const s of services)await writeFile(join(evidence,`${s.label.replaceAll(' ','-')}.log`),s.log().slice(-200000));
  throw error;
} finally {
  for(const s of services)await writeFile(join(s.dir,'stop'),'').catch(()=>{});
  await cleanup().catch(()=>{});
  for(const s of services){if(s.child.exitCode===null){await delay(3000);if(s.child.exitCode===null)s.child.kill('SIGTERM');}}
}
