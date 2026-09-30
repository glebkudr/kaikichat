import {useSyncExternalStore,type ReactNode} from 'react';
import {useT} from './i18n';

export type Theme='dark'|'light';
const storageKey='agentic.theme';
const listeners=new Set<()=>void>();

/** The saved theme, dark unless the owner chose light. */
export function savedTheme():Theme {
  try {return localStorage.getItem(storageKey)==='light'?'light':'dark';}
  catch {return 'dark';}
}
export function applyTheme(theme:Theme) {
  document.documentElement.dataset.theme=theme;
  for(const listener of listeners)listener();
}
function currentTheme():Theme {return document.documentElement.dataset.theme==='light'?'light':'dark';}
function subscribe(listener:()=>void) {listeners.add(listener);return ()=>{listeners.delete(listener);};}
function chooseTheme(next:Theme) {
  applyTheme(next);
  try {localStorage.setItem(storageKey,next);} catch { /* the choice lasts for this window only */ }
}

const icons:Record<Theme,ReactNode>={
  dark:<svg aria-hidden="true" viewBox="0 0 16 16"><path d="M13.5 10.2A5.8 5.8 0 0 1 5.8 2.5a5.8 5.8 0 1 0 7.7 7.7z" fill="currentColor"/></svg>,
  light:<svg aria-hidden="true" viewBox="0 0 16 16"><circle cx="8" cy="8" r="3" fill="currentColor"/><path d="M8 .8v2M8 13.2v2M.8 8h2M13.2 8h2M2.9 2.9l1.4 1.4M11.7 11.7l1.4 1.4M2.9 13.1l1.4-1.4M11.7 4.3l1.4-1.4" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"/></svg>,
};

/** Dark or light for the whole window; every switch shows the same choice.
 * The compact one shows icons named for assistive technology. */
export function ThemeSwitch({compact=false}:{compact?:boolean}) {
  const t=useT();
  const theme=useSyncExternalStore(subscribe,currentTheme);
  return <div className={`language-switch theme-switch${compact?' compact':''}`} role="group" aria-label={t.settings.theme}>
    {(['dark','light'] as Theme[]).map(option=>{
      const name=option==='dark'?t.settings.dark:t.settings.light;
      return <button key={option} type="button" className={option===theme?'current':''} aria-pressed={option===theme} aria-label={compact?name:undefined} title={compact?name:undefined} onClick={()=>chooseTheme(option)}>{compact?icons[option]:name}</button>;
    })}
  </div>;
}
