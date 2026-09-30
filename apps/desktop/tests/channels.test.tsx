import {render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {describe,expect,it,vi} from 'vitest';
import {GroupPanel,NewGroupPanel} from '../src/GroupPanel';
import {bobId,carolId,fakeApi,group,ownId} from './fake-api';

const daveId=`ain1${'d4'.repeat(32)}`;

describe('a group’s door',()=>{
  it('puts a door on a group on the owner’s choice',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const reading=await screen.findByRole('region',{name:'Who reads'});
    await user.click(within(reading).getByRole('button',{name:'Only by request'}));
    await waitFor(()=>expect(api.changeGroup).toHaveBeenCalledWith({access:'request',groupId:'g1',operationId:expect.any(String)}));
  });
  it('lets an admin let in or turn away who asks at the door, notes shown as text',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.group.mockResolvedValue(group({access:'request',owner:carolId,admins:[ownId],role:'admin'}));
    api.doorRequests.mockResolvedValue([
      {requestId:'r1'.repeat(32),networkId:daveId,note:'<img src=x onerror=alert(1)> let me in',receivedAt:1790000000,rejoin:false},
      {requestId:'r2'.repeat(32),networkId:bobId,note:'',receivedAt:1790000100,rejoin:false},
    ]);
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const door=await screen.findByRole('region',{name:'Asking to join'});
    // A note is text a stranger wrote.
    expect(await within(door).findByText('<img src=x onerror=alert(1)> let me in')).toBeVisible();
    expect(door.querySelector('img')).toBeNull();
    await user.click(within(door).getByRole('button',{name:/Let in ain1d4/}));
    expect(api.doorDecide).toHaveBeenCalledWith({groupId:'g1',requestId:'r1'.repeat(32),accept:true});
    await user.click(within(door).getByRole('button',{name:/Turn away ain1b2/}));
    expect(api.doorDecide).toHaveBeenLastCalledWith({groupId:'g1',requestId:'r2'.repeat(32),accept:false});
  });
  it('shows a plain member no applications',async()=>{
    const api=fakeApi();
    api.group.mockResolvedValue(group({access:'request',owner:carolId,role:'member'}));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    await screen.findByRole('region',{name:'Who reads'});
    expect(screen.queryByRole('region',{name:'Asking to join'})).not.toBeInTheDocument();
    expect(api.doorRequests).not.toHaveBeenCalled();
  });
});

describe('making a channel',()=>{
  it('makes a public channel only once the owner confirms it stays public, and a closed one without',async()=>{
    const api=fakeApi();const user=userEvent.setup();const onCreated=vi.fn();
    render(<NewGroupPanel api={api} onCreated={onCreated} onBack={()=>{}}/>);
    await user.click(screen.getByRole('radio',{name:/Channel/}));
    await user.type(screen.getByRole('textbox',{name:'Channel name'}),'News');
    await user.type(screen.getByRole('textbox',{name:'Team IDs (one per line)'}),bobId);
    await user.click(screen.getByRole('radio',{name:/Anyone reads/}));
    const make=screen.getByRole('button',{name:'Make channel'});
    expect(make).toBeDisabled();
    await user.click(screen.getByRole('checkbox',{name:/stays public for good/}));
    await user.click(make);
    expect(api.createGroup).toHaveBeenCalledWith({name:'News',members:[bobId],kind:'channel',access:'public',operationId:expect.any(String)});
    await user.click(screen.getByRole('radio',{name:/Only those given keys/}));
    await user.click(screen.getByRole('button',{name:'Make channel'}));
    expect(api.createGroup).toHaveBeenLastCalledWith({name:'News',members:[bobId],kind:'channel',access:'private',operationId:expect.any(String)});
    await waitFor(()=>expect(onCreated).toHaveBeenCalled());
  });
});

describe('a public channel’s history',()=>{
  it('shows what keeping the history costs and keeps it longer on the owner’s choice',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.group.mockResolvedValue(group({kind:'channel',access:'public',retention:90,admins:[bobId]}));
    api.channelStorage.mockResolvedValue({retention:90,parts:4,bytes:180_000,stampsPerMonth:5,addedLastMonth:1});
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const history=await screen.findByRole('region',{name:'History'});
    expect(await within(history).findByText(/4 parts/)).toBeVisible();
    expect(within(history).getByText(/5 stamps a month/)).toBeVisible();
    const kept=within(history).getByRole('combobox',{name:'Keep posts'});
    await user.selectOptions(kept,'forever');
    // For ever grows: the owner is told before it is saved.
    expect(within(history).getByText(/grows every month/)).toBeVisible();
    await user.click(within(history).getByRole('button',{name:'Save'}));
    expect(api.changeGroup).toHaveBeenCalledWith({retention:'forever',groupId:'g1',operationId:expect.any(String)});
    await user.selectOptions(kept,'180');
    await user.click(within(history).getByRole('button',{name:'Save'}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({retention:180,groupId:'g1',operationId:expect.any(String)});
  });
});

describe('a channel’s team',()=>{
  it('is added to by the owner alone; an admin only bans',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.group.mockResolvedValue(group({kind:'channel',access:'public',retention:30,admins:[bobId]}));
    const {unmount}=render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const team=await screen.findByRole('form',{name:'Add to the team'});
    expect(screen.getByRole('heading',{name:'Team · 3'})).toBeVisible();
    await user.type(within(team).getByRole('textbox',{name:'Network ID'}),daveId);
    await user.click(within(team).getByRole('button',{name:'Add'}));
    expect(api.changeGroup).toHaveBeenCalledWith({add:[daveId],groupId:'g1',operationId:expect.any(String)});
    unmount();
    api.group.mockResolvedValue(group({kind:'channel',access:'public',retention:30,owner:carolId,admins:[ownId],members:[carolId,ownId],role:'admin'}));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const bans=await screen.findByRole('form',{name:'Ban an ID'});
    expect(within(bans).queryByRole('button',{name:'Add'})).not.toBeInTheDocument();
    await user.type(within(bans).getByRole('textbox',{name:'Network ID'}),daveId);
    await user.click(within(bans).getByRole('button',{name:'Ban ID'}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({ban:[daveId],groupId:'g1',operationId:expect.any(String)});
  });
});

describe('a closed channel’s subscribers',()=>{
  it('gives keys to an id, removes a subscriber and reseeds, with no history to keep',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.group.mockResolvedValue(group({kind:'channel',access:'private',retention:30,admins:[bobId]}));
    render(<GroupPanel api={api} groupId="g1" ownId={ownId} onBack={()=>{}}/>);
    const subscribers=await screen.findByRole('region',{name:'Subscribers'});
    expect(screen.queryByRole('region',{name:'History'})).not.toBeInTheDocument();
    const field=within(subscribers).getByRole('textbox',{name:'Subscriber ID'});
    await user.type(field,daveId);
    await user.click(within(subscribers).getByRole('button',{name:'Give keys'}));
    expect(api.channelSubscribe).toHaveBeenCalledWith({groupId:'g1',members:[daveId],operationId:expect.any(String)});
    await user.type(field,daveId);
    await user.click(within(subscribers).getByRole('button',{name:'Take keys back'}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({unsubscribe:[daveId],groupId:'g1',operationId:expect.any(String)});
    await user.click(within(subscribers).getByRole('button',{name:'New keys for everyone'}));
    expect(api.changeGroup).toHaveBeenLastCalledWith({reseed:true,groupId:'g1',operationId:expect.any(String)});
  });
});
