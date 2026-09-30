import {act,render,screen,waitFor} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {describe,it,expect,vi} from 'vitest';
import {NetworkPanel} from '../src/NetworkPanel';
import type {DesktopApi} from '../src/types';
import {networkFixture as settings} from './network-fixture';
import {fakeApi} from './fake-api';
const relay='/ip4/203.0.113.20/tcp/4001/p2p/12D3KooWJm4jMSpfp8p5bVfpqzW2TjzWEdY9xw8rGzBz5Xq6f6Lo';
const verifier='/ip4/203.0.113.30/udp/4001/quic-v1/p2p/12D3KooWNWXCuxZww8PKPi3QKNtSByLMyoaQZuWZzkpyWT1LXomb';

function client() {
  let update:()=>void=()=>{};
  return {
    ...fakeApi(),
    networkSettings:vi.fn(async()=>settings()),configureNetwork:vi.fn<DesktopApi['configureNetwork']>(),
    subscribe:vi.fn((listener:()=>void)=>{update=listener;return()=>{update=()=>{};};}),
    update:()=>update(),
  } satisfies DesktopApi & {update:()=>void};
}
const save=()=>screen.getByRole('button',{name:'Save and reconnect'});

it('saves DHT service explicitly and retains the same role and revision after a lost reply',async()=>{
  const api=client();const user=userEvent.setup();render(<NetworkPanel api={api} onBack={()=>{}}/>);
  const checkbox=await screen.findByRole('checkbox',{name:'Help the network find nodes'});
  expect(checkbox).not.toBeChecked();expect(save()).toBeDisabled();
  expect(screen.getByText('Node lookup: queries only')).toBeVisible();
  await user.click(checkbox);
  expect(api.configureNetwork).not.toHaveBeenCalled();
  expect(screen.getByText('Node lookup: queries only')).toBeVisible();
  const committed=settings();committed.revision=4;committed.preferences.dhtServer=true;
  committed.status.routing={enabled:true,mode:'server',blockedByPolicy:false};
  api.configureNetwork.mockRejectedValueOnce(new Error('The reply was lost')).mockResolvedValueOnce(committed);
  await user.click(save());expect(await screen.findByRole('alert')).toHaveTextContent('The reply was lost');
  api.networkSettings.mockResolvedValue(committed);act(()=>api.update());
  expect(await screen.findByText('Node lookup: serving the network')).toBeVisible();
  expect(checkbox).toBeChecked();
  await user.click(save());await screen.findByRole('status');
  expect(api.configureNetwork.mock.calls[0][0]).toEqual({expectedRevision:3,preferences:{...settings().preferences,dhtServer:true}});
  expect(api.configureNetwork.mock.calls[1][0]).toEqual(api.configureNetwork.mock.calls[0][0]);
  expect(save()).toBeDisabled();
});

it('loads a saved DHT role, keeps a stale edit and explicitly reports relay suppression',async()=>{
  const api=client();const user=userEvent.setup();const initial=settings();
  initial.preferences.dhtServer=true;initial.status.routing={enabled:true,mode:'server',blockedByPolicy:false};
  api.networkSettings.mockResolvedValue(initial);
  render(<NetworkPanel api={api} onBack={()=>{}}/>);
  const checkbox=await screen.findByRole('checkbox',{name:'Help the network find nodes'});
  expect(checkbox).toBeChecked();await user.click(checkbox);
  const blocked=structuredClone(initial);blocked.revision=5;blocked.preferences.relayOnly=true;blocked.preferences.relays=[relay];
  blocked.status.routing={enabled:false,mode:'disabled',blockedByPolicy:true};
  api.networkSettings.mockResolvedValue(blocked);act(()=>api.update());
  expect(await screen.findByText('Node lookup is off in relay mode.')).toBeVisible();
  expect(checkbox).not.toBeChecked();
  api.configureNetwork.mockRejectedValueOnce(new Error('The settings changed'));
  await user.click(save());await screen.findByRole('alert');
  expect(api.configureNetwork.mock.calls[0][0]).toEqual({expectedRevision:3,preferences:{...initial.preferences,dhtServer:false}});
  await user.click(screen.getByRole('button',{name:'Reload saved settings'}));
  await waitFor(()=>expect(checkbox).toBeChecked());expect(save()).toBeDisabled();
  await user.click(checkbox);
  const disabled=structuredClone(blocked);disabled.revision=6;disabled.preferences.dhtServer=false;
  api.configureNetwork.mockResolvedValue(disabled);await user.click(save());await screen.findByRole('status');
  expect(api.configureNetwork.mock.calls[1][0]).toEqual({expectedRevision:5,preferences:disabled.preferences});
  expect(checkbox).not.toBeChecked();expect(screen.queryByText('Node lookup: serving the network')).not.toBeInTheDocument();
});
async function edit(user:ReturnType<typeof userEvent.setup>) {
  await user.type(await screen.findByRole('textbox',{name:'Relay providers'}),relay);
  await user.click(screen.getByRole('checkbox',{name:'Relay only'}));
  await user.type(screen.getByRole('textbox',{name:'External reachability check'}),verifier);
}
describe('network preferences for the owner',()=>{
  it('loads real settings, applies only an explicit save, and reports pending routes honestly',async()=>{
    const api=client();const user=userEvent.setup();const saved=settings();
    saved.revision=4;saved.preferences={relays:[relay],relayOnly:true,autoNatPeers:[verifier],lanDiscovery:false,dhtServer:false};saved.status.holePunchEnabled=false;
    api.configureNetwork.mockResolvedValue(saved);
    render(<NetworkPanel api={api} onBack={()=>{}}/>);
    await screen.findByRole('textbox',{name:'Relay providers'});
    expect(save()).toBeDisabled();expect(screen.getByText('External reachability: not checked')).toBeVisible();
    await edit(user);expect(api.configureNetwork).not.toHaveBeenCalled();
    await user.click(save());
    await screen.findByRole('status');
    expect(api.configureNetwork).toHaveBeenCalledExactlyOnceWith({expectedRevision:3,preferences:{relays:[relay],relayOnly:true,autoNatPeers:[verifier],lanDiscovery:false,dhtServer:false}});
    expect(screen.getByText('Relay: 0 of 1')).toBeVisible();
    expect(screen.queryByText('Address verified')).not.toBeInTheDocument();
    expect(save()).toBeDisabled();
  });
  it('retains the exact save intent after a lost reply even if status reports a newer revision',async()=>{
    const api=client();const user=userEvent.setup();const committed=settings();committed.revision=4;committed.preferences={relays:[relay],relayOnly:true,autoNatPeers:[verifier],lanDiscovery:false,dhtServer:false};
    api.configureNetwork.mockRejectedValueOnce(new Error('The core reply was lost')).mockResolvedValueOnce(committed);
    render(<NetworkPanel api={api} onBack={()=>{}}/>);await edit(user);await user.click(save());
    expect(await screen.findByRole('alert')).toHaveTextContent('The core reply was lost');
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    api.networkSettings.mockResolvedValue(committed);act(()=>api.update());
    expect(await screen.findByText('The saved settings changed. Retry the earlier save or load the current values.')).toBeVisible();
    expect(screen.getByRole('textbox',{name:'Relay providers'})).toHaveValue(relay);
    await user.click(save());await screen.findByRole('status');
    expect(api.configureNetwork.mock.calls[1][0]).toEqual(api.configureNetwork.mock.calls[0][0]);
    expect(api.configureNetwork.mock.calls[1][0].expectedRevision).toBe(3);
  });
  it('updates diagnostics without replacing the form, and explicitly reloads after a stale conflict',async()=>{
    const api=client();const user=userEvent.setup();render(<NetworkPanel api={api} onBack={()=>{}}/>);await edit(user);
    const newer=settings();newer.revision=5;newer.preferences={relays:[verifier],relayOnly:false,autoNatPeers:[],lanDiscovery:false,dhtServer:false};newer.status.autoNat.status='public';newer.status.autoNat.publicAddress='/ip4/203.0.113.44/tcp/4200';
    api.networkSettings.mockResolvedValue(newer);act(()=>api.update());
    expect(await screen.findByText('Address verified')).toBeVisible();
    expect(screen.getByRole('textbox',{name:'Relay providers'})).toHaveValue(relay);
    api.configureNetwork.mockRejectedValueOnce(new Error('Network settings changed; reload the current settings'));
    await user.click(save());await screen.findByRole('alert');
    expect(api.configureNetwork.mock.calls[0][0].expectedRevision).toBe(3);
    expect(screen.getByRole('checkbox',{name:'Relay only'})).toBeChecked();
    await user.click(screen.getByRole('button',{name:'Reload saved settings'}));
    await waitFor(()=>expect(screen.getByRole('textbox',{name:'Relay providers'})).toHaveValue(verifier));
    expect(screen.getByRole('checkbox',{name:'Relay only'})).not.toBeChecked();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();expect(save()).toBeDisabled();
    await user.click(screen.getByRole('checkbox',{name:'Relay only'}));
    api.configureNetwork.mockResolvedValue({...newer,revision:6,preferences:{...newer.preferences,relayOnly:true}});
    await user.click(save());await screen.findByRole('status');
    expect(api.configureNetwork.mock.calls[1][0].expectedRevision).toBe(5);
  });
  it('does not invent editable defaults or connectivity after a load failure, and can retry',async()=>{
    const api=client();api.networkSettings.mockRejectedValueOnce(new Error('The local node is unavailable'));
    const user=userEvent.setup();render(<NetworkPanel api={api} onBack={()=>{}}/>);
    expect(await screen.findByRole('alert')).toHaveTextContent('The local node is unavailable');
    expect(screen.queryByRole('textbox',{name:'Relay providers'})).not.toBeInTheDocument();
    expect(screen.queryByText('Address verified')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button',{name:'Reload saved settings'}));
    expect(await screen.findByRole('textbox',{name:'Relay providers'})).toHaveValue('');
    expect(api.configureNetwork).not.toHaveBeenCalled();
  });
  it('requires a relay for relay-only mode and bounds the number of providers before submission',async()=>{
    const api=client();const user=userEvent.setup();render(<NetworkPanel api={api} onBack={()=>{}}/>);
    await screen.findByRole('textbox',{name:'Relay providers'});
    await user.click(screen.getByRole('checkbox',{name:'Relay only'}));expect(save()).toBeDisabled();
    const routes=screen.getByRole('textbox',{name:'Relay providers'});await user.type(routes,Array(5).fill(relay).join('\n'));
    expect(save()).toBeDisabled();expect(screen.getByText('Give at most 4 addresses in each list.')).toBeVisible();
    expect(api.configureNetwork).not.toHaveBeenCalled();
  });
});

it('loads saved bootstrap hints, preserves edits during live diagnostics and retries the original revision',async()=>{
  const api=client();const user=userEvent.setup();const initial=settings();initial.preferences.bootstrapPeers=[relay];
  api.networkSettings.mockResolvedValue(initial);
  render(<NetworkPanel api={api} onBack={()=>{}}/>);
  const input=await screen.findByRole('textbox',{name:'Nodes to join the network'});
  expect(input).toHaveValue(relay);expect(save()).toBeDisabled();
  expect(screen.getByText('Network: no verified connections')).toBeVisible();
  await user.clear(input);await user.type(input,verifier);expect(api.configureNetwork).not.toHaveBeenCalled();
  const committed=settings();committed.revision=4;committed.preferences.bootstrapPeers=[verifier];
  committed.status.bootstrap.state='connected';committed.status.bootstrap.verifiedPeers=[{peerId:'live-peer',rootId:'ain1-signed-root'}];
  api.configureNetwork.mockRejectedValueOnce(new Error('The core reply was lost')).mockResolvedValueOnce(committed);
  await user.click(save());await screen.findByRole('alert');
  api.networkSettings.mockResolvedValue(committed);act(()=>api.update());
  expect(await screen.findByText('Network: verified nodes — 1')).toBeVisible();
  expect(input).toHaveValue(verifier);
  await user.click(save());await screen.findByRole('status');
  expect(api.configureNetwork.mock.calls[0][0]).toEqual({expectedRevision:3,preferences:{relays:[],relayOnly:false,autoNatPeers:[],bootstrapPeers:[verifier],lanDiscovery:false,dhtServer:false}});
  expect(api.configureNetwork.mock.calls[1][0]).toEqual(api.configureNetwork.mock.calls[0][0]);
  expect(save()).toBeDisabled();
});

it('bounds bootstrap entries and explains relay-only blocked hints without inventing verified connectivity',async()=>{
  const api=client();const user=userEvent.setup();const initial=settings();
  initial.preferences={relays:[relay],relayOnly:true,autoNatPeers:[],bootstrapPeers:[verifier],lanDiscovery:false,dhtServer:false};
  initial.status.connectedPeers=1;initial.status.bootstrap.policyBlockedHints=1;
  initial.status.bootstrap.candidateHints=1;initial.status.bootstrap.failedAttempts=2;
  api.networkSettings.mockResolvedValue(initial);
  render(<NetworkPanel api={api} onBack={()=>{}}/>);
  const input=await screen.findByRole('textbox',{name:'Nodes to join the network'});
  expect(screen.getByText('Network: no verified connections')).toBeVisible();
  expect(screen.getByText('Hints without a relay are not used in this mode: 1.')).toBeVisible();
  expect(screen.queryByText('Network: verified nodes — 1')).not.toBeInTheDocument();
  await user.clear(input);await user.type(input,Array(5).fill(verifier).join('\n'));
  expect(save()).toBeDisabled();expect(screen.getByText('Give at most 4 addresses in each list.')).toBeVisible();
  expect(api.configureNetwork).not.toHaveBeenCalled();
  await user.clear(input);expect(save()).toBeEnabled();
  const cleared={...initial,revision:4,preferences:{relays:[relay],relayOnly:true,autoNatPeers:[],lanDiscovery:false,dhtServer:false}};
  api.configureNetwork.mockResolvedValue(cleared);await user.click(save());await screen.findByRole('status');
  // An empty field gives the routes back to the network.
  expect(api.configureNetwork.mock.calls[0][0].preferences).not.toHaveProperty('bootstrapPeers');expect(save()).toBeDisabled();
});

it('saves other settings over the network routes without naming them, and keeps named routes untouched',async()=>{
  const api=client();const user=userEvent.setup();const initial=settings();initial.status.bootstrap.routes=[relay,verifier];
  api.networkSettings.mockResolvedValue(initial);
  render(<NetworkPanel api={api} onBack={()=>{}}/>);
  const input=await screen.findByRole('textbox',{name:'Nodes to join the network'});
  // The network's routes are shown, not put into the field as the owner's.
  expect(input).toHaveValue('');expect(input).toHaveAttribute('placeholder',`${relay}\n${verifier}`);
  await user.click(screen.getByRole('checkbox',{name:'Find nodes on the local network'}));
  const lan=structuredClone(initial);lan.revision=4;lan.preferences.lanDiscovery=true;
  api.configureNetwork.mockResolvedValueOnce(lan);await user.click(save());await screen.findByRole('status');
  expect(api.configureNetwork.mock.calls[0][0]).toEqual({expectedRevision:3,preferences:{relays:[],relayOnly:false,autoNatPeers:[],lanDiscovery:true,dhtServer:false}});
  expect(api.configureNetwork.mock.calls[0][0].preferences).not.toHaveProperty('bootstrapPeers');
  await user.type(input,verifier);
  const named=structuredClone(lan);named.revision=5;named.preferences.bootstrapPeers=[verifier];named.status.bootstrap.routes=[verifier];
  api.configureNetwork.mockResolvedValueOnce(named);await user.click(save());
  await waitFor(()=>expect(api.configureNetwork).toHaveBeenCalledTimes(2));
  expect(api.configureNetwork.mock.calls[1][0]).toEqual({expectedRevision:4,preferences:{relays:[],relayOnly:false,autoNatPeers:[],bootstrapPeers:[verifier],lanDiscovery:true,dhtServer:false}});
  await waitFor(()=>expect(save()).toBeDisabled());expect(input).toHaveValue(verifier);
  await user.click(screen.getByRole('checkbox',{name:'Help the network find nodes'}));
  api.configureNetwork.mockResolvedValueOnce({...named,revision:6,preferences:{...named.preferences,dhtServer:true}});await user.click(save());
  await waitFor(()=>expect(api.configureNetwork).toHaveBeenCalledTimes(3));
  expect(api.configureNetwork.mock.calls[2][0]).toEqual({expectedRevision:5,preferences:{relays:[],relayOnly:false,autoNatPeers:[],bootstrapPeers:[verifier],lanDiscovery:true,dhtServer:true}});
});

it('saves an empty field as the network routes, also over an empty list an earlier version saved',async()=>{
  const api=client();const user=userEvent.setup();const initial=settings();initial.preferences.bootstrapPeers=[];
  api.networkSettings.mockResolvedValue(initial);render(<NetworkPanel api={api} onBack={()=>{}}/>);
  await user.click(await screen.findByRole('checkbox',{name:'Help the network find nodes'}));
  api.configureNetwork.mockResolvedValue({...initial,revision:4,preferences:{...settings().preferences,dhtServer:true}});
  await user.click(save());await screen.findByRole('status');
  expect(api.configureNetwork.mock.calls[0][0]).toEqual({expectedRevision:3,preferences:{relays:[],relayOnly:false,autoNatPeers:[],lanDiscovery:false,dhtServer:true}});
  expect(api.configureNetwork.mock.calls[0][0].preferences).not.toHaveProperty('bootstrapPeers');
});

it('requires an explicit LAN opt-in, preserves its retry across polling, and explains relay policy',async()=>{
  const api=client();const user=userEvent.setup();render(<NetworkPanel api={api} onBack={()=>{}}/>);
  const checkbox=await screen.findByRole('checkbox',{name:'Find nodes on the local network'});
  expect(checkbox).not.toBeChecked();expect(save()).toBeDisabled();
  expect(screen.getByText('Local discovery is off.')).toBeVisible();
  await user.click(checkbox);expect(api.configureNetwork).not.toHaveBeenCalled();
  const committed=settings();committed.revision=4;committed.preferences.lanDiscovery=true;
  committed.status.lanDiscovery={enabled:true,active:true,blockedByPolicy:false,peers:['unverified-lan-peer']};
  api.configureNetwork.mockRejectedValueOnce(new Error('The reply was lost')).mockResolvedValueOnce(committed);
  await user.click(save());await screen.findByRole('alert');
  api.networkSettings.mockResolvedValue(committed);act(()=>api.update());
  expect(await screen.findByText('Local discovery is on.')).toBeVisible();
  expect(screen.getByText('Network: no verified connections')).toBeVisible();
  expect(checkbox).toBeChecked();
  await user.click(save());await screen.findByRole('status');
  expect(api.configureNetwork.mock.calls[0][0]).toEqual({expectedRevision:3,preferences:{...settings().preferences,lanDiscovery:true,dhtServer:false}});
  expect(api.configureNetwork.mock.calls[1][0]).toEqual(api.configureNetwork.mock.calls[0][0]);
  expect(save()).toBeDisabled();
  const blocked=structuredClone(committed);blocked.revision=5;blocked.preferences.relayOnly=true;blocked.preferences.relays=[relay];
  blocked.status.lanDiscovery={enabled:true,active:false,blockedByPolicy:true,peers:[]};
  api.networkSettings.mockResolvedValue(blocked);await user.click(screen.getByRole('button',{name:'Reload saved settings'}));
  expect(await screen.findByText('Local discovery is off in relay mode.')).toBeVisible();
  expect(checkbox).toBeChecked();expect(save()).toBeDisabled();
  await user.click(checkbox);const disabled=structuredClone(blocked);disabled.revision=6;disabled.preferences.lanDiscovery=false;
  disabled.status.lanDiscovery={enabled:false,active:false,blockedByPolicy:false,peers:[]};
  api.configureNetwork.mockResolvedValue(disabled);await user.click(save());await screen.findByRole('status');
  expect(api.configureNetwork.mock.calls.at(-1)?.[0]).toEqual({expectedRevision:5,preferences:disabled.preferences});
  expect(checkbox).not.toBeChecked();
});
