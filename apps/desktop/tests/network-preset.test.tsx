import {render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,describe,expect,it} from 'vitest';
import {ChatShell} from '../src/ChatShell';
import {SettingsPanel} from '../src/SettingsPanel';
import {coreError} from '../src/core-error';
import type {NetworkPreset} from '../src/types';
import {fakeApi,preset} from './fake-api';

afterEach(()=>localStorage.clear());

const offered=(overrides:Partial<NetworkPreset>={}):NetworkPreset=>preset({state:'switch',offered:{network:'kaiki-main',name:'Kaiki main network',serial:7},...overrides});

describe('the network preset in the settings',()=>{
  it('names the network from kaikichat.com and checks it again on request',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.networkPreset.mockResolvedValueOnce(preset({state:'cached',error:'timed out'}));
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const section=within(await screen.findByRole('region',{name:'Network'}));
    expect(await section.findByText('Kaiki testnet (Base Sepolia)')).toBeVisible();
    expect(section.getByText('Settings from kaikichat.com')).toBeVisible();
    expect(section.getByText(/^Last checked /)).toBeVisible();
    expect(section.getByText('kaikichat.com could not be checked; the saved settings are in use.')).toBeVisible();
    await user.click(section.getByRole('button',{name:'Check again'}));
    expect(api.refreshNetwork).toHaveBeenCalledWith({switch:false});
    await waitFor(()=>expect(section.queryByText('kaikichat.com could not be checked; the saved settings are in use.')).toBeNull());
  });
  it('says when the network is set by hand, and offers no check',async()=>{
    const api=fakeApi();
    api.networkPreset.mockResolvedValue(preset({source:'manual',state:'manual',network:null,name:null,serial:null,checkedAt:null}));
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const section=within(await screen.findByRole('region',{name:'Network'}));
    expect(await section.findByText('Set by hand with network flags')).toBeVisible();
    expect(section.queryByRole('button',{name:'Check again'})).toBeNull();
  });
});

describe('the network preset in the window',()=>{
  it('shows nothing while the network is current',async()=>{
    render(<ChatShell api={fakeApi()}/>);
    await screen.findByRole('heading',{name:'Messages'});
    await waitFor(()=>expect(screen.queryByRole('region',{name:'Network notice'})).toBeNull());
  });
  it('asks again when the settings could not be fetched',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.networkPreset.mockResolvedValueOnce(preset({state:'unavailable',network:null,name:null,serial:null,error:'offline'}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Network notice'}));
    expect(notice.getByText('Kaiki Chat could not get its network settings from kaikichat.com.')).toBeVisible();
    const snapshots=api.snapshot.mock.calls.length;
    await user.click(notice.getByRole('button',{name:'Try again'}));
    expect(api.refreshNetwork).toHaveBeenCalledWith({switch:false});
    await waitFor(()=>expect(screen.queryByRole('region',{name:'Network notice'})).toBeNull());
    expect(api.snapshot.mock.calls.length).toBeGreaterThan(snapshots);
  });
  it('offers another network and moves only when the owner asks',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.networkPreset.mockResolvedValue(offered());
    api.refreshNetwork.mockResolvedValueOnce(preset({network:'kaiki-main',name:'Kaiki main network',serial:7}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Network notice'}));
    expect(notice.getByText('The network Kaiki main network is available. Coins of the current network do not carry over.')).toBeVisible();
    expect(api.refreshNetwork).not.toHaveBeenCalled();
    await user.click(notice.getByRole('button',{name:'Switch'}));
    expect(notice.getByText('You cannot come back to the current network by yourself. Switch to Kaiki main network?')).toBeVisible();
    await user.click(notice.getByRole('button',{name:'Cancel'}));
    expect(api.refreshNetwork).not.toHaveBeenCalled();
    await user.click(notice.getByRole('button',{name:'Switch'}));
    await user.click(notice.getByRole('button',{name:'Switch now'}));
    expect(api.refreshNetwork).toHaveBeenCalledWith({switch:true});
    await waitFor(()=>expect(screen.queryByRole('region',{name:'Network notice'})).toBeNull());
  });
  it('asks for a newer app when the preset needs one',async()=>{
    const api=fakeApi();
    api.networkPreset.mockResolvedValue(preset({state:'update',required:'0.2.0'}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Network notice'}));
    expect(notice.getByText('Update Kaiki Chat to version 0.2.0 or later to join the new network.')).toBeVisible();
    expect(notice.queryByRole('button')).toBeNull();
  });
  it('shows a refusal of the switch and keeps the offer',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.networkPreset.mockResolvedValue(offered());
    api.refreshNetwork.mockRejectedValueOnce(coreError({code:'no_network_offer',message:'no other network is offered',retryable:false}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Network notice'}));
    await user.click(notice.getByRole('button',{name:'Switch'}));
    await user.click(notice.getByRole('button',{name:'Switch now'}));
    expect(await notice.findByRole('alert')).toHaveTextContent('No other network is offered now.');
    expect(notice.getByRole('button',{name:'Switch now'})).toBeEnabled();
  });
  it('shows the notice during the first run too',async()=>{
    const api=fakeApi();
    api.snapshot.mockResolvedValue({identity:null,network:{connectedPeers:0,state:'offline'},conversations:[]});
    api.networkPreset.mockResolvedValue(preset({state:'unavailable',network:null,name:null,serial:null}));
    render(<ChatShell api={api}/>);
    expect(await screen.findByRole('region',{name:'Network notice'})).toBeVisible();
  });
});
