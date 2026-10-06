import {networkFixture} from './network-fixture';
// UI-only visual fixture. Never imported by the production entry point.
// `?state=locked|new|keychain|nokeychain|stopped|failed|onboarding|coins|elsewhere|again|home` shows the profile gates
// (`keychain`: a key another program saved, which macOS asks about first;
// `nokeychain`: a Mac with no default keychain for the key; `failed`: a
// node that did not start)
// (`?release=newer|download|failed` a newer release;
// `?autostart=off|blocked|none` opening at login;
// `?move=offered|failed` an app outside the Applications folder)
// (`elsewhere`: the login's account already got its coins on another device;
// `again`: the owner logs in once more on the device that holds them);
// `?lang=ar&theme=light` starts in that language and theme; the
// screens are reached through the sidebar as in the app.
import {createRoot} from 'react-dom/client';
import {ChatShell} from '../src/ChatShell';
import type {Autostart,Balance,Card,DesktopApi,DoorRequest,Follow,Group,IntroPolicy,IntroRequest,NetworkPreset,Payment,ProfileStatus,Release,RuntimeInfo,Snapshot} from '../src/types';
import '../src/styles.css';
import {coreError} from '../src/core-error';
const own=`ain1${'5f'.repeat(32)}`,boris=`ain1${'9c'.repeat(32)}`,anna=`ain1${'3e'.repeat(32)}`,ivan=`ain1${'71'.repeat(32)}`;
const hostile='<img src=x onerror="document.body.dataset.pwned=1">';
const snapshot:Snapshot={identity:{name:'Alice',networkId:own},network:{state:'online',connectedPeers:12},conversations:[
  {id:'bob',title:'Boris',unread:0,messages:[
    {id:'1',author:boris,own:false,text:'Hi! I sent the task description. Can we discuss it today?',createdAt:1788563100,delivery:{phase:'delivered',replicas:10,target:10}},
    {id:'2',author:own,own:true,text:'Yes, I read it. Let us start with transport and check delivery between our nodes.',createdAt:1788563180,delivery:{phase:'delivered',replicas:10,target:10}},
    {id:'3',author:own,own:true,text:'Sending the results of the latest run.',createdAt:1788563240,delivery:{phase:'stored',replicas:7,target:10}},
    {id:'4',author:boris,own:false,text:hostile,createdAt:1788563260,delivery:{phase:'delivered',replicas:10,target:10}},
    {id:'5',author:boris,own:false,text:'Written over the office LAN while the Internet was down.',createdAt:1788563280,delivery:{phase:'delivered',replicas:10,target:10},lowTrust:true},
  ]},
  {id:'team',title:'Release team',unread:2,messages:[
    {id:'g1',author:anna,own:false,text:'The Linux build is ready, I am checking the package.',createdAt:1788563300,delivery:{phase:'delivered',replicas:10,target:10}},
    {id:'g2',author:own,own:true,text:'Great, we release after the check.',createdAt:1788563360,delivery:{phase:'queued',replicas:0,target:10}},
  ]},
  {id:'anna',title:'Anna',unread:0,messages:[]},
  {id:'readers',title:'Reading club',unread:0,messages:[]},
  {id:'news',title:'Release news',unread:0,messages:[
    {id:'n1',author:own,own:true,text:'0.3 is out: large groups and channels.',createdAt:1788563420,delivery:{phase:'stored',replicas:7,target:10}},
  ]},
  {id:'club',title:'Private club',unread:0,messages:[]},
  {id:'ab'.repeat(32),title:'Rustaceans',unread:0,messages:[
    {id:'p1',author:ivan,own:false,text:'Today we dig into async in Rust 2027: bring your questions.',createdAt:1788563500,delivery:{phase:'delivered',replicas:10,target:10}},
  ]},
  {id:'ac'.repeat(32),title:'Rust Weekly',unread:0,messages:[
    {id:'w1',author:ivan,own:false,text:'Issue 212: what is new in Cargo and three crates of the week.',createdAt:1788563550,delivery:{phase:'delivered',replicas:10,target:10}},
  ]},
]};
const follows:Follow[]=[
  {id:'ab'.repeat(32),name:'Rustaceans',owner:ivan,since:1788563000,closed:false},
  {id:'ac'.repeat(32),name:'Rust Weekly',owner:ivan,since:1788563000,closed:false,kind:'channel',retention:0,sealed:false},
];
const cards:Card[]=[
  {id:'c1'.repeat(32),kind:'group',name:'Rustaceans',about:'Rust and agents: we discuss tooling, with a code review every Friday.',tags:['rust','agents'],langs:['en','de'],owner:ivan,groupRef:'cd'.repeat(32),expiresAt:1791155100},
  {id:'c3'.repeat(32),kind:'channel',name:'Rust Weekly',about:'Once a week: Rust news, crates of the week and release reviews.',tags:['rust','news'],langs:['en'],owner:ivan,groupRef:'ce'.repeat(32),expiresAt:1791155100},
  {id:'c2'.repeat(32),kind:'profile',name:'Anna',about:'I build agents for customer support, looking for colleagues.',tags:['agents','support'],langs:['en'],owner:anna,groupRef:null,expiresAt:1791155100},
];
const groups:Group[]=[
  {id:'team',name:'Release team',epoch:4,owner:own,admins:[anna],members:[own,anna,boris,ivan],role:'owner',banned:[{id:`ain1${'e7'.repeat(32)}`,byOwner:true}],access:'public',groupRef:'ef'.repeat(32),kind:'group',retention:1},
  {id:'readers',name:'Reading club',epoch:12,owner:own,admins:[anna],members:[own,anna,boris],role:'owner',banned:[],access:'request',groupRef:'e1'.repeat(32),kind:'group',retention:1},
  {id:'news',name:'Release news',epoch:2,owner:own,admins:[anna],members:[own,anna],role:'owner',banned:[],access:'public',groupRef:'e2'.repeat(32),kind:'channel',retention:90},
  {id:'club',name:'Private club',epoch:5,owner:own,admins:[boris],members:[own,boris],role:'owner',banned:[],access:'private',groupRef:'e3'.repeat(32),kind:'channel',retention:30},
];
let doorRequests:DoorRequest[]=[
  {requestId:'d1'.repeat(32),networkId:ivan,note:'I read science fiction and want to discuss new books.',receivedAt:1788563000,rejoin:false},
  {requestId:'d2'.repeat(32),networkId:boris,note:hostile,receivedAt:1788563050,rejoin:true},
];
let requests:IntroRequest[]=[{requestId:'r1',networkId:ivan,name:'Ivan',receivedAt:1788563000},{requestId:'r2',networkId:boris,name:hostile,receivedAt:1788563050,group:'g9'}];
let policy:IntroPolicy={mode:'manual',dailyLimit:20,allowed:[]};
let runtimes:RuntimeInfo[]=[];
const balance:Balance={books:[{book:'0x01',kind:'granted',count:100,used:23,validUntil:1791155100},{book:'0x02',kind:'bought',count:1000,used:0,validUntil:1791155100}],pending:[],remaining:1077,claim:null,lastClaim:{status:'granted',book:'0x01'}};
const call=(to:string,uri:string)=>({to,calldata:'0x',uri});
const payment:Payment={book:'0x03',key:'0x'+'4a'.repeat(20),salt:'0x'+'11'.repeat(32),shop:'0x5FbDB2315678afecb367f032d93F642f64180aa3',chainId:84532,count:1000,validSeconds:2592000,priceUsdc:'5000000',
  eth:{...call('0x5FbDB2315678afecb367f032d93F642f64180aa3','ethereum:0x5FbDB2315678afecb367f032d93F642f64180aa3@84532/buy?address=0x4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a&bytes32=0x1111111111111111111111111111111111111111111111111111111111111111&value=1515000000000000'),quote:'1500000000000000',value:'1515000000000000'},
  usdc:{token:'0x036CbD53842c5426634e7929541eC2318f3dCF7e',amount:'5000000',approve:call('0x036CbD53842c5426634e7929541eC2318f3dCF7e','ethereum:0x036C@84532/approve'),buy:call('0x5FbDB2315678afecb367f032d93F642f64180aa3','ethereum:0x5FbD@84532/buyWithUsdc')},createdAt:1788563000};
const params=new URLSearchParams(location.search);const state=params.get('state');
if(params.has('lang'))localStorage.setItem('agentic.locale',params.get('lang')!);
if(params.has('theme'))localStorage.setItem('agentic.theme',params.get('theme')!);
if(params.has('empty'))snapshot.conversations[0].messages=[];
if(state==='onboarding'||state==='coins'||state==='elsewhere')snapshot.identity=null;
if(state==='home'||state==='onboarding'||state==='coins'||state==='elsewhere')snapshot.conversations=[];
if(state==='elsewhere')Object.assign(balance,{books:[],remaining:0,lastClaim:null});
let network=networkFixture();
// The routes the network's preset gave this start: the empty bootstrap field shows them in grey.
network.status.bootstrap.routes=['/ip4/51.91.126.3/udp/4105/quic-v1/p2p/12D3KooWGPvcE8WPX139xDitAhdHp3XekNpcFbcY4ZazCdgr2B9x','/ip4/51.91.126.3/udp/4108/quic-v1/p2p/12D3KooWPRHv7BSByJKYzBGtEoGikJbJzRzNTgTHCJLX56T2YXgN'];
let keychain=state==='keychain';
const status=():ProfileStatus=>state==='locked'?{state:'locked',secrets:'file',newProfile:false}:state==='new'?{state:'locked',secrets:'file',newProfile:true}:keychain?{state:'keychain',secrets:'keychain'}:{state:'connected',secrets:'keychain'};
let locked=state==='locked'||state==='new';let stopped=state==='stopped';
let idle=state==='failed'?{code:'start_failed',message:'profile directory must be private (0700)',retryable:true}:state==='nokeychain'?{code:'keychain_unavailable',message:'A default keychain could not be found.',retryable:false}:null;
/** kaikichat.com's preset as `?preset=` shows it: current (default),
 * unavailable, switch or update. */
let presetState=params.get('preset')??'current';
const networkPreset=():NetworkPreset=>{
  // What the live network recommends at the first run.
  const recommended=[{kind:'channel',ref:'ff'.repeat(32),owner:`ain1${'da'.repeat(32)}`,name:'Kaiki News'},{kind:'group',ref:'67'.repeat(32),owner:`ain1${'da'.repeat(32)}`,name:'Kaiki Lobby'}];
  const base:NetworkPreset={source:'preset',state:'current',network:'kaiki-base',name:'Kaiki Chat (Base)',serial:9,checkedAt:1790550000,offered:null,required:null,error:null,recommended};
  if(presetState==='unavailable')return {...base,state:'unavailable',network:null,name:null,serial:null,error:'timed out'};
  if(presetState==='switch')return {...base,state:'switch',offered:{network:'kaiki-main',name:'Kaiki main network',serial:2}};
  if(presetState==='update')return {...base,state:'update',required:'0.2.0'};
  if(presetState==='moved')return {...base,network:'kaiki-main',name:'Kaiki main network',serial:2};
  return base;
};
const refusal=(code:string,message:string,retryable=false)=>Promise.reject(coreError({code,message,retryable}));
/** The latest release as `?release=` shows it: none newer (default), newer
 * (this app replaces itself), download (it cannot) or failed (the update
 * was refused). */
const releaseState=params.get('release')??'current';
const autostartState=params.get('autostart')??'on';
/** An app outside the Applications folder as `?move=` shows it: none
 * (default), offered, or failed (the move was refused). */
const moveState=params.get('move')??'none';
const loginItem='/Users/owner/Library/LaunchAgents/com.kaikichat.app.plist';
let release:Release={current:'0.2.0',latest:releaseState==='current'?'0.2.0':'0.3.0',available:releaseState!=='current',skipped:false,checkedAt:1790550000,error:null,installable:releaseState!=='download'};
const api:DesktopApi={
  profileStatus:async()=>status(),
  unlockProfile:async()=>{locked=false;return {state:'connected',secrets:'file'};},
  reconnect:async()=>{stopped=false;idle=null;return {state:'connected',secrets:'keychain'};},
  // macOS's own dialog never shows here: going on opens the profile.
  openKeychain:async()=>{keychain=false;return {state:'connected',secrets:'keychain'};},
  networkPreset:async()=>networkPreset(),
  refreshNetwork:async({switch:move})=>{presetState=move?'moved':'current';return networkPreset();},
  release:async()=>structuredClone(release),
  checkRelease:async()=>structuredClone(release),
  skipRelease:async()=>{release={...release,skipped:true};return structuredClone(release);},
  installUpdate:async()=>releaseState==='failed'?refusal('hash_mismatch','not the announced build'):new Promise<void>(()=>{}),
  openDownloads:async()=>{},
  autostart:async()=>autostartState==='none'?null:{state:autostartState as Autostart['state'],path:loginItem},
  setAutostart:async({on})=>({state:!on?'off':autostartState==='blocked'?'blocked':'on',path:loginItem}),
  openLoginItems:async()=>{},
  moveOffer:async()=>moveState!=='none',
  // The moved app quits and opens from its new place: no answer comes.
  moveToApplications:()=>moveState==='failed'?refusal('move_failed','ditto: Permission denied'):new Promise<void>(()=>{}),
  snapshot:async()=>locked?refusal('profile_locked','locked'):keychain?refusal('keychain_consent','keychain'):idle?refusal(idle.code,idle.message,idle.retryable):stopped?refusal('daemon_unavailable','stopped',true):structuredClone(snapshot),
  conversationHistory:async({conversationId})=>({conversationId,messages:structuredClone(snapshot.conversations.find(c=>c.id===conversationId)?.messages??[]),nextBefore:null}),
  networkSettings:async()=>structuredClone(network),
  configureNetwork:async(request)=>{network={...network,revision:network.revision+1,preferences:request.preferences};return structuredClone(network);},
  listRuntimes:async()=>structuredClone(runtimes),
  provisionRuntime:async(request)=>{const runtime:RuntimeInfo={...request,grantId:Array(32).fill(1),principal:Array(32).fill(2),status:'active'};runtimes=[runtime];return {runtime,credentialsPath:'/private/profile/runtimes/example.json',cliConfig:{command:'/Applications/Kaiki Chat.app/Contents/MacOS/agentic-cli',args:['--credentials','/private/profile/runtimes/example.json']},mcpConfig:{mcpServers:{'ain-example':{command:'/Applications/Kaiki Chat.app/Contents/MacOS/agentic-mcp',args:['--credentials','/private/profile/runtimes/example.json']}}}};},
  revokeRuntime:async()=>{runtimes=runtimes.map(r=>({...r,status:'revoked'}));},
  installSkill:async({skill,host})=>({path:`/Users/alice/.${host}/skills/${skill}/SKILL.md`}),
  ownerCli:async()=>({command:'/Applications/Kaiki Chat.app/Contents/MacOS/kaiki',args:[]}),
  createInvitation:async()=>'ain-invite1:'+'a1b2c3d4'.repeat(150),
  subscribe:()=>()=>{},
  createIdentity:async({name})=>{snapshot.identity={name,networkId:own};return snapshot.identity;},
  addContact:async()=>{},
  sendMessage:async({text})=>({id:crypto.randomUUID(),author:own,own:true,text,createdAt:1788563400,delivery:{phase:'queued',replicas:0,target:10}}),
  requestContact:async({name})=>({conversationId:'anna',name}),
  introRequests:async()=>structuredClone(requests),
  acceptIntroRequest:async({requestId})=>{requests=requests.filter(r=>r.requestId!==requestId);return {conversationId:'anna',name:'Anna'};},
  rejectIntroRequest:async({requestId})=>{requests=requests.filter(r=>r.requestId!==requestId);},
  introPolicy:async()=>structuredClone(policy),
  setIntroPolicy:async(next)=>{policy=next;return structuredClone(policy);},
  groups:async()=>structuredClone(groups),
  group:async({groupId})=>structuredClone(groups.find(g=>g.id===groupId)!),
  createGroup:async({name})=>({...groups[0],id:'team',name}),
  changeGroup:async()=>({epoch:5,commit:'c',messageId:'m'}),
  doorRequests:async()=>structuredClone(doorRequests),
  doorDecide:async({requestId})=>{doorRequests=doorRequests.filter(r=>r.requestId!==requestId);},
  joinGroup:async()=>({groupId:'readers',messageId:'m'}),
  channelStorage:async({groupId})=>groupId==='news'?{retention:90,parts:4,bytes:180000,stampsPerMonth:5,addedLastMonth:1}:{retention:30,parts:0,bytes:0,stampsPerMonth:0,addedLastMonth:0},
  channelSubscribe:async({members})=>({subscribed:members}),
  coinsBalance:async()=>structuredClone(balance),
  coinsBuy:async()=>structuredClone(payment),
  claimCoins:async()=>state==='elsewhere'||state==='again'?(balance.lastClaim={status:'denied',reason:'already_claimed'},{status:'open',claimId:'c1',loginUrl:'https://id.agenticinternet.net/v1/claims/c1/login',expiresAt:1788564000}):state==='coins'?refusal('identity_not_configured','no server'):({status:'open',claimId:'c1',loginUrl:'https://id.agenticinternet.net/v1/claims/c1/login',expiresAt:1788564000}),
  openPayment:async()=>{},
  follows:async()=>structuredClone(follows),
  followGroup:async({group:ref,owner,name})=>({id:ref,name,owner,since:1788563600,closed:false}),
  unfollowGroup:async()=>{},
  discoverHandles:async({text})=>({handles:[...text.matchAll(/[\w.+-]+@[\w.-]+\.\w+/g)].map(match=>({kind:'google' as const,handle:match[0].toLowerCase()}))}),
  discoverLookup:async({handles})=>({found:handles.map((handle,index)=>({...handle,networkId:index%2===0?anna:null}))}),
  discoverLink:async()=>({linkId:'l1',loginUrl:'https://directory.kaikichat.com/v1/links/l1/login',code:'4F7K-9QX2',expiresAt:1788564000}),
  discoverStatus:async()=>({status:'pending',reason:null}),
  discoverUnlink:async()=>{},
  discoverPublish:async()=>({id:'c3'.repeat(32),expiresAt:1791155100}),
  discoverWithdraw:async()=>{},
  discoverSearch:async()=>({cards:structuredClone(cards)}),
};
createRoot(document.getElementById('root')!).render(<ChatShell api={api}/>);

// `?open=discover` opens that screen from the menu, as the owner would, and
// `&demo` fills it (addresses counted and checked, cards found), `&channel`
// makes the new group a channel anyone reads; `?chat=NAME`
// opens that conversation and `&members` its group screen.
const whenFound=<T extends Element>(find:()=>T|null|undefined,then:(element:T)=>void)=>{
  const timer=setInterval(()=>{const element=find();if(element){clearInterval(timer);then(element);}},50);
};
const typeInto=(element:HTMLInputElement|HTMLTextAreaElement,value:string)=>{
  const prototype=element instanceof HTMLTextAreaElement?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
  Object.getOwnPropertyDescriptor(prototype,'value')!.set!.call(element,value);
  element.dispatchEvent(new Event('input',{bubbles:true}));
};
const opened=params.get('open');
if(opened)whenFound(()=>document.getElementById('nav-menu'),gear=>{
  gear.click();
  whenFound(()=>document.getElementById(`nav-${opened}`),item=>{
    item.click();
    if(params.has('channel'))whenFound(()=>document.querySelector<HTMLInputElement>('input[name="group-kind"]:not(:checked)'),kind=>{
      kind.click();
      whenFound(()=>document.querySelector<HTMLInputElement>('input[name="channel-access"]'),anyone=>anyone.click());
    });
    if(!params.has('demo'))return;
    whenFound(()=>document.querySelector<HTMLElement>('.panel-stack .agent-form'),known=>{
      typeInto(known.querySelector('textarea')!,'Anna <anna@example.org>, boris@example.org\nivan@example.org');
      whenFound(()=>known.querySelector<HTMLButtonElement>('button:not(:disabled)'),count=>{
        count.click();
        whenFound(()=>known.querySelector('.claim-outcome')&&known.querySelector<HTMLButtonElement>('button:not(:disabled)'),check=>check.click());
      });
    });
    whenFound(()=>document.querySelector<HTMLFormElement>('.discover-search'),search=>{
      typeInto(search.querySelector('input')!,'rust');
      search.requestSubmit();
    });
  });
});
const chat=params.get('chat');
if(chat)whenFound(()=>[...document.querySelectorAll<HTMLButtonElement>('.conversation')].find(button=>button.textContent?.includes(chat)),button=>{
  button.click();
  if(params.has('members'))whenFound(()=>document.querySelector<HTMLButtonElement>('.header-action'),members=>members.click());
});
// `?confirm` asks to move to the offered network, up to the confirmation.
if(params.has('confirm'))whenFound(()=>[...document.querySelectorAll<HTMLButtonElement>('.preset-notice button')].find(button=>!button.classList.contains('secondary')),button=>button.click());
