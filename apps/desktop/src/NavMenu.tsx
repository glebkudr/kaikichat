import {useEffect,useRef,useState,type KeyboardEvent} from 'react';
import {useT} from './i18n';

export type NavItem={id:string;label:string;current:boolean;badge?:number;onSelect:()=>void};

/** The owner's screens behind one gear button. */
export function NavMenu({items,disabled}:{items:NavItem[];disabled:boolean}) {
  const t=useT();
  const [open,setOpen]=useState(false);
  const root=useRef<HTMLDivElement>(null);const toggle=useRef<HTMLButtonElement>(null);
  const waiting=items.reduce((sum,item)=>sum+(item.badge??0),0);
  useEffect(()=>{
    if(!open)return;
    root.current?.querySelector<HTMLElement>('[role=menuitem]:not(:disabled)')?.focus();
    const outside=(event:PointerEvent)=>{if(!root.current?.contains(event.target as Node))setOpen(false);};
    document.addEventListener('pointerdown',outside);
    return()=>document.removeEventListener('pointerdown',outside);
  },[open]);
  function keys(event:KeyboardEvent) {
    if(!open)return;
    if(event.key==='Escape'){event.preventDefault();setOpen(false);toggle.current?.focus();return;}
    if(event.key!=='ArrowDown'&&event.key!=='ArrowUp')return;
    event.preventDefault();
    const entries=[...root.current!.querySelectorAll<HTMLElement>('[role=menuitem]')];
    const at=entries.indexOf(document.activeElement as HTMLElement);
    entries[(at+(event.key==='ArrowDown'?1:entries.length-1))%entries.length]?.focus();
  }
  return <div className="nav-menu" ref={root} onKeyDown={keys}>
    <button id="nav-menu" ref={toggle} type="button" className="icon-button gear" aria-label={t.shell.menu} aria-haspopup="menu" aria-expanded={open} aria-controls={open?'nav-menu-items':undefined} aria-describedby={waiting?'nav-menu-waiting':undefined} onClick={()=>setOpen(!open)}>
      <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
      {waiting>0&&<span className="gear-badge" aria-hidden="true"/>}
    </button>
    {waiting>0&&<span id="nav-menu-waiting" className="sr-only">{t.shell.waiting(waiting)}</span>}
    {open&&<div className="nav-menu-items" id="nav-menu-items" role="menu" aria-label={t.shell.menu}>
      {items.map(item=><button key={item.id} id={`nav-${item.id}`} type="button" role="menuitem" className={`secondary agent-nav ${item.current?'current':''}`} disabled={disabled} onClick={()=>{setOpen(false);item.onSelect();}}>{item.label}{item.badge?<span className="unread" aria-label={t.shell.waiting(item.badge)}>{item.badge}</span>:null}</button>)}
    </div>}
  </div>;
}
