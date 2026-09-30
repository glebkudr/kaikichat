import {createRoot} from 'react-dom/client';
import {invoke,isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {ChatShell} from './ChatShell';
import {createDesktopApi} from './desktop-api';
import './styles.css';
import {applyTheme,savedTheme} from './theme';
applyTheme(savedTheme());
const api=createDesktopApi({
  invoke:(command,args)=>isTauri()?invoke(command,args):Promise.reject({code:'core_outside_app',message:'core_outside_app',retryable:false}),
  listen:(event,handler)=>isTauri()?listen(event,handler):Promise.resolve(()=>{}),
});
const root=document.getElementById('root');
if(root)createRoot(root).render(<ChatShell api={api}/>);
