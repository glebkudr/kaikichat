import type {Autostart,DesktopApi,Identity,Message,Snapshot,DesktopOverview,ConversationHistory,NetworkPreset,Release,NetworkSettings,RuntimeInfo,RuntimeSetup,ProfileStatus,IntroRequest,IntroPolicy,Group,DoorRequest,ChannelStorage,Balance,Payment,ClaimOpen,OwnerCli,Follow,Handle,Found,LinkOpen,LinkStatus,Published,Card} from './types';
import {CoreError,coreError} from './core-error';
export interface CoreTransport {
  invoke(command:string,args?:Record<string,unknown>):Promise<unknown>;
  listen(event:string,listener:()=>void):Promise<()=>void>;
}
export function createDesktopApi(core:CoreTransport):DesktopApi {
  const transport:CoreTransport={invoke:(command,args)=>(args===undefined?core.invoke(command):core.invoke(command,args)).catch(error=>{throw coreError(error);}),listen:core.listen};
  return {
    profileStatus:()=>transport.invoke('profile_status') as Promise<ProfileStatus>,
    unlockProfile:request=>transport.invoke('unlock_profile',{request}) as Promise<ProfileStatus>,
    reconnect:()=>transport.invoke('reconnect') as Promise<ProfileStatus>,
    networkPreset:()=>transport.invoke('network_preset') as Promise<NetworkPreset>,
    refreshNetwork:request=>transport.invoke('refresh_network',{request}) as Promise<NetworkPreset>,
    release:()=>transport.invoke('release_status') as Promise<Release>,
    checkRelease:()=>transport.invoke('check_release') as Promise<Release>,
    skipRelease:request=>transport.invoke('skip_release',{request}) as Promise<Release>,
    installUpdate:async()=>{await transport.invoke('install_update');},
    openDownloads:async()=>{await transport.invoke('open_downloads');},
    autostart:()=>transport.invoke('autostart_status') as Promise<Autostart|null>,
    setAutostart:request=>transport.invoke('set_autostart',{request}) as Promise<Autostart>,
    openLoginItems:async()=>{await transport.invoke('open_login_items');},
    installSkill:request=>transport.invoke('install_skill',{request}) as Promise<{path:string}>,
    ownerCli:()=>transport.invoke('owner_cli') as Promise<OwnerCli>,
    requestContact:request=>transport.invoke('request_contact',{request}) as Promise<{conversationId:string;name:string}>,
    introRequests:()=>transport.invoke('intro_requests') as Promise<IntroRequest[]>,
    acceptIntroRequest:request=>transport.invoke('accept_intro_request',{request}) as Promise<{conversationId:string;name:string}>,
    rejectIntroRequest:async request=>{await transport.invoke('reject_intro_request',{request});},
    introPolicy:()=>transport.invoke('intro_policy') as Promise<IntroPolicy>,
    setIntroPolicy:request=>transport.invoke('set_intro_policy',{request}) as Promise<IntroPolicy>,
    groups:()=>transport.invoke('groups') as Promise<Group[]>,
    group:request=>transport.invoke('group',{request}) as Promise<Group>,
    createGroup:request=>transport.invoke('create_group',{request}) as Promise<Group>,
    changeGroup:request=>transport.invoke('change_group',{request}) as Promise<{epoch:number;commit:string;messageId:string}>,
    doorRequests:request=>transport.invoke('door_requests',{request}) as Promise<DoorRequest[]>,
    doorDecide:async request=>{await transport.invoke('door_decide',{request});},
    joinGroup:request=>transport.invoke('join_group',{request}) as Promise<{groupId:string;messageId:string}>,
    channelStorage:request=>transport.invoke('channel_storage',{request}) as Promise<ChannelStorage>,
    channelSubscribe:request=>transport.invoke('channel_subscribe',{request}) as Promise<{subscribed:string[]}>,
    coinsBalance:()=>transport.invoke('coins_balance') as Promise<Balance>,
    coinsBuy:()=>transport.invoke('coins_buy') as Promise<Payment>,
    claimCoins:request=>transport.invoke('claim_coins',{request}) as Promise<ClaimOpen>,
    openPayment:async request=>{await transport.invoke('open_payment',{request});},
    follows:()=>transport.invoke('follows') as Promise<Follow[]>,
    followGroup:request=>transport.invoke('follow_group',{request}) as Promise<Follow>,
    unfollowGroup:async request=>{await transport.invoke('unfollow_group',{request});},
    discoverHandles:request=>transport.invoke('discover_handles',{request}) as Promise<{handles:Handle[]}>,
    discoverLookup:request=>transport.invoke('discover_lookup',{request}) as Promise<{found:Found[]}>,
    discoverLink:request=>transport.invoke('discover_link',{request}) as Promise<LinkOpen>,
    discoverStatus:request=>transport.invoke('discover_status',{request}) as Promise<LinkStatus>,
    discoverUnlink:async request=>{await transport.invoke('discover_unlink',{request});},
    discoverPublish:request=>transport.invoke('discover_publish',{request}) as Promise<Published>,
    discoverWithdraw:async request=>{await transport.invoke('discover_withdraw',{request});},
    discoverSearch:request=>transport.invoke('discover_search',{request}) as Promise<{cards:Card[]}>,
    networkSettings:()=>transport.invoke('network_settings') as Promise<NetworkSettings>,
    configureNetwork:request=>transport.invoke('configure_network',{request}) as Promise<NetworkSettings>,
    listRuntimes:()=>transport.invoke('list_runtimes') as Promise<RuntimeInfo[]>,
    provisionRuntime:request=>transport.invoke('provision_runtime',{request}) as Promise<RuntimeSetup>,
    revokeRuntime:request=>transport.invoke('revoke_runtime',{request}) as Promise<void>,
    async snapshot():Promise<Snapshot> {
      let after:string|null=null;
      let result:Snapshot|undefined;
      do {
        const page=await transport.invoke('desktop_overview',{request:{after}}) as DesktopOverview;
        result=result?{...result,conversations:[...result.conversations,...page.conversations]}:{identity:page.identity,network:page.network,conversations:page.conversations};
        if(page.nextAfter!==null&&after!==null&&page.nextAfter<=after)throw new CoreError('cursor_stuck','cursor_stuck',false);
        after=page.nextAfter;
      } while(after!==null);
      return result;
    },
    conversationHistory:request=>transport.invoke('conversation_history',{request}) as Promise<ConversationHistory>,
    createInvitation:()=>transport.invoke('create_invitation',{request:{}}) as Promise<string>,
    createIdentity:request=>transport.invoke('create_identity',{request}) as Promise<Identity>,
    addContact:request=>transport.invoke('add_contact',{request}) as Promise<void>,
    sendMessage:request=>transport.invoke('send_message',{request}) as Promise<Message>,
    subscribe(listener) {
      let closed=false;let stop:(()=>void)|undefined;
      void transport.listen('core:changed',listener).then(unsubscribe=>{if(closed)unsubscribe();else stop=unsubscribe;}).catch(()=>{if(!closed)listener();});
      return ()=>{closed=true;stop?.();};
    },
  };
}
