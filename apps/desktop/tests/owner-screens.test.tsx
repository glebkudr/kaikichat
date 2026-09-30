import {render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {describe,expect,it,vi} from 'vitest';
import {ChatShell} from '../src/ChatShell';
import {ContactsPanel} from '../src/ContactsPanel';
import {GroupPanel,NewGroupPanel} from '../src/GroupPanel';
import {SettingsPanel} from '../src/SettingsPanel';
import {WalletPanel,ether} from '../src/WalletPanel';
import {groupRights} from '../src/group-roles';
import {coreError} from '../src/core-error';
import {bobId,carolId,emptyBalance,fakeApi,group,ownId,policy} from './fake-api';

const refusal=(code:string,retryable=false)=>coreError({code,message:code,retryable});
const daveId=`ain1${'d4'.repeat(32)}`;
const identity={name:'Alice',networkId:ownId};

describe('contacts by network id',()=>{
  it('asks a valid id for a conversation, keeping the operation across a retry, and opens it',async()=>{
    const api=fakeApi();const onOpen=vi.fn();const user=userEvent.setup();
    api.requestContact.mockRejectedValueOnce(refusal('network_unavailable',true));
    render(<ContactsPanel api={api} identity={identity} onOpen={onOpen} onBack={()=>{}}/>);
    const send=screen.getByRole('button',{name:'Add friend'});
    await user.type(screen.getByRole('textbox',{name:'Invitation or ID'}),'ain1xyz');
    await user.type(screen.getByRole('textbox',{name:'What to call them'}),'Bob');
    expect(send).toBeDisabled();
    await user.clear(screen.getByRole('textbox',{name:'Invitation or ID'}));await user.type(screen.getByRole('textbox',{name:'Invitation or ID'}),bobId);
    await user.click(send);
    expect(await screen.findByRole('alert')).toHaveTextContent('The network is not reachable yet');
    await user.click(send);
    await waitFor(()=>expect(onOpen).toHaveBeenCalledWith('new-contact'));
    const [first,second]=api.requestContact.mock.calls.map(([request])=>request);
    expect(first).toEqual({networkId:bobId,name:'Bob',operationId:expect.any(String)});
    expect(second.operationId).toBe(first.operationId);
  });
  it('accepts and rejects waiting requests and shows hostile names as text',async()=>{
    const api=fakeApi();const onOpen=vi.fn();const user=userEvent.setup();
    const hostile='<img src=x onerror="window.pwned=1">';
    api.introRequests.mockResolvedValue([{requestId:'r1',networkId:bobId,name:'Bob',receivedAt:1788563000},{requestId:'r2',networkId:carolId,name:hostile,receivedAt:1788563050,group:'g2'}]);
    render(<ContactsPanel api={api} identity={identity} onOpen={onOpen} onBack={()=>{}}/>);
    expect(await screen.findByText(hostile)).toBeVisible();
    expect(document.querySelector('img')).toBeNull();
    expect(screen.getByText('Group invitation')).toBeVisible();
    await user.click(screen.getByRole('button',{name:`Reject request ${hostile}`}));
    expect(api.rejectIntroRequest).toHaveBeenCalledWith({requestId:'r2'});
    await user.click(screen.getByRole('button',{name:'Accept request Bob'}));
    expect(api.acceptIntroRequest).toHaveBeenCalledWith({requestId:'r1'});
    await waitFor(()=>expect(onOpen).toHaveBeenCalledWith('accepted'));
    expect((window as {pwned?:number}).pwned).toBeUndefined();
  });
});

describe('settings: who can write by id',()=>{
  it('saves who may ask: a list of valid ids, refusing a malformed one',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const form=await screen.findByRole('form',{name:'Who can write by ID'});
    const save=within(form).getByRole('button',{name:'Save rules'});
    expect(save).toBeDisabled();
    await user.click(within(form).getByRole('radio',{name:/Only the list/}));
    await user.type(within(form).getByRole('textbox',{name:'ID list'}),`${bobId}\nnot-an-id`);
    expect(save).toBeDisabled();
    await user.clear(within(form).getByRole('textbox',{name:'ID list'}));
    await user.type(within(form).getByRole('textbox',{name:'ID list'}),`${bobId}\n${carolId}`);
    await user.click(save);
    expect(api.setIntroPolicy).toHaveBeenCalledWith({...policy(),mode:'list',allowed:[bobId,carolId]});
    expect(await screen.findByText('Rules saved.')).toBeVisible();
  });
});

describe('groups and roles',()=>{
  it('follows the roster rules: admins remove neither the owner nor admins, only the owner names admins',()=>{
    const owner=groupRights(group());
    expect([owner.canAdd,owner.canSetAdmins,owner.canRemove(bobId),owner.canRemove(ownId)]).toEqual([true,true,true,false]);
    const admin=groupRights(group({owner:carolId,admins:[ownId,bobId],role:'admin'}));
    expect([admin.canAdd,admin.canSetAdmins,admin.canRemove(carolId),admin.canRemove(bobId)]).toEqual([true,false,false,false]);
    const member=groupRights(group({owner:carolId,role:'member'}));
    expect([member.canAdd,member.canRemove(bobId)]).toEqual([false,false]);
  });
  it('lets the owner remove a member and name admins with one commit each, retrying with the same operation',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.changeGroup.mockRejectedValueOnce(refusal('group_busy',true));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    await screen.findByRole('heading',{name:'Team'});
    expect(screen.getByText('You')).toBeVisible();
    await user.click(screen.getByRole('checkbox',{name:/Admin ain1b2/}));
    await user.click(screen.getByRole('button',{name:'Save admins'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('The previous group change');
    await user.click(screen.getByRole('button',{name:'Save admins'}));
    const [first,second]=api.changeGroup.mock.calls.map(([request])=>request);
    expect(first).toEqual({groupId:'g1',admins:[bobId],operationId:expect.any(String)});
    expect(second.operationId).toBe(first.operationId);
    await user.click(screen.getByRole('button',{name:/Remove ain1c3/}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({groupId:'g1',remove:[carolId],operationId:expect.any(String)});
    expect(api.changeGroup.mock.calls[2][0].operationId).not.toBe(first.operationId);
  });
  it('shows a plain member the roster and the bans without controls',async()=>{
    const api=fakeApi();api.group.mockResolvedValue(group({owner:carolId,role:'member',members:[carolId,ownId,bobId],banned:[{id:daveId,byOwner:false}]}));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    expect(await screen.findByText('The owner and admins change the membership.')).toBeVisible();
    const banned=screen.getByRole('list',{name:'Banned'});
    expect(within(banned).getByTitle(daveId)).toBeVisible();
    expect(within(banned).queryByRole('button')).not.toBeInTheDocument();
    expect(screen.queryByRole('button',{name:/remove|ban/i})).not.toBeInTheDocument();
    expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();
  });
  it('lets the owner ban a member or any id and lift a ban, one commit each',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    const stranger=`ain1${'e5'.repeat(32)}`;
    api.group.mockResolvedValue(group({banned:[{id:daveId,byOwner:false}]}));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    await user.click(await screen.findByRole('button',{name:/^Ban ain1c3/}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({groupId:'g1',ban:[carolId],operationId:expect.any(String)});
    expect(screen.queryByRole('button',{name:/^Ban ain1a1/})).not.toBeInTheDocument();
    const banned=screen.getByRole('list',{name:'Banned'});
    await user.click(within(banned).getByRole('button',{name:/Unban ain1d4/}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({groupId:'g1',unban:[daveId],operationId:expect.any(String)});
    const field=screen.getByRole('textbox',{name:'Member ID'});
    await user.type(field,stranger);
    await user.click(screen.getByRole('button',{name:'Ban ID'}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({groupId:'g1',ban:[stranger],operationId:expect.any(String)});
    // A banned id is not added: the node's refusal says why.
    api.changeGroup.mockRejectedValueOnce(refusal('banned'));
    await user.clear(field);await user.type(field,daveId);
    await user.click(screen.getByRole('button',{name:'Add'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('This ID is banned in the group');
    api.changeGroup.mockRejectedValueOnce(refusal('member_outdated'));
    await user.clear(field);await user.type(field,`ain1${'f6'.repeat(32)}`);
    await user.click(screen.getByRole('button',{name:'Add'}));
    await waitFor(()=>expect(screen.getByRole('alert')).toHaveTextContent('update Kaiki Chat'));
  });
  it('shows an admin the bans it may place and lift: not on the owner or admins, not the owner’s',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    const byOwner=`ain1${'e5'.repeat(32)}`;const byAdmin=`ain1${'f6'.repeat(32)}`;
    api.group.mockResolvedValue(group({owner:carolId,admins:[ownId,bobId],role:'admin',members:[carolId,ownId,bobId,daveId],banned:[{id:byOwner,byOwner:true},{id:byAdmin,byOwner:false}]}));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const banned=await screen.findByRole('list',{name:'Banned'});
    expect(within(banned).getByTitle(byOwner)).toBeVisible();
    expect(within(banned).queryByRole('button',{name:/Unban ain1e5/})).not.toBeInTheDocument();
    await user.click(within(banned).getByRole('button',{name:/Unban ain1f6/}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({groupId:'g1',unban:[byAdmin],operationId:expect.any(String)});
    expect(screen.queryByRole('button',{name:/^Ban ain1c3/})).not.toBeInTheDocument();
    expect(screen.queryByRole('button',{name:/^Ban ain1b2/})).not.toBeInTheDocument();
    await user.click(screen.getByRole('button',{name:/^Ban ain1d4/}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({groupId:'g1',ban:[daveId],operationId:expect.any(String)});
  });
  it('adds a member by id and makes a group of ids',async()=>{
    const api=fakeApi();const user=userEvent.setup();const onCreated=vi.fn();
    const newcomer=`ain1${'d4'.repeat(32)}`;
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    await user.type(await screen.findByRole('textbox',{name:'Member ID'}),newcomer);
    await user.click(screen.getByRole('button',{name:'Add'}));
    expect(api.changeGroup).toHaveBeenCalledWith({groupId:'g1',add:[newcomer],operationId:expect.any(String)});
    render(<NewGroupPanel api={api} onCreated={onCreated} onBack={()=>{}}/>);
    await user.type(screen.getByRole('textbox',{name:'Group name'}),'Release');
    await user.type(screen.getByRole('textbox',{name:'Member IDs (one per line)'}),`${bobId}\n${carolId}`);
    await user.click(screen.getByRole('button',{name:'Make group'}));
    expect(api.createGroup).toHaveBeenCalledWith({name:'Release',members:[bobId,carolId],operationId:expect.any(String)});
    await waitFor(()=>expect(onCreated).toHaveBeenCalledWith('made'));
  });
});

describe('wallet',()=>{
  it('tops up with crypto once the node knows the shop: USD price, ETH at the quote or USDC in two steps',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    const call=(name:string)=>({to:'0x5f',calldata:'0x',uri:`ethereum:${name}`});
    const payment={book:'0xb00c',key:'0x1',salt:'0x2',shop:'0x5f',chainId:84532,count:1000,validSeconds:2592000,priceUsdc:'5000000',eth:{...call('eth'),quote:'1485000000000000',value:'1500000000000000'},usdc:{token:'0x6e',amount:'5000000',approve:call('approve'),buy:call('buy')},createdAt:1};
    api.coinsBuy.mockRejectedValueOnce(refusal('chain_pending',true)).mockResolvedValueOnce(payment);
    render(<WalletPanel api={api} onBack={()=>{}}/>);
    await user.click(await screen.findByRole('button',{name:'Top up with crypto'}));
    expect(await screen.findByText('$5.00',{},{timeout:3000})).toBeVisible();
    expect(screen.getByRole('textbox',{name:'Payment link'})).toHaveValue('ethereum:eth');
    expect(screen.getByText(/0\.0015 ETH at the current rate/)).toBeVisible();
    await user.click(screen.getByRole('button',{name:'Pay with ETH'}));
    await user.click(screen.getByRole('button',{name:'1. Allow USDC'}));
    await user.click(screen.getByRole('button',{name:'2. Buy with USDC'}));
    expect(api.openPayment.mock.calls.map(([request])=>request)).toEqual([{book:'0xb00c',step:'eth'},{book:'0xb00c',step:'approve'},{book:'0xb00c',step:'buy'}]);
    expect(api.coinsBuy).toHaveBeenCalledTimes(2);
  });
  it('offers USDC while the ETH rate is stale',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    const call={to:'0x5f',calldata:'0x',uri:'ethereum:x'};
    api.coinsBuy.mockResolvedValue({book:'0xb',key:'0x1',salt:'0x2',shop:'0x5f',chainId:1,count:1000,validSeconds:1,priceUsdc:'5000000',eth:null,usdc:{token:'0x6e',amount:'5000000',approve:call,buy:call},createdAt:1});
    render(<WalletPanel api={api} onBack={()=>{}}/>);
    await user.click(await screen.findByRole('button',{name:'Top up with crypto'}));
    expect(await screen.findByText(/The ETH rate is being read/)).toBeVisible();
    expect(screen.queryByRole('button',{name:'Pay with ETH'})).not.toBeInTheDocument();
    expect(screen.getByRole('button',{name:'1. Allow USDC'})).toBeEnabled();
  });
  it('gets coins through Google and reports the outcome from the balance',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.claimCoins.mockResolvedValue({status:'open',claimId:'c1',loginUrl:'https://id.example/v1/claims/c1/login',expiresAt:2});
    api.coinsBalance.mockResolvedValueOnce(emptyBalance()).mockResolvedValue({...emptyBalance(),remaining:100,books:[{book:'0x1',kind:'granted',count:100,used:0,validUntil:1791155100}],lastClaim:{status:'granted',book:'0x1'}});
    render(<WalletPanel api={api} onBack={()=>{}}/>);
    await user.click(await screen.findByRole('button',{name:'Log in with GitHub'}));
    expect(api.claimCoins).toHaveBeenCalledWith({provider:'github'});
    expect(await screen.findByText('Your monthly coins have arrived.')).toBeVisible();
    expect(screen.getByTestId('coins-left')).toHaveTextContent('100');
    expect(screen.getByText('Monthly coins for your login')).toBeVisible();
  });
  it('tells an owner who logs in again on this device that the key here already signs them in',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    const books=[{book:'0x1',kind:'granted' as const,count:100,used:0,validUntil:1791155100}];
    api.claimCoins.mockResolvedValue({status:'open',claimId:'c2',loginUrl:'https://id.example/v1/claims/c2/login',expiresAt:2});
    api.coinsBalance.mockResolvedValueOnce({...emptyBalance(),remaining:100,books,lastClaim:{status:'granted',book:'0x1'}}).mockResolvedValue({...emptyBalance(),remaining:100,books,lastClaim:{status:'denied',reason:'already_claimed'}});
    render(<WalletPanel api={api} onBack={()=>{}}/>);
    await user.click(await screen.findByRole('button',{name:'Log in with Google'}));
    const headline=await screen.findByText('You are already logged in on this device; there is no need to log in again.');
    expect(headline).toBeVisible();
    const outcome=headline.closest('[role=status]');
    expect(outcome).toHaveTextContent('Kaiki Chat knows you by the private key kept on this device');
    expect(outcome).not.toHaveTextContent('The server refused');
    expect(outcome).not.toHaveTextContent('another device');
    expect(screen.queryByRole('textbox',{name:'Login link'})).not.toBeInTheDocument();
  });
  it('formats wei without rounding a small price away',()=>{
    expect(ether('1000')).toBe('0.000000000000001 ETH');expect(ether('1500000000000000',',')).toBe('0,0015 ETH');expect(ether('2000000000000000000')).toBe('2 ETH');
  });
});

describe('profile gates',()=>{
  it('asks for the password of a sealed profile and opens it',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.snapshot.mockRejectedValueOnce(refusal('profile_locked'));
    api.profileStatus.mockResolvedValue({state:'locked',secrets:'file',newProfile:false});
    api.unlockProfile.mockRejectedValueOnce(refusal('secrets_locked'));
    render(<ChatShell api={api}/>);
    const password=await screen.findByLabelText('Password');
    await user.type(password,'wrong password');await user.click(screen.getByRole('button',{name:'Open profile'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('The password does not open this profile');
    await user.clear(password);await user.type(password,'correct horse');await user.click(screen.getByRole('button',{name:'Open profile'}));
    expect(api.unlockProfile).toHaveBeenLastCalledWith({password:'correct horse'});
    expect(await screen.findByRole('heading',{name:'Your agent needs friends to talk to.'})).toBeVisible();
  });
  it('seals a new profile only with a repeated password of eight characters',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.snapshot.mockRejectedValueOnce(refusal('profile_locked'));
    api.profileStatus.mockResolvedValue({state:'locked',secrets:'file',newProfile:true});
    render(<ChatShell api={api}/>);
    const save=await screen.findByRole('button',{name:'Save password'});
    await user.type(screen.getByLabelText('Password'),'short');expect(save).toBeDisabled();
    await user.type(screen.getByLabelText('Password'),' but long');await user.type(screen.getByLabelText('Repeat the password'),'short but lonk');
    expect(save).toBeDisabled();
    await user.clear(screen.getByLabelText('Repeat the password'));await user.type(screen.getByLabelText('Repeat the password'),'short but long');
    await user.click(save);
    expect(api.unlockProfile).toHaveBeenCalledWith({password:'short but long'});
  });
  it('leaves a stopped daemon stopped until the owner starts it',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.snapshot.mockRejectedValueOnce(refusal('daemon_unavailable',true));
    render(<ChatShell api={api}/>);
    await user.click(await screen.findByRole('button',{name:'Start the node'}));
    expect(api.reconnect).toHaveBeenCalledOnce();
    expect(await screen.findByRole('heading',{name:'Your agent needs friends to talk to.'})).toBeVisible();
  });
});

describe('untrusted text renders as text on every screen',()=>{
  const hostile=['<img src=x onerror="window.pwned=1">','<script>window.pwned=2</script>','<a href="javascript:window.pwned=3">link</a>','‮gnp.exe'];
  it('keeps names, group names, messages and authors inert',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.snapshot.mockResolvedValue({identity:{name:hostile[0],networkId:ownId},network:{connectedPeers:1,state:'online'},conversations:[
      {id:'g1',title:hostile[1],unread:1,messages:[{id:'m1',author:hostile[2],own:false,text:hostile[3],createdAt:1,delivery:{phase:'delivered',replicas:10,target:10}}]},
    ]});
    api.groups.mockResolvedValue([group({name:hostile[1]})]);
    api.conversationHistory.mockImplementation(async({conversationId})=>({conversationId,messages:[{id:'m1',author:hostile[2],own:false,text:hostile.join(' '),createdAt:1,delivery:{phase:'delivered',replicas:10,target:10}}],nextBefore:null}));
    render(<ChatShell api={api}/>);
    expect(await screen.findByText(hostile.join(' '))).toBeVisible();
    expect(screen.getAllByText(hostile[1]).length).toBeGreaterThan(0);
    await user.click(screen.getByRole('button',{name:'Members'}));
    await screen.findByRole('heading',{name:'Team'});
    expect(document.querySelector('img,script:not([src]),a[href]')).toBeNull();
    expect((window as {pwned?:number}).pwned).toBeUndefined();
  });
  it('builds the UI without raw HTML sinks',async()=>{
    const sources=import.meta.glob('../src/**/*.{ts,tsx}',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
    expect(Object.keys(sources).length).toBeGreaterThan(10);
    for(const [path,text] of Object.entries(sources))expect(text,path).not.toMatch(/dangerouslySetInnerHTML|innerHTML|outerHTML|insertAdjacentHTML|document\.write|\beval\(|new Function/);
  });
});
