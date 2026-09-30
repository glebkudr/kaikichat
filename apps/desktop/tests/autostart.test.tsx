import {render,screen,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,describe,expect,it} from 'vitest';
import {SettingsPanel} from '../src/SettingsPanel';
import {autostart,fakeApi} from './fake-api';

afterEach(()=>{localStorage.clear();});

describe('opening at login in the settings',()=>{
  it('opens the app at login until the owner turns it off, and back on',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const section=within(await screen.findByRole('region',{name:'Start at login'}));
    const open=await section.findByRole('checkbox',{name:'Open Kaiki Chat when you log in'});
    expect(open).toBeChecked();
    await user.click(open);
    expect(api.setAutostart).toHaveBeenLastCalledWith({on:false});
    expect(open).not.toBeChecked();
    await user.click(open);
    expect(api.setAutostart).toHaveBeenLastCalledWith({on:true});
    expect(open).toBeChecked();
  });
  it('asks the owner to allow it in the system’s login items when the system blocks it',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.autostart.mockResolvedValue(autostart({state:'blocked'}));
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const section=within(await screen.findByRole('region',{name:'Start at login'}));
    expect(await section.findByText('The system’s login items keep Kaiki Chat from opening at login. Allow it there.')).toBeVisible();
    await user.click(section.getByRole('button',{name:'Open login items'}));
    expect(api.openLoginItems).toHaveBeenCalledTimes(1);
  });
  it('says so when the owner turns it on and the system still blocks it',async()=>{
    const api=fakeApi();const user=userEvent.setup();
    api.autostart.mockResolvedValue(autostart({state:'off'}));
    api.setAutostart.mockResolvedValueOnce(autostart({state:'blocked'}));
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    const section=within(await screen.findByRole('region',{name:'Start at login'}));
    const open=await section.findByRole('checkbox',{name:'Open Kaiki Chat when you log in'});
    expect(open).not.toBeChecked();
    expect(section.queryByRole('button',{name:'Open login items'})).toBeNull();
    await user.click(open);
    expect(api.setAutostart).toHaveBeenCalledWith({on:true});
    expect(await section.findByRole('button',{name:'Open login items'})).toBeVisible();
    expect(open).toBeChecked();
  });
  it('has no such setting where the app cannot open at login',async()=>{
    const api=fakeApi();
    api.autostart.mockResolvedValue(null);
    render(<SettingsPanel api={api} onBack={()=>{}}/>);
    await screen.findByRole('region',{name:'Updates'});
    expect(api.autostart).toHaveBeenCalled();
    expect(screen.queryByRole('region',{name:'Start at login'})).toBeNull();
  });
});
