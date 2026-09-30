import {render,screen,within} from '@testing-library/react';
import {afterEach,describe,expect,it} from 'vitest';
import page from '../index.html?raw';
import mainRs from '../src-tauri/src/main.rs?raw';
import bundle from '../src-tauri/tauri.conf.json';
import {ChatShell} from '../src/ChatShell';
import {fakeApi} from './fake-api';

afterEach(()=>localStorage.clear());

describe('the app is called Kaiki Chat',()=>{
  it('in the bundle, the window title, the page title and the sidebar',async()=>{
    expect(bundle.productName).toBe('Kaiki Chat');
    expect(mainRs).toContain('.title("Kaiki Chat")');
    expect(page).toContain('<title>Kaiki Chat</title>');
    render(<ChatShell api={fakeApi()}/>);
    await screen.findByRole('heading',{name:'Messages'});
    expect(within(screen.getByRole('complementary')).getByText('Kaiki Chat')).toBeVisible();
  });
});
