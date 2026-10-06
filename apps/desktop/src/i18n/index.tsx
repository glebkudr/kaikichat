import {createContext,useCallback,useContext,useEffect,useMemo,useState,type ReactNode} from 'react';
import {CoreError} from '../core-error';
import {ar} from './ar';
import {cs} from './cs';
import {de} from './de';
import {en,type Messages} from './en';
import {es} from './es';
import {fa} from './fa';
import {fr} from './fr';
import {hu} from './hu';
import {id} from './id';
import {it} from './it';
import {ja} from './ja';
import {ko} from './ko';
import {nl} from './nl';
import {pl} from './pl';
import {pt} from './pt';
import {ru} from './ru';
import {tr} from './tr';
import {uk} from './uk';
import {vi} from './vi';
import {zh} from './zh';

/** The twenty most used languages of the web, the most used first. */
export const locales={en,es,de,ja,fr,pt,ru,it,nl,pl,tr,zh,fa,vi,cs,id,ko,uk,hu,ar} satisfies Record<string,Messages>;
export type Locale=keyof typeof locales;
export const defaultLocale:Locale='en';
const storageKey='agentic.locale';

/** The saved choice, if the webview's storage can be read. */
export function savedLocale():Locale {
  try {const value=localStorage.getItem(storageKey);return value&&value in locales?value as Locale:defaultLocale;}
  catch {return defaultLocale;}
}
type Context={locale:Locale;t:Messages;setLocale:(locale:Locale)=>void};
const I18n=createContext<Context>({locale:defaultLocale,t:locales[defaultLocale],setLocale:()=>{}});

export function LocaleProvider({children,initial}:{children:ReactNode;initial?:Locale}) {
  const [locale,choose]=useState<Locale>(()=>initial??savedLocale());
  const setLocale=useCallback((next:Locale)=>{
    choose(next);
    try {localStorage.setItem(storageKey,next);} catch { /* the choice lasts for this window only */ }
  },[]);
  useEffect(()=>{document.documentElement.lang=locale;document.documentElement.dir=locales[locale].dir;},[locale]);
  const value=useMemo(()=>({locale,t:locales[locale],setLocale}),[locale,setLocale]);
  return <I18n.Provider value={value}>{children}</I18n.Provider>;
}
export const useLocale=()=>useContext(I18n);
export const useT=()=>useContext(I18n).t;

/** Refusals whose message is the reason itself, from the node or macOS: the
 * owner needs it to fix the cause, so it stays beside the known text. */
const withReason=new Set(['start_failed','keychain_unavailable']);

/** An error as the owner reads it: a known refusal in the current language. */
export function describe(t:Messages,error:unknown):string {
  if(error instanceof CoreError) {
    const known=t.errors[error.code];
    if(!known)return error.message;
    return withReason.has(error.code)&&error.message!==error.code?`${known} (${error.message})`:known;
  }
  return error instanceof Error?error.message:String(error);
}
export function useDescribe() {
  const t=useT();
  return useCallback((error:unknown)=>describe(t,error),[t]);
}

/** A native list of the languages, each named in itself. The compact face
 * shows the current language's short code. */
export function LanguageSelect({compact=false}:{compact?:boolean}) {
  const {locale,t,setLocale}=useLocale();
  return <div className={`language-select${compact?' compact':''}`}>
    <span aria-hidden="true" lang={locale}>{compact?t.language.short:t.language.name}</span>
    <svg aria-hidden="true" viewBox="0 0 10 6"><path d="M1 1l4 4 4-4" fill="none" stroke="currentColor" strokeWidth="1.4"/></svg>
    <select aria-label={t.language.label} value={locale} onChange={event=>setLocale(event.target.value as Locale)}>
      {(Object.keys(locales) as Locale[]).map(code=><option key={code} value={code} lang={code}>{locales[code].language.name}</option>)}
    </select>
  </div>;
}
