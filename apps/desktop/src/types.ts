export type Delivery = {phase:'queued'|'stored'|'delivered'|'read'|'failed';replicas:number;target:number};
/** `lowTrust`: taken directly while its payment could not be checked. */
export type Message = {id:string;author:string;own:boolean;text:string;createdAt:number;delivery:Delivery;lowTrust?:boolean};
export type Identity = {name:string;networkId:string};
export type Conversation = {id:string;title:string;unread:number;messages:Message[]};
export type Snapshot = {identity:Identity|null;network:{connectedPeers:number;state:'online'|'offline'|'connecting'};conversations:Conversation[]};
export type ConversationHistory = {conversationId:string;messages:Message[];nextBefore:string|null};
export type DesktopOverview = Snapshot & {nextAfter:string|null};
export interface DesktopApi {
  profileStatus():Promise<ProfileStatus>;
  unlockProfile(request:{password:string}):Promise<ProfileStatus>;
  /** Reads the keychain key macOS guards with its dialog, once the owner goes on. */
  openKeychain():Promise<ProfileStatus>;
  reconnect():Promise<ProfileStatus>;
  networkPreset():Promise<NetworkPreset>;
  refreshNetwork(request:{switch:boolean}):Promise<NetworkPreset>;
  /** The latest release as known, after a check at most every 12 hours. */
  release():Promise<Release>;
  /** Asks kaikichat.com now. */
  checkRelease():Promise<Release>;
  skipRelease(request:{version:string}):Promise<Release>;
  /** Replaces the app with the latest release and starts it again. */
  installUpdate():Promise<void>;
  openDownloads():Promise<void>;
  /** Whether the app opens at login; null where it cannot. */
  autostart():Promise<Autostart|null>;
  setAutostart(request:{on:boolean}):Promise<Autostart>;
  /** Opens the system's login items, where the owner allows the app. */
  openLoginItems():Promise<void>;
  /** Whether the app runs outside the Applications folder and can move there. */
  moveOffer():Promise<boolean>;
  /** Moves the app into the Applications folder and opens it from there. */
  moveToApplications():Promise<void>;
  snapshot():Promise<Snapshot>;
  conversationHistory(request:{conversationId:string;before:string|null}):Promise<ConversationHistory>;
  networkSettings():Promise<NetworkSettings>;
  configureNetwork(request:{expectedRevision:number;preferences:NetworkPreferences}):Promise<NetworkSettings>;
  listRuntimes():Promise<RuntimeInfo[]>;
  provisionRuntime(request:ProvisionRuntimeRequest):Promise<RuntimeSetup>;
  revokeRuntime(request:{grantId:number[]}):Promise<void>;
  installSkill(request:{skill:SkillName;host:SkillHost}):Promise<{path:string}>;
  ownerCli():Promise<OwnerCli>;
  createInvitation():Promise<string>;
  sendMessage(request:{conversationId:string;text:string;operationId:string}):Promise<Message>;
  createIdentity(request:{name:string}):Promise<Identity>;
  addContact(request:{name:string;invitation:string}):Promise<void>;
  requestContact(request:{networkId:string;name:string;operationId:string}):Promise<{conversationId:string;name:string}>;
  introRequests():Promise<IntroRequest[]>;
  acceptIntroRequest(request:{requestId:string}):Promise<{conversationId:string;name:string}>;
  rejectIntroRequest(request:{requestId:string}):Promise<void>;
  introPolicy():Promise<IntroPolicy>;
  setIntroPolicy(request:IntroPolicy):Promise<IntroPolicy>;
  groups():Promise<Group[]>;
  group(request:{groupId:string}):Promise<Group>;
  createGroup(request:{name:string;members:string[];kind?:GroupKind;access?:GroupAccess;operationId:string}):Promise<Group>;
  changeGroup(request:GroupChange):Promise<{epoch:number;commit:string;messageId:string}>;
  doorRequests(request:{groupId:string}):Promise<DoorRequest[]>;
  doorDecide(request:{groupId:string;requestId:string;accept:boolean}):Promise<void>;
  joinGroup(request:{groupRef:string;note:string;operationId:string}):Promise<{groupId:string;messageId:string}>;
  channelStorage(request:{groupId:string}):Promise<ChannelStorage>;
  channelSubscribe(request:{groupId:string;members:string[];operationId:string}):Promise<{subscribed:string[]}>;
  coinsBalance():Promise<Balance>;
  coinsBuy():Promise<Payment>;
  claimCoins(request:{provider:LoginProvider}):Promise<ClaimOpen>;
  openPayment(request:{book:string;step:PaymentStep}):Promise<void>;
  follows():Promise<Follow[]>;
  followGroup(request:{group:string;owner:string;name:string}):Promise<Follow>;
  unfollowGroup(request:{groupId:string}):Promise<void>;
  discoverHandles(request:{text:string}):Promise<{handles:Handle[]}>;
  discoverLookup(request:{handles:Handle[]}):Promise<{found:Found[]}>;
  discoverLink(request:{kind:LoginProvider}):Promise<LinkOpen>;
  discoverStatus(request:{linkId:string}):Promise<LinkStatus>;
  discoverUnlink(request:{kind:LoginProvider}):Promise<void>;
  discoverPublish(request:CardDraft):Promise<Published>;
  discoverWithdraw(request:{cardId:string}):Promise<void>;
  discoverSearch(request:{query:string;kind?:CardKind}):Promise<{cards:Card[]}>;
  subscribe(listener:()=>void):()=>void;
}

export type RuntimeAction = 'read_inbox'|'send_message';
export type ProvisionRuntimeRequest = {operationId:string;name:string;agentId:number[];serviceId:number[];conversationIds:string[];actions:RuntimeAction[];expiresAt:number;maxDataBytes:number};
export type RuntimeInfo = Omit<ProvisionRuntimeRequest,'operationId'> & {grantId:number[];principal:number[];status:'active'|'expired'|'revoked'};
export type RuntimeCommand = {command:string;args:string[]};
export type RuntimeSetup = {runtime:RuntimeInfo;credentialsPath:string;mcpConfig:{mcpServers:Record<string,RuntimeCommand>};cliConfig:RuntimeCommand};

export type NetworkPreferences = {relays:string[];relayOnly:boolean;autoNatPeers:string[];bootstrapPeers?:string[];lanDiscovery:boolean;dhtServer:boolean};
export type NetworkSettings = {revision:number;preferences:NetworkPreferences;status:{
  peerId:string;listeners:string[];relayRoutes:string[];advertisedAddresses:string[];
  routing:{enabled:boolean;mode:'client'|'server'|'disabled';blockedByPolicy:boolean};
  lanDiscovery:{enabled:boolean;active:boolean;blockedByPolicy:boolean;peers:string[]};connectedPeers:number;holePunchEnabled:boolean;listening:boolean;
  bootstrap:{state:'connected'|'bootstrap-needed';action:string;routes:string[];networkDomain:string;verifiedPeers:{peerId:string;rootId:string}[];candidateHints:number;cachedHints:number;policyBlockedHints:number;inFlight:number;failedAttempts:number;rejectedRecords:number;rateLimitedRequests:number};
  autoNat:{status:'unknown'|'public'|'private';publicAddress:string|null;serverEnabled:boolean;probeIntervalSeconds:number;configuredPeers:string[];successfulProbes:number;failedProbes:number;refusedProbes:number;inboundProbes:number;deniedProbes:number};
}};

/** The profile's network: kaikichat.com's signed preset, flags set by hand,
 * or none (Docs/V1_NETWORK_PRESET_2026_09_28_RU.md). */
/** This app's version and the latest release the signed preset names;
 * `installable` when this app can replace itself with it. */
/** The app opening at login: `blocked` while the system's login items keep
 * it from running until the owner allows it. */
export type Autostart = {state:'on'|'off'|'blocked';path:string};
export type Release = {current:string;latest:string|null;available:boolean;skipped:boolean;checkedAt:number|null;error:string|null;installable:boolean};
/** A channel or group the network recommends to a new profile, signed in
 * its preset: a channel is followed, a group joined. */
export type Recommended = {kind:string;ref:string;owner:string;name:string};
export type NetworkPreset = {source:'preset'|'manual'|'off';state:'current'|'cached'|'unavailable'|'switch'|'update'|'manual'|'off';network:string|null;name:string|null;serial:number|null;checkedAt:number|null;offered:{network:string;name:string;serial:number}|null;required:string|null;error:string|null;recommended?:Recommended[]};
export type ProfileStatus = {state:'connected'|'locked'|'keychain'|'unavailable';secrets:'keychain'|'file';newProfile?:boolean;error?:{code:string;message:string}|null};
export type IntroMode = 'all'|'list'|'manual';
export type IntroPolicy = {mode:IntroMode;dailyLimit:number;allowed:string[]};
export type IntroRequest = {requestId:string;networkId:string;name:string;receivedAt:number;group?:string};
export type GroupRole = 'owner'|'admin'|'member';
/** An id banned from a group; only the owner lifts the owner's bans. */
export type BannedId = {id:string;byOwner:boolean};
/** Who reads a group: its members, those let in at its door, or anyone
 * (an open group). */
export type GroupAccess = 'private'|'request'|'public';
/** A group, or a channel: written by its owner and admins only. */
export type GroupKind = 'group'|'channel';
/** Days a channel keeps its history, or for ever. */
export type Retention = 30|90|180|365|'forever';
export type Group = {id:string;name:string;epoch:number;owner:string;admins:string[];members:string[];role:GroupRole;banned:BannedId[];access:GroupAccess;groupRef:string;kind:GroupKind;retention:number|null};
export type GroupChange = {groupId:string;add?:string[];remove?:string[];admins?:string[];ban?:string[];unban?:string[];access?:GroupAccess;retention?:Retention;unsubscribe?:string[];reseed?:boolean;operationId:string};
/** An application waiting at a group's door; its note is a stranger's text. */
export type DoorRequest = {requestId:string;networkId:string;note:string;receivedAt:number;rejoin:boolean};
/** What keeping a channel's history costs. */
export type ChannelStorage = {retention:number|null;parts:number;bytes:number;stampsPerMonth:number;addedLastMonth:number};
/** An open group or a channel this profile reads without being a member. */
export type Follow = {id:string;name:string;owner:string;since:number;closed:boolean;kind?:GroupKind;retention?:number|null;sealed?:boolean};
/** An address or a GitHub login, in its normal form. */
export type Handle = {kind:LoginProvider;handle:string};
export type Found = Handle & {networkId:string|null};
export type LinkOpen = {linkId:string;loginUrl:string;code:string;expiresAt:number};
export type LinkStatus = {status:'pending'|'linked'|'denied';reason:string|null};
export type CardKind = 'group'|'channel'|'profile';
/** A card of the discovery service, checked: what its author wrote. */
export type Card = {id:string;kind:CardKind;name:string;about:string;tags:string[];langs:string[];owner:string;groupRef:string|null;expiresAt:number};
export type CardDraft = {kind:CardKind;groupId?:string;about:string;tags:string[];langs:string[]};
export type Published = {id:string;expiresAt:number};
export type Book = {book:string;kind:'granted'|'bought';count:number;used:number;validUntil:number};
export type ClaimOpen = {status:'open';claimId:string;loginUrl:string;expiresAt:number};
export type ClaimState = {status:'open'|'starting';claimId:string|null;loginUrl:string|null;expiresAt:number|null};
export type ClaimOutcome = {status:'granted';book:string}|{status:'denied';reason:string};
export type Balance = {books:Book[];pending:{book:string;key:string;salt:string;createdAt:number}[];remaining:number;claim:ClaimState|null;lastClaim:ClaimOutcome|null};
export type PaymentCall = {to:string;calldata:string;uri:string};
export type PaymentStep = 'eth'|'approve'|'buy';
/** A book to buy, priced in USD: in ETH at the feed's quote (`null` while the
 * rate is stale), or in USDC in two calls. */
export type Payment = {book:string;key:string;salt:string;shop:string;chainId:number;count:number;validSeconds:number;priceUsdc:string;eth:(PaymentCall&{quote:string;value:string})|null;usdc:{token:string;amount:string;approve:PaymentCall;buy:PaymentCall};createdAt:number};
export type SkillName = 'kaiki'|'agentic-messaging';
/** How an agent runs the owner CLI on this profile: the app's bundle is not on PATH. */
export type OwnerCli = {command:string;args:string[]};
export type SkillHost = 'claude'|'codex';
export type LoginProvider = 'google'|'github';
