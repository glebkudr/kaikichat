import {vi} from 'vitest';
import {networkFixture} from './network-fixture';
import type {Autostart,Balance,DesktopApi,Group,IntroPolicy,NetworkPreset,Release} from '../src/types';

export const ownId=`ain1${'a1'.repeat(32)}`;
export const bobId=`ain1${'b2'.repeat(32)}`;
export const carolId=`ain1${'c3'.repeat(32)}`;
/** The owner CLI inside the macOS app bundle, off PATH and with a space. */
export const appCli='/Applications/Kaiki Chat.app/Contents/MacOS/kaiki';
export const emptyBalance=():Balance=>({books:[],pending:[],remaining:0,claim:null,lastClaim:null});
export const policy=():IntroPolicy=>({mode:'all',dailyLimit:100,allowed:[]});
/** The public testnet as kaikichat.com's preset gives it. */
export const preset=(overrides:Partial<NetworkPreset>={}):NetworkPreset=>({source:'preset',state:'current',network:'kaiki-testnet-base-sepolia',name:'Kaiki testnet (Base Sepolia)',serial:1,checkedAt:1790550000,offered:null,required:null,error:null,...overrides});
/** This app, the latest release, checked from kaikichat.com. */
export const release=(overrides:Partial<Release>={}):Release=>({current:'0.2.0',latest:'0.2.0',available:false,skipped:false,checkedAt:1790550000,error:null,installable:true,...overrides});
/** The app opening at login from the owner's launchd agent. */
export const autostart=(overrides:Partial<Autostart>={}):Autostart=>({state:'on',path:'/Users/owner/Library/LaunchAgents/com.kaikichat.app.plist',...overrides});
export const group=(overrides:Partial<Group>={}):Group=>({id:'g1',name:'Team',epoch:3,owner:ownId,admins:[],members:[ownId,bobId,carolId],role:'owner',banned:[],access:'private',groupRef:'ef'.repeat(32),kind:'group',retention:1,...overrides});

/** Every owner command as a mock with a harmless answer; tests override
 * what they exercise. */
export function fakeApi() {
  return {
    profileStatus:vi.fn<DesktopApi['profileStatus']>(async()=>({state:'connected' as const,secrets:'keychain' as const})),
    unlockProfile:vi.fn<DesktopApi['unlockProfile']>(async()=>({state:'connected' as const,secrets:'file' as const})),
    reconnect:vi.fn<DesktopApi['reconnect']>(async()=>({state:'connected' as const,secrets:'keychain' as const})),
    openKeychain:vi.fn<DesktopApi['openKeychain']>(async()=>({state:'connected' as const,secrets:'keychain' as const})),
    networkPreset:vi.fn<DesktopApi['networkPreset']>(async()=>preset()),
    refreshNetwork:vi.fn<DesktopApi['refreshNetwork']>(async()=>preset()),
    release:vi.fn<DesktopApi['release']>(async()=>release()),
    checkRelease:vi.fn<DesktopApi['checkRelease']>(async()=>release()),
    skipRelease:vi.fn<DesktopApi['skipRelease']>(async({version})=>release({latest:version,available:true,skipped:true})),
    installUpdate:vi.fn<DesktopApi['installUpdate']>(async()=>undefined),
    openDownloads:vi.fn<DesktopApi['openDownloads']>(async()=>undefined),
    autostart:vi.fn<DesktopApi['autostart']>(async()=>autostart()),
    setAutostart:vi.fn<DesktopApi['setAutostart']>(async({on})=>autostart({state:on?'on':'off'})),
    openLoginItems:vi.fn<DesktopApi['openLoginItems']>(async()=>undefined),
    snapshot:vi.fn<DesktopApi['snapshot']>(async()=>({identity:{name:'Alice',networkId:ownId},network:{connectedPeers:3,state:'online' as const},conversations:[]})),
    conversationHistory:vi.fn<DesktopApi['conversationHistory']>(async({conversationId}:{conversationId:string;before:string|null})=>({conversationId,messages:[],nextBefore:null as string|null})),
    networkSettings:vi.fn<DesktopApi['networkSettings']>(async()=>networkFixture()),configureNetwork:vi.fn<DesktopApi['configureNetwork']>(async()=>networkFixture()),
    listRuntimes:vi.fn<DesktopApi['listRuntimes']>(async()=>[]),provisionRuntime:vi.fn<DesktopApi['provisionRuntime']>(),revokeRuntime:vi.fn<DesktopApi['revokeRuntime']>(async()=>undefined),
    installSkill:vi.fn<DesktopApi['installSkill']>(async({skill,host})=>({path:`/home/owner/.${host}/skills/${skill}/SKILL.md`})),
    ownerCli:vi.fn<DesktopApi['ownerCli']>(async()=>({command:appCli,args:[]})),
    createInvitation:vi.fn<DesktopApi['createInvitation']>(async()=>'ain-invite1:signed'),
    sendMessage:vi.fn<DesktopApi['sendMessage']>(async(request:{conversationId:string;text:string;operationId:string})=>({id:'sent',author:ownId,own:true,text:request.text,createdAt:101,delivery:{phase:'queued' as const,replicas:0,target:10}})),
    createIdentity:vi.fn<DesktopApi['createIdentity']>(async({name}:{name:string})=>({name,networkId:ownId})),
    addContact:vi.fn<DesktopApi['addContact']>(async()=>undefined),
    requestContact:vi.fn<DesktopApi['requestContact']>(async({name}:{networkId:string;name:string;operationId:string})=>({conversationId:'new-contact',name})),
    introRequests:vi.fn<DesktopApi['introRequests']>(async()=>[]),
    acceptIntroRequest:vi.fn<DesktopApi['acceptIntroRequest']>(async()=>({conversationId:'accepted',name:'Bob'})),
    rejectIntroRequest:vi.fn<DesktopApi['rejectIntroRequest']>(async()=>undefined),
    introPolicy:vi.fn<DesktopApi['introPolicy']>(async()=>policy()),
    setIntroPolicy:vi.fn<DesktopApi['setIntroPolicy']>(async(next:IntroPolicy)=>next),
    groups:vi.fn<DesktopApi['groups']>(async():Promise<Group[]>=>[]),
    group:vi.fn<DesktopApi['group']>(async()=>group()),
    createGroup:vi.fn<DesktopApi['createGroup']>(async({name}:{name:string;members:string[];operationId:string})=>group({id:'made',name})),
    changeGroup:vi.fn<DesktopApi['changeGroup']>(async()=>({epoch:4,commit:'c',messageId:'m'})),
    doorRequests:vi.fn<DesktopApi['doorRequests']>(async()=>[]),
    doorDecide:vi.fn<DesktopApi['doorDecide']>(async()=>undefined),
    joinGroup:vi.fn<DesktopApi['joinGroup']>(async()=>({groupId:'g1',messageId:'m'})),
    channelStorage:vi.fn<DesktopApi['channelStorage']>(async()=>({retention:30,parts:0,bytes:0,stampsPerMonth:0,addedLastMonth:0})),
    channelSubscribe:vi.fn<DesktopApi['channelSubscribe']>(async({members}:{groupId:string;members:string[];operationId:string})=>({subscribed:members})),
    coinsBalance:vi.fn<DesktopApi['coinsBalance']>(async()=>emptyBalance()),
    coinsBuy:vi.fn<DesktopApi['coinsBuy']>(),
    claimCoins:vi.fn<DesktopApi['claimCoins']>(),
    openPayment:vi.fn<DesktopApi['openPayment']>(async()=>undefined),
    follows:vi.fn<DesktopApi['follows']>(async()=>[]),
    followGroup:vi.fn<DesktopApi['followGroup']>(async({group:ref,owner,name}:{group:string;owner:string;name:string})=>({id:ref,name,owner,since:1788563000,closed:false})),
    unfollowGroup:vi.fn<DesktopApi['unfollowGroup']>(async()=>undefined),
    discoverHandles:vi.fn<DesktopApi['discoverHandles']>(async()=>({handles:[]})),
    discoverLookup:vi.fn<DesktopApi['discoverLookup']>(async()=>({found:[]})),
    discoverLink:vi.fn<DesktopApi['discoverLink']>(),
    discoverStatus:vi.fn<DesktopApi['discoverStatus']>(async()=>({status:'pending' as const,reason:null})),
    discoverUnlink:vi.fn<DesktopApi['discoverUnlink']>(async()=>undefined),
    discoverPublish:vi.fn<DesktopApi['discoverPublish']>(async()=>({id:'c1'.repeat(32),expiresAt:1791155000})),
    discoverWithdraw:vi.fn<DesktopApi['discoverWithdraw']>(async()=>undefined),
    discoverSearch:vi.fn<DesktopApi['discoverSearch']>(async()=>({cards:[]})),
    subscribe:vi.fn<DesktopApi['subscribe']>((_listener:()=>void)=>()=>undefined),
  } satisfies DesktopApi;
}
