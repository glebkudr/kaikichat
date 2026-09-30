import {describe,it,expect,vi} from 'vitest';
import {createDesktopApi} from '../src/desktop-api';

describe('desktop core IPC adapter',()=>{
  it('maps commands without granting arbitrary signing or filesystem access',async()=>{
    const invoke=vi.fn(async(_command:string,_args?:Record<string,unknown>):Promise<unknown>=>({identity:{networkId:'ain1real',name:'Alice'},network:{state:'offline',connectedPeers:0},conversations:[],nextAfter:null}));
    const api=createDesktopApi({invoke,listen:async()=>()=>{}});
    await api.snapshot(); await api.createIdentity({name:'Alice'}); await api.createInvitation();
    await api.addContact({name:'Bob',invitation:'signed-invitation'});
    await api.sendMessage({conversationId:'room',text:'hello',operationId:'op1'});
    expect(invoke.mock.calls).toEqual([
      ['desktop_overview',{request:{after:null}}],['create_identity',{request:{name:'Alice'}}],['create_invitation',{request:{}}],
      ['add_contact',{request:{name:'Bob',invitation:'signed-invitation'}}],
      ['send_message',{request:{conversationId:'room',text:'hello',operationId:'op1'}}],
    ]);
  });
  it('collects bounded overview pages and forwards the exact exclusive history cursor',async()=>{
    const identity={name:'Alice',networkId:'ain1alice'};const network={state:'offline',connectedPeers:0};
    const a={id:'a',title:'A',unread:1,messages:[]};const b={id:'b',title:'B',unread:0,messages:[]};
    const invoke=vi.fn().mockResolvedValueOnce({identity,network,conversations:[a],nextAfter:'a'}).mockResolvedValueOnce({identity,network,conversations:[b],nextAfter:null}).mockResolvedValueOnce({conversationId:'b',messages:[],nextBefore:null});
    const api=createDesktopApi({invoke,listen:async()=>()=>{}});
    expect(await api.snapshot()).toEqual({identity,network,conversations:[a,b]});
    expect(await api.conversationHistory({conversationId:'b',before:'message-id'})).toEqual({conversationId:'b',messages:[],nextBefore:null});
    expect(invoke.mock.calls).toEqual([['desktop_overview',{request:{after:null}}],['desktop_overview',{request:{after:'a'}}],['conversation_history',{request:{conversationId:'b',before:'message-id'}}]]);
  });
  it('unsubscribes even when the component closes before listener registration resolves',async()=>{
    let complete:((stop:()=>void)=>void)|undefined; const stop=vi.fn(); const listener=vi.fn();
    const listen=vi.fn(()=>new Promise<()=>void>(resolve=>{complete=resolve;}));
    const api=createDesktopApi({invoke:vi.fn(),listen});
    const unsubscribe=api.subscribe(listener); unsubscribe(); complete?.(stop);
    await Promise.resolve(); expect(stop).toHaveBeenCalledOnce();
    expect(listen).toHaveBeenCalledWith('core:changed',listener);
  });
  it('preserves a rejected core command instead of synthesizing success',async()=>{
    const api=createDesktopApi({invoke:vi.fn().mockRejectedValue(new Error('denied')),listen:async()=>()=>{}});
    await expect(api.sendMessage({conversationId:'x',text:'hi',operationId:'op'})).rejects.toThrow('denied');
  });
});

it('maps runtime provisioning, listing and exact revocation through the owner bridge',async()=>{
  const invoke=vi.fn(async()=>null);const api=createDesktopApi({invoke,listen:async()=>()=>{}});
  const request={operationId:'setup-1',name:'Assistant',agentId:Array(32).fill(1),serviceId:Array(32).fill(2),conversationIds:['ab'.repeat(32)],actions:['read_inbox' as const],expiresAt:1800000000,maxDataBytes:4096};
  await api.listRuntimes();await api.provisionRuntime(request);await api.revokeRuntime({grantId:Array(32).fill(3)});
  expect(invoke.mock.calls).toEqual([['list_runtimes'],['provision_runtime',{request}],['revoke_runtime',{request:{grantId:Array(32).fill(3)}}]]);
});

it('passes explicit owner network settings and CAS revision without extra privileges',async()=>{
  const invoke=vi.fn(async()=>null);const api=createDesktopApi({invoke,listen:async()=>()=>{}});
  const request={expectedRevision:7,preferences:{relays:[],relayOnly:false,autoNatPeers:[],bootstrapPeers:["/ip4/203.0.113.10/tcp/4001/p2p/peer"],lanDiscovery:true,dhtServer:false}};
  await api.networkSettings();await api.configureNetwork(request);
  expect(invoke.mock.calls).toEqual([['network_settings'],['configure_network',{request}]]);
});



it('maps the owner screens to their own commands and turns refusals into coded errors',async()=>{
  const invoke=vi.fn(async(_command:string,_args?:Record<string,unknown>):Promise<unknown>=>null);
  const api=createDesktopApi({invoke,listen:async()=>()=>{}});
  await api.requestContact({networkId:'ain1x',name:'Bob',operationId:'op'});await api.setIntroPolicy({mode:'manual',dailyLimit:3,allowed:[]});
  await api.changeGroup({groupId:'g',remove:['ain1y'],operationId:'op2'});await api.coinsBuy();await api.claimCoins({provider:'github'});
  await api.openPayment({book:'0xb',step:'approve'});await api.unlockProfile({password:'pw'});await api.openKeychain();await api.installSkill({skill:'kaiki',host:'codex'});
  expect(invoke.mock.calls).toEqual([
    ['request_contact',{request:{networkId:'ain1x',name:'Bob',operationId:'op'}}],['set_intro_policy',{request:{mode:'manual',dailyLimit:3,allowed:[]}}],
    ['change_group',{request:{groupId:'g',remove:['ain1y'],operationId:'op2'}}],['coins_buy'],['claim_coins',{request:{provider:'github'}}],
    ['open_payment',{request:{book:'0xb',step:'approve'}}],['unlock_profile',{request:{password:'pw'}}],['open_keychain'],['install_skill',{request:{skill:'kaiki',host:'codex'}}],
  ]);
  invoke.mockRejectedValueOnce({code:'card_pending',message:'looking',retryable:true});
  await expect(api.requestContact({networkId:'ain1x',name:'Bob',operationId:'op'})).rejects.toMatchObject({code:'card_pending',retryable:true,message:'looking'});
});
