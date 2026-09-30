import {render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {describe,expect,it,vi} from 'vitest';
import {ChatShell} from '../src/ChatShell';
import {DiscoverPanel,cardWords} from '../src/DiscoverPanel';
import {GroupPanel} from '../src/GroupPanel';
import {coreError} from '../src/core-error';
import {LocaleProvider} from '../src/i18n';
import type {Card} from '../src/types';
import {bobId,carolId,fakeApi,group,ownId} from './fake-api';

const refusal=(code:string,retryable=false)=>coreError({code,message:code,retryable});
const hostile='<img src=x onerror="window.pwned=1">';
const card=(overrides:Partial<Card>):Card=>({id:'c1'.repeat(32),kind:'group',name:'Rustaceans',about:'Rust and agents',tags:['rust'],langs:['en'],owner:carolId,groupRef:'cd'.repeat(32),expiresAt:1791155100,...overrides});
const panel=(api:ReturnType<typeof fakeApi>,onOpen=vi.fn())=>render(<LocaleProvider initial="en"><DiscoverPanel api={api} onOpen={onOpen} onBack={()=>{}} poll={5}/></LocaleProvider>);

describe('finding people the owner knows',()=>{
  it('counts the pasted addresses and their price before paying, then asks the found ones for a conversation',async()=>{
    const api=fakeApi();const onOpen=vi.fn();const user=userEvent.setup();
    api.discoverHandles.mockResolvedValue({handles:[{kind:'google',handle:'ann@example.org'},{kind:'github',handle:'octo-cat'}]});
    api.discoverLookup.mockResolvedValue({found:[{kind:'google',handle:'ann@example.org',networkId:bobId},{kind:'github',handle:'octo-cat',networkId:null}]});
    panel(api,onOpen);
    const known=screen.getByRole('region',{name:'People you know'});
    await user.type(within(known).getByRole('textbox',{name:'Addresses and GitHub logins'}),'Ann <ann@example.org>, github:octo-cat');
    await user.click(within(known).getByRole('button',{name:'Count the addresses'}));
    expect(await within(known).findByText('Addresses found: 2. Checking them costs 2 coins.')).toBeVisible();
    expect(api.discoverLookup).not.toHaveBeenCalled();
    await user.click(within(known).getByRole('button',{name:'Check for 2 coins'}));
    const results=await within(known).findByRole('list',{name:'Results'});
    expect(within(results).getByText('Not in Kaiki Chat')).toBeVisible();
    expect(api.discoverLookup).toHaveBeenCalledWith({handles:[{kind:'google',handle:'ann@example.org'},{kind:'github',handle:'octo-cat'}]});
    await user.click(within(results).getByRole('button',{name:'Add contact'}));
    await waitFor(()=>expect(onOpen).toHaveBeenCalledWith('new-contact'));
    expect(api.requestContact).toHaveBeenCalledWith({networkId:bobId,name:'ann',operationId:expect.any(String)});
  });
  it('says what is missing when the node has no book to pay with',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.discoverHandles.mockResolvedValue({handles:[{kind:'google',handle:'ann@example.org'}]});
    api.discoverLookup.mockRejectedValue(refusal('book_required'));
    panel(api);
    const known=screen.getByRole('region',{name:'People you know'});
    await user.type(within(known).getByRole('textbox',{name:'Addresses and GitHub logins'}),'ann@example.org');
    await user.click(within(known).getByRole('button',{name:'Count the addresses'}));
    await user.click(await within(known).findByRole('button',{name:'Check for 1 coins'}));
    expect(await within(known).findByRole('alert')).toHaveTextContent('This needs an active book of coins');
  });
});

describe('groups and people by interest',()=>{
  it('finds cards, shows what strangers wrote as text, reads a group without joining and asks a person',async()=>{
    const api=fakeApi();const onOpen=vi.fn();const user=userEvent.setup();
    api.discoverSearch.mockResolvedValue({cards:[card({name:hostile}),card({id:'c2'.repeat(32),kind:'profile',name:'Anna',owner:bobId,groupRef:null})]});
    panel(api,onOpen);
    const explore=screen.getByRole('region',{name:'Groups and people by interest'});
    await user.type(within(explore).getByRole('textbox',{name:'Words'}),'rust');
    await user.selectOptions(within(explore).getByRole('combobox',{name:'Kind'}),'group');
    await user.click(within(explore).getByRole('button',{name:'Search'}));
    expect(await within(explore).findByText(hostile)).toBeVisible();
    expect((window as {pwned?:number}).pwned).toBeUndefined();
    expect(api.discoverSearch).toHaveBeenCalledWith({query:'rust',kind:'group'});
    await user.click(within(explore).getByRole('button',{name:'Read the group'}));
    await waitFor(()=>expect(onOpen).toHaveBeenCalledWith('cd'.repeat(32)));
    expect(api.followGroup).toHaveBeenCalledWith({group:'cd'.repeat(32),owner:carolId,name:hostile});
    await user.click(within(explore).getByRole('button',{name:'Add contact'}));
    await waitFor(()=>expect(api.requestContact).toHaveBeenCalledWith({networkId:bobId,name:'Anna',operationId:expect.any(String)}));
  });
  it('finds channels as channels and reads one without joining',async()=>{
    const api=fakeApi();const onOpen=vi.fn();const user=userEvent.setup();
    api.discoverSearch.mockResolvedValue({cards:[card({id:'c3'.repeat(32),kind:'channel',name:'Rust Weekly',groupRef:'ce'.repeat(32)})]});
    panel(api,onOpen);
    const explore=screen.getByRole('region',{name:'Groups and people by interest'});
    await user.selectOptions(within(explore).getByRole('combobox',{name:'Kind'}),'channel');
    await user.click(within(explore).getByRole('button',{name:'Search'}));
    expect(api.discoverSearch).toHaveBeenCalledWith({query:'',kind:'channel'});
    const found=await within(explore).findByRole('list',{name:'Results'});
    expect(within(found).getByText('Channel')).toBeVisible();
    expect(within(found).queryByRole('button',{name:'Add contact'})).not.toBeInTheDocument();
    await user.click(within(found).getByRole('button',{name:'Read the channel'}));
    await waitFor(()=>expect(onOpen).toHaveBeenCalledWith('ce'.repeat(32)));
    expect(api.followGroup).toHaveBeenCalledWith({group:'ce'.repeat(32),owner:carolId,name:'Rust Weekly'});
  });
});

describe('being found',()=>{
  it('opens the login with the code to compare and reports the link once the service decides',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.discoverLink.mockResolvedValue({linkId:'l1',loginUrl:'https://directory.example/v1/links/l1/login',code:'4F7K-9QX2',expiresAt:Date.now()/1000+900});
    api.discoverStatus.mockResolvedValueOnce({status:'pending',reason:null}).mockResolvedValue({status:'linked',reason:null});
    panel(api);
    const findable=screen.getByRole('region',{name:'Let people find you'});
    await user.click(within(findable).getByRole('button',{name:'Log in with Google'}));
    expect(await within(findable).findByText(/shows the code 4F7K-9QX2/)).toBeVisible();
    expect(await within(findable).findByText('Linked: people who know this account will find you.')).toBeVisible();
    expect(api.discoverLink).toHaveBeenCalledWith({kind:'google'});
    expect(api.discoverStatus).toHaveBeenCalledWith({linkId:'l1'});
  });
  it('publishes a profile card for ten coins with its tags and languages, and withdraws it',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    panel(api);
    const form=screen.getByRole('region',{name:'Your card'});
    await user.type(within(form).getByRole('textbox',{name:'About'}),'I write agents in Rust');
    await user.type(within(form).getByRole('textbox',{name:'Tags, separated by spaces'}),'Rust agents rust');
    await user.type(within(form).getByRole('textbox',{name:'Languages, for example en de'}),'en de');
    await user.click(within(form).getByRole('button',{name:'Publish for 10 coins'}));
    expect(await within(form).findByText(/^Published until/)).toBeVisible();
    expect(api.discoverPublish).toHaveBeenCalledWith({kind:'profile',about:'I write agents in Rust',tags:['rust','agents'],langs:['en','de']});
    await user.click(within(form).getByRole('button',{name:'Withdraw the card'}));
    expect(await within(form).findByText('The card was withdrawn.')).toBeVisible();
    expect(api.discoverWithdraw).toHaveBeenCalledWith({cardId:'c1'.repeat(32)});
  });
  it('takes each tag once, lowercase, and no more than the card holds',()=>{
    expect(cardWords('Rust, agents RUST  go',2)).toEqual(['rust','agents']);
  });
});

describe('who reads a group',()=>{
  it('opens a group to everyone only after the owner confirms it stays public, then offers its card',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const reading=await screen.findByRole('region',{name:'Who reads'});
    expect(within(reading).getByText('Only members read this group.')).toBeVisible();
    const open=within(reading).getByRole('button',{name:'Open to everyone'});
    expect(open).toBeDisabled();
    await user.click(within(reading).getByRole('checkbox',{name:/stays public for good/}));
    api.group.mockResolvedValue(group({access:'public'}));
    await user.click(open);
    await waitFor(()=>expect(api.changeGroup).toHaveBeenCalledWith({access:'public',groupId:'g1',operationId:expect.any(String)}));
    expect(await screen.findByRole('region',{name:'Group card'})).toBeVisible();
    expect(within(screen.getByRole('region',{name:'Who reads'})).getByRole('button',{name:'Close to members only'})).toBeVisible();
  });
  it('tells a member that an open group is public, without the owner’s controls',async()=>{
    const api=fakeApi();
    api.group.mockResolvedValue(group({access:'public',role:'member',owner:bobId}));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const reading=await screen.findByRole('region',{name:'Who reads'});
    expect(within(reading).getByText(/Anyone can read this group/)).toBeVisible();
    expect(within(reading).queryByRole('button')).not.toBeInTheDocument();
    expect(screen.queryByRole('region',{name:'Group card'})).not.toBeInTheDocument();
  });
});

describe('a group read without joining',()=>{
  it('lists it with the chats, shows its posts without a composer and stops reading it',async()=>{
    const api=fakeApi();const user=userEvent.setup();const followId='cd'.repeat(32);
    api.snapshot.mockResolvedValue({identity:{name:'Alice',networkId:ownId},network:{connectedPeers:3,state:'online'},conversations:[{id:followId,title:'Rustaceans',unread:0,messages:[]}]});
    api.follows.mockResolvedValue([{id:followId,name:'Rustaceans',owner:carolId,since:1788563000,closed:false}]);
    api.conversationHistory.mockResolvedValue({conversationId:followId,messages:[{id:'p1',author:carolId,own:false,text:'Async review on Friday',createdAt:1788563500,delivery:{phase:'delivered',replicas:10,target:10}}],nextBefore:null});
    render(<ChatShell api={api} locale="en"/>);
    expect(await screen.findByText('Async review on Friday')).toBeVisible();
    expect(await screen.findByText('Open group · you read it without joining')).toBeVisible();
    expect(screen.getByText('Only members write here.')).toBeVisible();
    expect(screen.queryByRole('textbox',{name:'Message'})).not.toBeInTheDocument();
    await user.click(screen.getByRole('button',{name:'Stop reading'}));
    await waitFor(()=>expect(api.unfollowGroup).toHaveBeenCalledWith({groupId:followId}));
  });
  it('says a followed channel is written by its team',async()=>{
    const api=fakeApi();const followId='ce'.repeat(32);
    api.snapshot.mockResolvedValue({identity:{name:'Alice',networkId:ownId},network:{connectedPeers:3,state:'online'},conversations:[{id:followId,title:'Rust Weekly',unread:0,messages:[]}]});
    api.follows.mockResolvedValue([{id:followId,name:'Rust Weekly',owner:carolId,since:1788563000,closed:false,kind:'channel',retention:0,sealed:false}]);
    api.conversationHistory.mockResolvedValue({conversationId:followId,messages:[],nextBefore:null});
    render(<ChatShell api={api} locale="en"/>);
    expect(await screen.findByText('Channel · you read it without writing')).toBeVisible();
    expect(screen.getByText('Only the channel’s team writes here.')).toBeVisible();
    expect(screen.queryByRole('textbox',{name:'Message'})).not.toBeInTheDocument();
  });
});
