import {act,render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,describe,expect,it} from 'vitest';
import {ChatShell} from '../src/ChatShell';
import {coreError} from '../src/core-error';
import {fakeApi} from './fake-api';

afterEach(()=>localStorage.clear());

const noProfile={identity:null,network:{connectedPeers:0,state:'offline' as const},conversations:[]};

describe('an app opened outside the Applications folder',()=>{
  it('offers the move from the first screen on and moves when asked',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.snapshot.mockResolvedValue(structuredClone(noProfile));
    api.moveOffer.mockResolvedValue(true);
    let finish:()=>void=()=>{};
    api.moveToApplications.mockImplementationOnce(()=>new Promise<void>(resolve=>{finish=resolve;}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Move notice'}));
    expect(notice.getByText('Kaiki Chat is not in the Applications folder: it cannot open at login, and your agent loses it once the app quits.')).toBeVisible();
    // The first screen is still the one to go on with.
    expect(screen.getByRole('button',{name:'Get started'})).toBeVisible();
    await user.click(notice.getByRole('button',{name:'Move to Applications'}));
    expect(api.moveToApplications).toHaveBeenCalledTimes(1);
    // The app quits and opens from there: the button stays busy.
    expect(notice.getByRole('button',{name:'Moving…'})).toBeDisabled();
    await act(async()=>finish());
  });

  it('says why the move failed and lets the owner try again',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.moveOffer.mockResolvedValue(true);
    api.moveToApplications.mockRejectedValueOnce(coreError({code:'move_failed',message:'ditto: Permission denied',retryable:false}));
    render(<ChatShell api={api}/>);
    const notice=within(await screen.findByRole('region',{name:'Move notice'}));
    await user.click(notice.getByRole('button',{name:'Move to Applications'}));
    expect(await notice.findByRole('alert')).toHaveTextContent('Kaiki Chat could not be moved: drag it into the Applications folder yourself.');
    await user.click(notice.getByRole('button',{name:'Move to Applications'}));
    expect(api.moveToApplications).toHaveBeenCalledTimes(2);
  });

  it('says nothing for an app in its place',async()=>{
    const api=fakeApi();
    render(<ChatShell api={api}/>);
    await screen.findByRole('heading',{name:'Messages'});
    await waitFor(()=>expect(api.moveOffer).toHaveBeenCalled());
    expect(screen.queryByRole('region',{name:'Move notice'})).toBeNull();
  });
});
