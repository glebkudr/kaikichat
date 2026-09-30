import {act,render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,describe,expect,it,vi} from 'vitest';
import {ChatShell} from '../src/ChatShell';
import {SettingsPanel} from '../src/SettingsPanel';
import {coreError} from '../src/core-error';
import {fakeApi,release} from './fake-api';

afterEach(()=>{localStorage.clear();vi.useRealTimers();});

const newer=release({latest:'0.3.0',available:true});

describe('a newer release in the window',()=>{
  it('says nothing while the app is the latest',async()=>{
    const api=fakeApi();
    render(<ChatShell api={api}/>);
    await screen.findByRole('heading',{name:'Messages'});
    await waitFor(()=>expect(api.release).toHaveBeenCalled());
    expect(screen.queryByRole('region',{name:'Update notice'})).toBeNull();
  });
  it('names the newer version and updates the app when asked',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.release.mockResolvedValue(newer);
    let finish:()=>void=()=>{};
    api.installUpdate.mockImplementationOnce(()=>new Promise<void>(resolve=>{finish=resolve;}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Update notice'}));
    expect(notice.getByText('Kaiki Chat 0.3.0 is available. You have 0.2.0.')).toBeVisible();
    await user.click(notice.getByRole('button',{name:'Update'}));
    expect(api.installUpdate).toHaveBeenCalledTimes(1);
    expect(notice.getByRole('button',{name:'Updating…'})).toBeDisabled();
    expect(notice.getByRole('button',{name:'Skip this version'})).toBeDisabled();
    await act(async()=>finish());
  });
  it('skips the version and says no more about it',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.release.mockResolvedValue(newer);
    api.skipRelease.mockResolvedValueOnce({...newer,skipped:true});
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Update notice'}));
    await user.click(notice.getByRole('button',{name:'Skip this version'}));
    expect(api.skipRelease).toHaveBeenCalledWith({version:'0.3.0'});
    await waitFor(()=>expect(screen.queryByRole('region',{name:'Update notice'})).toBeNull());
    expect(api.installUpdate).not.toHaveBeenCalled();
  });
  it('offers the download where the app cannot replace itself',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.release.mockResolvedValue({...newer,installable:false});
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Update notice'}));
    expect(notice.queryByRole('button',{name:'Update'})).toBeNull();
    await user.click(notice.getByRole('button',{name:'Download'}));
    expect(api.openDownloads).toHaveBeenCalledTimes(1);
    expect(api.installUpdate).not.toHaveBeenCalled();
  });
  it('says why an update failed and lets the owner try again',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.release.mockResolvedValue(newer);
    api.installUpdate.mockRejectedValueOnce(coreError({code:'hash_mismatch',message:'the download is not the announced build',retryable:false}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Update notice'}));
    await user.click(notice.getByRole('button',{name:'Update'}));
    expect(await notice.findByRole('alert')).toHaveTextContent('The download is not the announced version; nothing was changed.');
    expect(notice.getByRole('button',{name:'Update'})).toBeEnabled();
  });
  it('looks again every hour while the window is open',async()=>{
    vi.useFakeTimers({shouldAdvanceTime:true});
    const api=fakeApi();
    render(<ChatShell api={api}/>);
    await waitFor(()=>expect(api.release).toHaveBeenCalledTimes(1));
    api.release.mockResolvedValue(newer);
    await act(async()=>{await vi.advanceTimersByTimeAsync(60*60*1000);});
    await waitFor(()=>expect(api.release).toHaveBeenCalledTimes(2));
    expect(await screen.findByRole('region',{name:'Update notice'})).toBeVisible();
  });
  it('shows the release during the first run too',async()=>{
    const api=fakeApi();
    api.snapshot.mockResolvedValue({identity:null,network:{connectedPeers:0,state:'offline'},conversations:[]});
    api.release.mockResolvedValue(newer);
    render(<ChatShell api={api}/>);
    expect(await screen.findByRole('region',{name:'Update notice'})).toBeVisible();
  });
});

describe('updates in the settings',()=>{
  it('names this version and checks for a newer one on request',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.checkRelease.mockResolvedValueOnce(newer);
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const section=within(await screen.findByRole('region',{name:'Updates'}));
    expect(await section.findByText('Version 0.2.0')).toBeVisible();
    expect(section.getByText('Up to date')).toBeVisible();
    expect(section.getByText(/^Last checked /)).toBeVisible();
    expect(section.queryByRole('button',{name:'Update'})).toBeNull();
    await user.click(section.getByRole('button',{name:'Check for updates'}));
    expect(api.checkRelease).toHaveBeenCalledTimes(1);
    expect(await section.findByText('Kaiki Chat 0.3.0 is available. You have 0.2.0.')).toBeVisible();
    // A skipped version can still be installed from here.
    await user.click(section.getByRole('button',{name:'Update'}));
    expect(api.installUpdate).toHaveBeenCalledTimes(1);
  });
  it('says when kaikichat.com could not be checked',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.checkRelease.mockResolvedValueOnce(release({error:'timed out'}));
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const section=within(await screen.findByRole('region',{name:'Updates'}));
    await user.click(await section.findByRole('button',{name:'Check for updates'}));
    expect(await section.findByText('kaikichat.com could not be checked for updates.')).toBeVisible();
  });
});
