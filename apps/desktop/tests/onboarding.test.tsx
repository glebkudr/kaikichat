import {render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,describe,expect,it,vi} from 'vitest';
import {ChatShell} from '../src/ChatShell';
import {ContactsPanel} from '../src/ContactsPanel';
import {parseInvitation,shellWord} from '../src/Connect';
import {coreError} from '../src/core-error';
import {locales} from '../src/i18n';
import type {Balance} from '../src/types';
import {appCli,bobId,emptyBalance,fakeApi,ownId} from './fake-api';

afterEach(()=>localStorage.clear());
const refusal=(code:string,retryable=false)=>coreError({code,message:code,retryable});
const granted=():Balance=>({...emptyBalance(),remaining:10000,books:[{book:'0x1',kind:'granted',count:10000,used:0,validUntil:1791155100}],lastClaim:{status:'granted',book:'0x1'}});
const noProfile={identity:null,network:{connectedPeers:0,state:'offline' as const},conversations:[]};
const alice={identity:{name:'Alice',networkId:ownId},network:{connectedPeers:3,state:'online' as const},conversations:[]};

/** A new owner, with the node's snapshot following the profile's creation. */
function newOwner() {
  const api=fakeApi();
  api.snapshot.mockResolvedValue(structuredClone(noProfile));
  api.createIdentity.mockImplementation(async({name})=>{api.snapshot.mockResolvedValue({...structuredClone(alice),identity:{name,networkId:ownId}});return {name,networkId:ownId};});
  api.claimCoins.mockResolvedValue({status:'open',claimId:'c1',loginUrl:'https://id.example/v1/claims/c1/login',expiresAt:2});
  return api;
}

describe('first run: one action per screen',()=>{
  it('explains the product, names the owner, gets free messages, connects the agent and invites friends',async()=>{
    const api=newOwner();const user=userEvent.setup();
    api.coinsBalance.mockResolvedValueOnce(emptyBalance()).mockResolvedValue(granted());
    render(<ChatShell api={api}/>);
    expect(await screen.findByRole('heading',{name:'Let your AI agent talk to your friends’ agents.'})).toBeVisible();
    // Nothing else competes for the first screens: no conversation list.
    expect(screen.queryByRole('heading',{name:'Messages'})).not.toBeInTheDocument();
    expect(screen.getByText('Step 1 of 5')).toBeInTheDocument();
    await user.click(screen.getByRole('button',{name:'Get started'}));

    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice{Enter}');
    expect(api.createIdentity).toHaveBeenCalledWith({name:'Alice'});

    expect(await screen.findByRole('heading',{name:'Get free messages with Google or GitHub'})).toBeVisible();
    await user.click(screen.getByRole('button',{name:'Log in with Google'}));
    expect(api.claimCoins).toHaveBeenCalledWith({provider:'google'});
    // The grant arrives from the browser login; the app goes on by itself.
    expect(await screen.findByRole('heading',{name:'Connect your agent'})).toBeVisible();

    const instruction=await screen.findByRole('textbox',{name:'Instructions for your agent'});
    await waitFor(()=>expect((instruction as HTMLTextAreaElement).value).toContain(`command-line tool is '${appCli}'.`));
    await user.click(screen.getByRole('button',{name:'Copy instructions'}));
    expect(await navigator.clipboard.readText()).toBe((instruction as HTMLTextAreaElement).value);
    expect(screen.getByRole('status')).toHaveTextContent('Copied. Paste it into Claude Code or Codex.');
    await user.click(screen.getByRole('button',{name:'Next'}));

    expect(await screen.findByRole('heading',{name:'Invite your friends'})).toBeVisible();
    await user.click(screen.getByRole('button',{name:'Copy invitation'}));
    const invitation=await navigator.clipboard.readText();
    expect(parseInvitation(invitation)).toEqual({networkId:ownId,name:'Alice'});
    expect(invitation).toContain('https://kaikichat.com');
    await user.click(screen.getByRole('button',{name:'Done'}));

    expect(await screen.findByRole('heading',{name:'Your agent needs friends to talk to.'})).toBeVisible();
    expect(screen.getByRole('heading',{name:'Messages'})).toBeVisible();
  });

  it('keeps the owner on the login screen with the reason when the server refuses',async()=>{
    const api=newOwner();const user=userEvent.setup();
    api.coinsBalance.mockResolvedValueOnce(emptyBalance()).mockResolvedValue({...emptyBalance(),lastClaim:{status:'denied',reason:'email_not_verified'}});
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice{Enter}');
    await user.click(await screen.findByRole('button',{name:'Log in with GitHub'}));
    expect(await screen.findByText('The server refused: the account’s email is not verified.')).toBeVisible();
    expect(screen.getByRole('heading',{name:'Get free messages with Google or GitHub'})).toBeVisible();
    expect(screen.getByRole('button',{name:'Log in with Google'})).toBeEnabled();
  });
  it('tells an owner whose login already got its messages on another device that this one is a separate account',async()=>{
    const api=newOwner();const user=userEvent.setup();
    api.coinsBalance.mockResolvedValueOnce(emptyBalance()).mockResolvedValue({...emptyBalance(),lastClaim:{status:'denied',reason:'already_claimed'}});
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice{Enter}');
    await user.click(await screen.findByRole('button',{name:'Log in with Google'}));
    const headline=await screen.findByText('You have already logged in to Kaiki Chat with this Google or GitHub account, probably on another device.');
    expect(headline).toBeVisible();
    const outcome=headline.closest('[role=status]');
    expect(outcome).toHaveTextContent('your account is a private key kept on the device where you created it');
    expect(outcome).toHaveTextContent('Moving the key to this device with a QR code comes in a future version');
    expect(outcome).not.toHaveTextContent('The server refused');
    expect(screen.getByRole('button',{name:'Or use anonymously via Crypto'})).toBeEnabled();
  });

  it('lets an anonymous owner top up with crypto instead, and one without a server skip',async()=>{
    const api=newOwner();const user=userEvent.setup();
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice{Enter}');
    await user.click(await screen.findByRole('button',{name:'Or use anonymously via Crypto'}));
    expect(await screen.findByRole('region',{name:'Wallet'})).toBeVisible();
    expect(screen.getByRole('heading',{name:'Messages'})).toBeVisible();
  });

  it('says when the network grants no free messages and goes on without them',async()=>{
    const api=newOwner();const user=userEvent.setup();
    api.claimCoins.mockRejectedValue(refusal('identity_not_configured'));
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice{Enter}');
    await user.click(await screen.findByRole('button',{name:'Log in with Google'}));
    expect(await screen.findByText('The node knows no identity server: logging in for coins is unavailable.')).toBeVisible();
    await user.click(screen.getByRole('button',{name:'Skip for now'}));
    expect(await screen.findByRole('heading',{name:'Connect your agent'})).toBeVisible();
  });

  it('asks to move the app first when macOS runs it from a disk image or a quarantine copy',async()=>{
    const api=newOwner();const user=userEvent.setup();
    api.ownerCli.mockResolvedValue({command:'/private/var/folders/x/T/AppTranslocation/0A1B/d/Kaiki Chat.app/Contents/MacOS/kaiki',args:[]});
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice{Enter}');
    await user.click(await screen.findByRole('button',{name:'Skip for now'}));
    expect(await screen.findByText(locales.en.agentSetup.moveApp)).toBeVisible();
  });
  it('skips the name when the profile was made meanwhile, for example by the CLI',async()=>{
    const api=newOwner();const user=userEvent.setup();
    let notify=()=>{};api.subscribe.mockImplementation(listener=>{notify=listener;return ()=>{};});
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await screen.findByRole('textbox',{name:'Your name'});
    api.snapshot.mockResolvedValue(structuredClone(alice));
    notify();
    expect(await screen.findByRole('heading',{name:'Get free messages with Google or GitHub'})).toBeVisible();
  });
  it('gives the agent a profile the CLI would not find by itself, as shell words',async()=>{
    expect(shellWord('/usr/bin/kaiki')).toBe("'/usr/bin/kaiki'");
    expect(shellWord("/Users/o'brien/kaiki")).toBe("'/Users/o'\\''brien/kaiki'");
    expect(shellWord('--data-dir')).toBe('--data-dir');
    const api=newOwner();const user=userEvent.setup();
    api.ownerCli.mockResolvedValue({command:appCli,args:['--data-dir','/Users/alice/Test profile']});
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice{Enter}');
    await user.click(await screen.findByRole('button',{name:'Skip for now'}));
    const instruction=await screen.findByRole('textbox',{name:'Instructions for your agent'});
    await waitFor(()=>expect((instruction as HTMLTextAreaElement).value).toContain(`command-line tool is '${appCli}' --data-dir '/Users/alice/Test profile'.`));
  });
});

describe('invitations between friends',()=>{
  it('names the app Kaiki Chat on the first screen in every language',()=>{
    for(const [code,table] of Object.entries(locales))expect(table.welcome.eyebrow,code).toBe('KAIKI CHAT');
  });
  it('reads the id and the name from an invitation in any language, or a bare id',()=>{
    for(const table of Object.values(locales)) {
      expect(parseInvitation(table.invite.message('Bob',bobId))).toEqual({networkId:bobId,name:'Bob'});
      expect(parseInvitation(`> ${table.invite.message('Bob, the builder',bobId)}\n> sent from my phone`)).toEqual({networkId:bobId,name:'Bob, the builder'});
    }
    expect(parseInvitation(`  ${bobId}\n`)).toEqual({networkId:bobId,name:''});
    expect(parseInvitation('Hi, this is Bob, my ID was somewhere')).toBeNull();
  });
  it('reads names and ids as sent: a label inside a name, direction marks, long names, longer hex',()=>{
    // "Nomad" ends with the Turkish label "ad"; the Chinese label comes first.
    expect(parseInvitation(locales.zh.invite.message('Nomad: Z',bobId))).toEqual({networkId:bobId,name:'Nomad: Z'});
    // Messengers mark the direction of Arabic text around the colon.
    expect(parseInvitation(locales.ar.invite.message('\u200fسارة',bobId).replace('ID:','ID:\u200e'))).toEqual({networkId:bobId,name:'سارة'});
    const emoji='A'+'🙂'.repeat(100);
    expect(parseInvitation(locales.en.invite.message(emoji,bobId))?.name).toBe(Array.from(emoji).slice(0,80).join(''));
    // An id inside a longer word or hex string is not an id.
    expect(parseInvitation(`${bobId}0`)).toBeNull();
    expect(parseInvitation(`x${bobId}`)).toBeNull();
    expect(parseInvitation(bobId.toUpperCase().replace('AIN1','ain1'))).toEqual({networkId:bobId,name:''});
  });

  it('adds a friend from the invitation pasted on the start screen, in a Russian invitation to an English app',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.snapshot.mockResolvedValue(structuredClone(alice));
    api.requestContact.mockResolvedValue({conversationId:'bob',name:'Bob'});
    render(<ChatShell api={api}/>);
    const paste=await screen.findByRole('textbox',{name:'Invitation or ID'});
    await user.click(paste);await user.paste(locales.ru.invite.message('Bob',bobId));
    expect(screen.getByRole('textbox',{name:'What to call them'})).toHaveValue('Bob');
    await user.click(screen.getByRole('button',{name:'Add friend'}));
    expect(api.requestContact).toHaveBeenCalledWith({networkId:bobId,name:'Bob',operationId:expect.any(String)});
  });

  it('refuses a text without an id and the owner\'s own id',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    render(<ContactsPanel api={api} identity={alice.identity} onOpen={()=>{}} onBack={()=>{}}/>);
    const paste=screen.getByRole('textbox',{name:'Invitation or ID'});
    await user.click(paste);await user.paste('hi, add me');
    expect(screen.getByText('There is no ID in this text. An ID starts with ain1.')).toBeVisible();
    expect(screen.getByRole('button',{name:'Add friend'})).toBeDisabled();
    await user.clear(paste);await user.click(paste);await user.paste(locales.en.invite.message('Alice',ownId));
    expect(screen.getByText('This is your own ID.')).toBeVisible();
    expect(screen.getByRole('button',{name:'Add friend'})).toBeDisabled();
    expect(api.requestContact).not.toHaveBeenCalled();
  });
});

describe('the start screen keeps the next steps in reach',()=>{
  it('offers the agent instructions and the invitation, and free messages while none are left',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.snapshot.mockResolvedValue(structuredClone(alice));
    api.claimCoins.mockResolvedValue({status:'open',claimId:'c1',loginUrl:'https://id.example/v1/claims/c1/login',expiresAt:2});
    render(<ChatShell api={api}/>);
    const start=await screen.findByRole('region',{name:'Your agent needs friends to talk to.'});
    await user.click(within(start).getByRole('button',{name:'Copy instructions'}));
    await waitFor(async()=>expect(await navigator.clipboard.readText()).toContain(`'${appCli}'`));
    await user.click(within(start).getByRole('button',{name:'Copy invitation'}));
    expect(parseInvitation(await navigator.clipboard.readText())).toEqual({networkId:ownId,name:'Alice'});
    const coins=await within(start).findByRole('region',{name:'Get free messages'});
    await user.click(within(coins).getByRole('button',{name:'Log in with GitHub'}));
    expect(api.claimCoins).toHaveBeenCalledWith({provider:'github'});
  });
  it('does not ask for a login while messages are left',async()=>{
    const api=fakeApi();
    api.snapshot.mockResolvedValue(structuredClone(alice));
    api.coinsBalance.mockResolvedValue(granted());
    render(<ChatShell api={api}/>);
    await screen.findByRole('region',{name:'Your agent needs friends to talk to.'});
    await waitFor(()=>expect(api.coinsBalance).toHaveBeenCalled());
    expect(screen.queryByRole('region',{name:'Get free messages'})).not.toBeInTheDocument();
  });
});
