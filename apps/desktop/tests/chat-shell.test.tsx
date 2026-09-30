import {networkFixture} from './network-fixture';
import {fakeApi} from './fake-api';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ChatShell } from '../src/ChatShell';
import { DeliveryBadge } from '../src/DeliveryBadge';
import type { ConversationHistory, DesktopApi, Snapshot } from '../src/types';

function initial(): Snapshot {
  return {
    identity: { name: 'Alice', networkId: 'ain1-alice' },
    network: { connectedPeers: 3, state: 'online' },
    conversations: [{ id: 'bob', title: 'Bob', unread: 0, messages: [
      { id: 'm1', author: 'bob', own: false, text: 'Shall we test delivery?', createdAt: 100,
        delivery: { phase: 'delivered', replicas: 10, target: 10 } },
    ] }],
  };
}
/** A screen from the gear menu in the sidebar. */
async function openScreen(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(screen.getByRole('button', { name: 'Menu' }));
  await user.click(screen.getByRole('menuitem', { name }));
}
function api(snapshot = initial()) {
  const client = {
    ...fakeApi(),
    networkSettings:vi.fn(async()=>networkFixture()),configureNetwork:vi.fn(),
    snapshot: vi.fn(async () => structuredClone(snapshot)),
    conversationHistory: vi.fn(async (request:{conversationId:string;before:string|null}):Promise<ConversationHistory> => ({conversationId:request.conversationId,messages:structuredClone((snapshot.conversations.length?snapshot:initial()).conversations.find(c=>c.id===request.conversationId)?.messages??[]),nextBefore:null as string|null})),
    sendMessage: vi.fn(async (request: { conversationId: string; text: string; operationId: string }) => ({
      id: 'new-message', author: 'ain1-alice', own: true, text: request.text, createdAt: 101,
      delivery: { phase: 'queued' as const, replicas: 0, target: 10 },
    })),
    createIdentity: vi.fn(async () => ({ name: 'Alice', networkId: 'ain1-created' })),
    addContact: vi.fn(async () => undefined),
    createInvitation: vi.fn(async () => 'ain-invite1:complete-signed-public-invitation'),
    listRuntimes: vi.fn(async () => []), provisionRuntime: vi.fn(), revokeRuntime: vi.fn(),
    subscribe: vi.fn((_listener: () => void) => () => undefined),
  } satisfies DesktopApi;
  return client;
}

describe('conversation UI with a replaceable real-core adapter', () => {
  it('loads conversations and sends the composed message to the selected recipient', async () => {
    const user = userEvent.setup(); const client = api();
    render(<ChatShell api={client} />);
    await messageInHistory('Shall we test delivery?');
    await user.type(screen.getByRole('textbox', { name: 'Message' }), 'Hi, Bob');
    await user.click(screen.getByRole('button', { name: 'Send message' }));
    expect(client.sendMessage).toHaveBeenCalledTimes(1);
    expect(client.sendMessage).toHaveBeenCalledWith({ conversationId: 'bob', text: 'Hi, Bob', operationId: expect.any(String) });
    expect(await messageInHistory('Hi, Bob')).toBeVisible();
    expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue('');
    expect(screen.getByText('Queued')).toBeVisible();
  });

  it('sends on Enter and keeps Shift+Enter and an unfinished IME word in the draft', async () => {
    const user = userEvent.setup(); const client = api();
    render(<ChatShell api={client} />);
    await messageInHistory('Shall we test delivery?');
    const box = screen.getByRole('textbox', { name: 'Message' });
    expect(screen.getByText('Enter to send')).toBeVisible();
    expect(screen.queryByText(/costs one coin/)).not.toBeInTheDocument();
    await user.type(box, 'First line{Shift>}{Enter}{/Shift}second');
    expect(box).toHaveValue('First line\nsecond');
    fireEvent.keyDown(box, { key: 'Enter', isComposing: true });
    expect(client.sendMessage).not.toHaveBeenCalled();
    await user.keyboard('{Enter}');
    expect(client.sendMessage).toHaveBeenCalledTimes(1);
    expect(client.sendMessage).toHaveBeenCalledWith({ conversationId: 'bob', text: 'First line\nsecond', operationId: expect.any(String) });
    expect(await messageInHistory('First line second')).toBeVisible();
    expect(box).toHaveValue('');
  });

  it('keeps unsent content and the operation ID when a response is lost and the user retries', async () => {
    const user = userEvent.setup(); const client = api();
    client.sendMessage.mockRejectedValueOnce(new Error('Connection to the core was lost'));
    render(<ChatShell api={client} />);
    await messageInHistory('Shall we test delivery?');
    await user.type(screen.getByRole('textbox', { name: 'Message' }), 'Do not lose this');
    await user.click(screen.getByRole('button', { name: 'Send message' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Connection to the core was lost');
    expect(screen.getByRole('textbox', { name: 'Message' })).toHaveValue('Do not lose this');
    await user.click(screen.getByRole('button', { name: 'Send message' }));
    await messageInHistory('Do not lose this');
    expect(client.sendMessage.mock.calls[0][0].operationId).toEqual(client.sendMessage.mock.calls[1][0].operationId);
  });

  it('allocates a new operation when the user edits a failed send', async () => {
    const user = userEvent.setup(); const client = api(); client.sendMessage.mockRejectedValueOnce(new Error('offline'));
    render(<ChatShell api={client} />); await messageInHistory('Shall we test delivery?');
    const input = screen.getByRole('textbox', { name: 'Message' });
    await user.type(input, 'First version'); await user.click(screen.getByRole('button', { name: 'Send message' }));
    await screen.findByRole('alert'); await user.clear(input); await user.type(input, 'Second version');
    await user.click(screen.getByRole('button', { name: 'Send message' }));
    await messageInHistory('Second version');
    expect(client.sendMessage.mock.calls[0][0].operationId).not.toEqual(client.sendMessage.mock.calls[1][0].operationId);
  });

  it('renders received content as text without executing supplied markup', async () => {
    const snapshot = initial(); snapshot.conversations[0].messages[0].text = '<img src=x onerror="window.stolen=true">';
    render(<ChatShell api={api(snapshot)} />);
    expect(await messageInHistory('<img src=x onerror="window.stolen=true">')).toBeVisible();
    expect(document.querySelector('img[onerror]')).toBeNull();
  });

  it('does not display an online network or fabricated contacts when the core is unavailable', async () => {
    const client = api(); client.snapshot.mockRejectedValueOnce(new Error('The core is unavailable'));
    render(<ChatShell api={client} />);
    expect(await screen.findByRole('alert')).toHaveTextContent('The core is unavailable');
    expect(screen.queryByText('Shall we test delivery?')).not.toBeInTheDocument();
    expect(screen.queryByRole('textbox', { name: 'Message' })).not.toBeInTheDocument();
  });

  it('prevents empty sends and applies core updates without duplicating messages', async () => {
    const client = api(); let onUpdate: (() => void) | undefined;
    client.subscribe.mockImplementation((listener?: () => void) => { onUpdate=listener; return () => {}; });
    render(<ChatShell api={client} />); await messageInHistory('Shall we test delivery?');
    expect(screen.getByRole('button', { name: 'Send message' })).toBeDisabled();
    act(() => onUpdate?.()); await waitFor(() => expect(client.snapshot).toHaveBeenCalledTimes(2));
    expect(within(screen.getByRole('region', {name:'Message history'})).getAllByText('Shall we test delivery?')).toHaveLength(1);
  });
});

describe('low trust', () => {
  it('marks a received message whose payment could not be checked, and says why; a checked one carries no mark', async () => {
    const snapshot = initial();
    snapshot.conversations[0].messages.push({ ...snapshot.conversations[0].messages[0], id: 'm2', text: 'Written on the LAN', createdAt: 101, lowTrust: true });
    render(<ChatShell api={api(snapshot)} />);
    const offline = (await messageInHistory('Written on the LAN')).closest('article')!;
    const mark = within(offline).getByText('Low trust');
    expect(mark).toBeVisible();
    expect(mark).toHaveAttribute('title', 'Received directly while the network could not be reached: the sender’s payment was not checked.');
    const checked = (await messageInHistory('Shall we test delivery?')).closest('article')!;
    expect(within(checked).queryByText('Low trust')).not.toBeInTheDocument();
  });
});

describe('honest delivery statuses', () => {
  it('does not call seven storage receipts delivered or fully replicated', () => {
    render(<DeliveryBadge delivery={{ phase: 'stored', replicas: 7, target: 10 }} />);
    expect(screen.getByText('Copies 7/10 · repairing')).toBeVisible();
    expect(screen.queryByText('Delivered')).not.toBeInTheDocument();
  });
  it('distinguishes target storage from recipient acknowledgment', () => {
    const view=render(<DeliveryBadge delivery={{ phase: 'stored', replicas: 10, target: 10 }} />);
    expect(screen.getByText('Stored 10/10')).toBeVisible();
    view.rerender(<DeliveryBadge delivery={{ phase: 'delivered', replicas: 10, target: 10 }} />);
    expect(screen.getByText('Delivered')).toBeVisible();
    expect(screen.queryByText('Read')).not.toBeInTheDocument();
  });
});

describe('profile and contact entry', () => {
  it('creates a local profile with the chosen name and reaches the chats past every optional step', async () => {
    const client=api({identity:null,network:{connectedPeers:0,state:'offline'},conversations:[]});
    const user=userEvent.setup(); render(<ChatShell api={client}/>);
    await user.click(await screen.findByRole('button',{name:'Get started'}));
    await user.type(await screen.findByRole('textbox',{name:'Your name'}),'Alice');
    client.snapshot.mockResolvedValue(initial());
    await user.click(screen.getByRole('button',{name:'Continue'}));
    expect(client.createIdentity).toHaveBeenCalledWith({name:'Alice'});
    await user.click(await screen.findByRole('button',{name:'Skip for now'}));
    await user.click(await screen.findByRole('button',{name:'Later'}));
    await user.click(await screen.findByRole('button',{name:'Skip'}));
    await messageInHistory('Shall we test delivery?');
  });
  it('passes the complete invitation and contact name to the core and surfaces validation errors', async () => {
    const client=api(); const user=userEvent.setup(); client.addContact.mockRejectedValueOnce(new Error('The invitation signature is wrong'));
    render(<ChatShell api={client}/>); await messageInHistory('Shall we test delivery?');
    await user.click(screen.getByRole('button',{name:'Add contact'}));
    await user.click(await screen.findByText('Other ways: my ID and an invitation code'));
    await user.type(screen.getByRole('textbox',{name:'Contact name'}),'Carl');
    await user.type(screen.getByRole('textbox',{name:'Received invitation code'}),'ain-invite-signed');
    await user.click(screen.getByRole('button',{name:'Save contact'}));
    expect(client.addContact).toHaveBeenCalledWith({name:'Carl',invitation:'ain-invite-signed'});
    expect(await screen.findByRole('alert')).toHaveTextContent('The invitation signature is wrong');
    expect(screen.getByRole('textbox',{name:'Received invitation code'})).toHaveValue('ain-invite-signed');
  });
});

async function messageInHistory(text:string) {return within(await screen.findByRole('region',{name:'Message history'})).findByText(text);}
const message=(id:string,text=id)=>({...initial().conversations[0].messages[0],id,text});

describe('bounded conversation history',()=>{
  it('loads full recent bodies separately from previews and retries older pages without duplicates',async()=>{
    const client=api();const user=userEvent.setup();
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1001','The full last message')],nextBefore:'1001'});
    render(<ChatShell api={client}/>);
    await messageInHistory('The full last message');
    expect(within(screen.getByRole('region',{name:'Message history'})).queryByText('Shall we test delivery?')).not.toBeInTheDocument();
    expect(client.conversationHistory).toHaveBeenCalledWith({conversationId:'bob',before:null});
    client.conversationHistory.mockRejectedValueOnce(new Error('History is temporarily unavailable'));
    await user.click(screen.getByRole('button',{name:'Load earlier messages'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('History is temporarily unavailable');
    await messageInHistory('The full last message');
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('999'),message('1000')],nextBefore:null});
    await user.click(screen.getByRole('button',{name:'Load earlier messages'}));
    await messageInHistory('999');
    expect(client.conversationHistory.mock.calls.slice(1).map(c=>c[0])).toEqual([{conversationId:'bob',before:'1001'},{conversationId:'bob',before:'1001'}]);
    expect([...screen.getByRole('region',{name:'Message history'}).querySelectorAll('article p')].map(p=>p.textContent)).toEqual(['999','1000','The full last message']);
    expect(screen.queryByRole('button',{name:'Load earlier messages'})).not.toBeInTheDocument();
  });
  it('keeps loaded older messages when live updates overlap and refreshes delivery status',async()=>{
    const client=api();const user=userEvent.setup();let update=()=>{};
    client.subscribe.mockImplementation(listener=>{update=listener;return()=>{};});
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1001')],nextBefore:'1001'});
    render(<ChatShell api={client}/>);await messageInHistory('1001');
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1000')],nextBefore:null});
    await user.click(screen.getByRole('button',{name:'Load earlier messages'}));await messageInHistory('1000');
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1001'),message('1002')],nextBefore:'1001'});
    act(()=>update());await messageInHistory('1002');
    expect([...screen.getByRole('region',{name:'Message history'}).querySelectorAll('article p')].map(p=>p.textContent)).toEqual(['1000','1001','1002']);
    expect(screen.queryByRole('button',{name:'Load earlier messages'})).not.toBeInTheDocument();
  });
  it('keeps a bounded recent window until the owner explicitly loads an older page',async()=>{
    const client=api();let update=()=>{};
    client.subscribe.mockImplementation(listener=>{update=listener;return()=>{};});
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('0'),message('1')],nextBefore:null});
    render(<ChatShell api={client}/>);await messageInHistory('0');
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1'),message('2')],nextBefore:'1'});
    act(()=>update());await messageInHistory('2');
    expect([...screen.getByRole('region',{name:'Message history'}).querySelectorAll('article p')].map(p=>p.textContent)).toEqual(['1','2']);
    expect(screen.getByRole('button',{name:'Load earlier messages'})).toBeVisible();
  });
  it('does not mix a late older page into a newly selected conversation',async()=>{
    const data=initial();data.conversations.push({id:'carol',title:'Caroline',unread:0,messages:[message('carol-message','A talk with Caroline')]});
    const client=api(data);const user=userEvent.setup();
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('bob-new')],nextBefore:'bob-new'});
    render(<ChatShell api={client}/>);await messageInHistory('bob-new');
    let finish!:(page:{conversationId:string;messages:ReturnType<typeof message>[];nextBefore:null})=>void;
    client.conversationHistory.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
    await user.click(screen.getByRole('button',{name:'Load earlier messages'}));
    await user.click(screen.getByRole('button',{name:/Caroline/}));await messageInHistory('A talk with Caroline');
    await act(async()=>finish({conversationId:'bob',messages:[message('bob-old')],nextBefore:null}));
    const history=screen.getByRole('region',{name:'Message history'});
    expect(within(history).queryByText('bob-old')).not.toBeInTheDocument();
    expect(within(history).queryByText('bob-new')).not.toBeInTheDocument();
  });
  it('refreshes delivery receipts on already loaded older messages outside the recent page',async()=>{
    const client=api();const user=userEvent.setup();let update=()=>{};
    client.subscribe.mockImplementation(listener=>{update=listener;return()=>{};});
    const outgoing=(id:string,phase:'queued'|'delivered')=>({...message(id),own:true,delivery:{phase,replicas:0,target:10}});
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[outgoing('50','queued')],nextBefore:'50'});
    render(<ChatShell api={client}/>);await messageInHistory('50');
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[outgoing('49','queued')],nextBefore:null});
    await user.click(screen.getByRole('button',{name:'Load earlier messages'}));await messageInHistory('49');
    expect(screen.getAllByText('Queued')).toHaveLength(2);
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[outgoing('50','delivered')],nextBefore:'50'}).mockResolvedValueOnce({conversationId:'bob',messages:[outgoing('49','delivered')],nextBefore:null});
    act(()=>update());
    await waitFor(()=>expect(screen.getAllByText('Delivered')).toHaveLength(2));
    expect(screen.queryByText('Queued')).not.toBeInTheDocument();
    expect(client.conversationHistory).toHaveBeenLastCalledWith({conversationId:'bob',before:'50'});
  });
  it('exposes the intervening cursor when a live burst no longer overlaps loaded history',async()=>{
    const client=api();let update=()=>{};const user=userEvent.setup();
    client.subscribe.mockImplementation(listener=>{update=listener;return()=>{};});
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1000')],nextBefore:null});
    render(<ChatShell api={client}/>);await messageInHistory('1000');
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1100')],nextBefore:'1100'});
    act(()=>update());await messageInHistory('1100');
    expect(within(screen.getByRole('region',{name:'Message history'})).queryByText('1000')).not.toBeInTheDocument();
    client.conversationHistory.mockResolvedValueOnce({conversationId:'bob',messages:[message('1099')],nextBefore:'1099'});
    await user.click(screen.getByRole('button',{name:'Load earlier messages'}));await messageInHistory('1099');
    expect(client.conversationHistory).toHaveBeenLastCalledWith({conversationId:'bob',before:'1100'});
  });
  it('offers a retry when loading the selected history fails',async()=>{
    const client=api();const user=userEvent.setup();client.conversationHistory.mockRejectedValueOnce(new Error('Failed to load the history'));
    render(<ChatShell api={client}/>);
    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to load the history');
    await user.click(screen.getByRole('button',{name:'Retry loading history'}));
    await messageInHistory('Shall we test delivery?');
  });
});


describe('sharing an invitation', () => {
  it('shows the complete core-issued invitation without replacing a composed draft', async () => {
    const client=api(); const user=userEvent.setup();render(<ChatShell api={client}/>);
    await messageInHistory('Shall we test delivery?');
    await user.type(screen.getByRole('textbox',{name:'Message'}),'An unfinished reply');
    await openScreen(user,'Contacts & requests');
    await user.click(await screen.findByText('Other ways: my ID and an invitation code'));
    await user.click(screen.getByRole('button',{name:'Create invitation'}));
    const invitation=await screen.findByRole('textbox',{name:'Your invitation code'});
    expect(invitation).toHaveValue('ain-invite1:complete-signed-public-invitation');
    expect(invitation).toHaveAttribute('readonly');
    expect(client.createInvitation).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole('button',{name:'Back to chat'}));
    expect(screen.getByRole('textbox',{name:'Message'})).toHaveValue('An unfinished reply');
    expect(screen.getByText('Peers connected: 3')).toBeVisible();
  });
  it('surfaces invitation creation failure and retries without inventing a usable code', async () => {
    const client=api();const user=userEvent.setup();client.createInvitation.mockRejectedValueOnce(new Error('No route available'));
    render(<ChatShell api={client}/>);await messageInHistory('Shall we test delivery?');
    await openScreen(user,'Contacts & requests');
    await user.click(await screen.findByText('Other ways: my ID and an invitation code'));
    await user.click(screen.getByRole('button',{name:'Create invitation'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('No route available');
    expect(screen.queryByRole('textbox',{name:'Your invitation code'})).not.toBeInTheDocument();
    await user.click(screen.getByRole('button',{name:'Create invitation'}));
    expect(await screen.findByRole('textbox',{name:'Your invitation code'})).toHaveValue('ain-invite1:complete-signed-public-invitation');
    expect(client.createInvitation).toHaveBeenCalledTimes(2);
  });
});

it('opens runtime permissions and returns without losing the chat draft',async()=>{
  const client=api();const user=userEvent.setup();render(<ChatShell api={client}/>);
  await messageInHistory('Shall we test delivery?');await user.type(screen.getByRole('textbox',{name:'Message'}),'A reply after setting up the agent');
  await openScreen(user,'Agents');
  expect(await screen.findByRole('heading',{name:'Agent access'})).toBeVisible();expect(client.listRuntimes).toHaveBeenCalled();
  await user.click(screen.getByRole('button',{name:'Back to chat'}));
  expect(screen.getByRole('textbox',{name:'Message'})).toHaveValue('A reply after setting up the agent');
});

it('opens network settings and preserves the composed chat across panel navigation',async()=>{
  const client=api();const user=userEvent.setup();render(<ChatShell api={client}/>);
  await messageInHistory('Shall we test delivery?');await user.type(screen.getByRole('textbox',{name:'Message'}),'A reply after setting up the network');
  await openScreen(user,'Settings');
  expect(await screen.findByRole('heading',{name:'Network settings'})).toBeVisible();
  await user.click(screen.getByRole('button',{name:'Back to chat'}));
  expect(screen.getByRole('textbox',{name:'Message'})).toHaveValue('A reply after setting up the network');
});

it('keeps the owner screens behind the gear: a badge for waiting requests, Escape and outside clicks close it',async()=>{
  const client=api();const user=userEvent.setup();
  client.introRequests.mockResolvedValue([{requestId:'r1',networkId:'ain1-bob',name:'Bob',receivedAt:100}]);
  render(<ChatShell api={client}/>);await messageInHistory('Shall we test delivery?');
  expect(screen.queryByRole('menuitem')).not.toBeInTheDocument();
  const gear=screen.getByRole('button',{name:'Menu'});
  await waitFor(()=>expect(gear).toHaveAccessibleDescription('waiting for a decision: 1'));
  await user.click(gear);
  expect(gear).toHaveAttribute('aria-expanded','true');
  const menu=screen.getByRole('menu',{name:'Menu'});
  expect(within(menu).getAllByRole('menuitem').map(item=>item.id)).toEqual(['nav-contacts','nav-discover','nav-new-group','nav-agents','nav-wallet','nav-settings']);
  expect(within(menu).getByRole('menuitem',{name:/Contacts & requests/})).toHaveFocus();
  expect(within(menu).getByLabelText('waiting for a decision: 1')).toBeVisible();
  await user.keyboard('{ArrowUp}');
  expect(within(menu).getByRole('menuitem',{name:'Settings'})).toHaveFocus();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  expect(gear).toHaveFocus();
  await user.click(gear);await user.click(screen.getByRole('region',{name:'Message history'}));
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  await openScreen(user,'Wallet');
  expect(await screen.findByRole('heading',{name:'Wallet'})).toBeVisible();
  expect(screen.queryByRole('menu')).not.toBeInTheDocument();
});
