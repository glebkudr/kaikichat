import {render,screen,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {afterEach,describe,expect,it} from 'vitest';
import {ChatShell} from '../src/ChatShell';
import {CoreError} from '../src/core-error';
import {describe as describeError,locales} from '../src/i18n';
import {fakeApi} from './fake-api';

afterEach(()=>{localStorage.clear();document.documentElement.removeAttribute('dir');});


/** Every leaf as `path: kind`, functions with their arity. */
function shape(value:unknown,path='',out:string[]=[]):string[] {
  if(typeof value==='function')out.push(`${path}: fn/${value.length}`);
  else if(value&&typeof value==='object')for(const key of Object.keys(value).sort())shape((value as Record<string,unknown>)[key],path?`${path}.${key}`:key,out);
  else out.push(`${path}: ${typeof value}`);
  return out;
}
/** Every leaf with its path. */
function leaves(value:unknown,path='',out=new Map<string,unknown>()):Map<string,unknown> {
  if(value&&typeof value==='object')for(const [key,item] of Object.entries(value))leaves(item,path?`${path}.${key}`:key,out);
  else out.set(path,value);
  return out;
}
const cyrillic=/[А-Яа-яЁё]/;
/** Every shown text by its path; a text made from values is made with markers. */
function texts(table:unknown):Map<string,string> {
  const out=new Map<string,string>();
  for(const [path,value] of leaves(table)) {
    if(['tag','decimal','dir','language.short'].includes(path))continue;
    if(typeof value==='string')out.set(path,value);
    else if(typeof value==='function')out.set(path,String(value(...Array.from({length:value.length},(_,index)=>`v${7001+index}`))));
  }
  return out;
}
const plain=(text:string)=>text.toLowerCase().replace(/v70\d\d/g,'').replace(/[\s\p{P}\p{S}\d]+/gu,' ').trim();
/** Words without acronyms such as ETH or CLI; Latin words of three letters
 * or more, so that "de" or "in" do not count as English. */
const words=(text:string)=>(text.match(/\p{L}+/gu)??[]).filter(word=>!/^\p{Lu}+$/u.test(word)&&(word.length>=3||!/^\p{Script=Latin}+$/u.test(word))).map(word=>word.toLowerCase());
/** Languages with a script of their own: every text shows it. */
const scripts:Record<string,RegExp>={ja:/[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}]/u,zh:/\p{Script=Han}/u,ko:/\p{Script=Hangul}/u,fa:/\p{Script=Arabic}/u,ar:/\p{Script=Arabic}/u,ru:/\p{Script=Cyrillic}/u,uk:/\p{Script=Cyrillic}/u};
/** Texts a language keeps as in English on purpose: the brand, and words it
 * borrows as they are. Any other text that reads as English is a stand-in. */
const sameAsEnglish:Record<string,string[]>={
  '*':['shell.coreEyebrow','welcome.eyebrow'],
  de:['invite.nameLabel','shell.navWallet','wallet.label','wallet.title','roles.admin','groups.admin','groups.adminLabel','wallet.shop','wallet.ethTitle','wallet.usdcTitle','groups.team'],
  fr:['shell.messages','shell.message','shell.conversations','agents.conversations','shell.navAgents','contacts.label','network.routes','shell.menu'],
  id:['roles.admin','groups.admin','groups.adminLabel','shell.menu'],
  it:['unlock.password','wallet.ethTitle','wallet.usdcTitle','shell.menu'],
  pl:['shell.menu'],
  pt:['shell.menu'],
  nl:['network.routes','wallet.ethTitle','wallet.usdcTitle','shell.menu','groups.team'],
};

const code=(table:typeof locales.en)=>(Object.entries(locales).find(([,item])=>item===table)?.[0]??'').toUpperCase();

describe('localization tables',()=>{
  it('offer the twenty most used languages of the web, each named in itself',()=>{
    expect(Object.keys(locales)).toEqual(['en','es','de','ja','fr','pt','ru','it','nl','pl','tr','zh','fa','vi','cs','id','ko','uk','hu','ar']);
    expect(Object.values(locales).map(table=>table.language.name)).toEqual(['English','Español','Deutsch','日本語','Français','Português','Русский','Italiano','Nederlands','Polski','Türkçe','简体中文','فارسی','Tiếng Việt','Čeština','Bahasa Indonesia','한국어','Українська','Magyar','العربية']);
    expect(Object.entries(locales).filter(([,table])=>table.language.short!==code(table)).map(([key])=>key)).toEqual([]);
    expect(Object.entries(locales).filter(([,table])=>table.dir==='rtl').map(([key])=>key)).toEqual(['fa','ar']);
  });
  it('translate every text into every language: the same keys, nothing missing, extra or empty',()=>{
    const english=new Set(shape(locales.en));
    for(const [code,table] of Object.entries(locales)) {
      const own=new Set(shape(table));
      expect({missing:[...english].filter(key=>!own.has(key)),extra:[...own].filter(key=>!english.has(key))},code).toEqual({missing:[],extra:[]});
      expect([...texts(table)].filter(([,text])=>!text.trim()).map(([path])=>path),code).toEqual([]);
      if(code!=='ru'&&code!=='uk')expect([...texts(table)].filter(([,text])=>cyrillic.test(text)).map(([path])=>path),code).toEqual([]);
    }
  });
  it('translate for real: no text is English left as a stand-in, apart from what each language keeps',()=>{
    const english=texts(locales.en);
    const vocabulary=new Set([...english.values()].flatMap(words));
    const standIns:Record<string,string[]>={},translatedAfterAll:Record<string,string[]>={};
    for(const [code,table] of Object.entries(locales)) {
      if(code==='en')continue;
      const kept=new Set([...sameAsEnglish['*'],...sameAsEnglish[code]??[]]);
      const script=scripts[code];
      const copied=[...texts(table)].filter(([path,text])=>{
        const same=plain(text)===plain(english.get(path)??'');
        const englishWords=words(text).length>=2&&words(text).every(word=>vocabulary.has(word));
        return same||englishWords||(script!==undefined&&!script.test(text));
      }).map(([path])=>path);
      const unexpected=copied.filter(path=>!kept.has(path)),stale=[...kept].filter(path=>!copied.includes(path));
      if(unexpected.length)standIns[code]=unexpected.map(path=>`${path}: ${texts(table).get(path)}`);
      if(stale.length)translatedAfterAll[code]=stale;
    }
    expect({standIns,translatedAfterAll}).toEqual({standIns:{},translatedAfterAll:{}});
  });
  it('keep every value a text is made of',()=>{
    for(const [code,table] of Object.entries(locales))for(const [path,value] of leaves(table)) {
      if(typeof value!=='function')continue;
      const values=Array.from({length:value.length},(_,index)=>`v${7001+index}`);
      const text=String(value(...values));
      expect(values.filter(item=>!text.includes(item)),`${code} ${path}: ${text}`).toEqual([]);
    }
  });
  it('keep every shown text in the tables: no Cyrillic in the sources outside the Russian and Ukrainian tables',()=>{
    const sources=import.meta.glob('../src/**/*.{ts,tsx}',{query:'?raw',import:'default',eager:true}) as Record<string,string>;
    const offenders=Object.entries(sources).filter(([path,text])=>!/\/i18n\/(ru|uk)\.ts$/.test(path)&&cyrillic.test(text)).map(([path])=>path);
    expect(offenders).toEqual([]);
  });
  it('name the app Kaiki Chat in every language; Agentic Internet stays the name of the protocol',()=>{
    for(const [code,table] of Object.entries(locales)) {
      expect(table.shell.coreEyebrow,code).toBe('KAIKI CHAT');
      expect([...texts(table)].filter(([,text])=>/agentic\s+internet/i.test(text)).map(([path])=>path),code).toEqual([]);
    }
  });
  it('shows a known refusal in the current language and an unknown one as the daemon said it',()=>{
    const refusal=new CoreError('network_unavailable','no directory',true);
    expect(describeError(locales.en,refusal)).toBe(locales.en.errors.network_unavailable);
    expect(describeError(locales.ru,refusal)).toBe(locales.ru.errors.network_unavailable);
    expect(describeError(locales.en,new CoreError('brand_new_code','as the daemon said',false))).toBe('as the daemon said');
  });
});

describe('language choice',()=>{
  it('starts in English, offers every language from the sidebar, switches and remembers the choice',async()=>{
    const user=userEvent.setup();
    const first=render(<ChatShell api={fakeApi()}/>);
    expect(await screen.findByRole('heading',{name:'Messages'})).toBeVisible();
    expect(document.documentElement.lang).toBe('en');
    expect(document.documentElement.dir).toBe('ltr');
    const language=within(screen.getByRole('complementary')).getByRole('combobox',{name:'Language'});
    expect(language).toHaveValue('en');
    expect(within(language).getAllByRole('option').map(option=>option.textContent)).toEqual(Object.values(locales).map(table=>table.language.name));
    await user.selectOptions(language,'ru');
    expect(await screen.findByRole('heading',{name:'Сообщения'})).toBeVisible();
    await user.click(screen.getByRole('button',{name:'Меню'}));
    expect(screen.getByRole('menuitem',{name:'Контакты и запросы'})).toBeVisible();
    expect(document.documentElement.lang).toBe('ru');
    first.unmount();
    render(<ChatShell api={fakeApi()}/>);
    expect(await screen.findByRole('heading',{name:'Сообщения'})).toBeVisible();
    await user.selectOptions(within(screen.getByRole('complementary')).getByRole('combobox',{name:'Язык'}),'ar');
    expect(await screen.findByRole('heading',{name:locales.ar.shell.messages})).toBeVisible();
    expect(document.documentElement.lang).toBe('ar');
    expect(document.documentElement.dir).toBe('rtl');
    await user.selectOptions(within(screen.getByRole('complementary')).getByRole('combobox',{name:locales.ar.language.label}),'en');
    expect(await screen.findByRole('heading',{name:'Messages'})).toBeVisible();
    expect(document.documentElement.dir).toBe('ltr');
  });
});

describe('theme',()=>{
  it('starts dark, switches from the sidebar, shows the same choice in the settings and keeps it',async()=>{
    const user=userEvent.setup();
    delete document.documentElement.dataset.theme;
    const first=render(<ChatShell api={fakeApi()}/>);
    await screen.findByRole('heading',{name:'Messages'});
    expect(document.documentElement.dataset.theme).toBe('dark');
    const sidebar=within(within(screen.getByRole('complementary')).getByRole('group',{name:'Theme'}));
    expect(sidebar.getByRole('button',{name:'Dark'})).toHaveAttribute('aria-pressed','true');
    await user.click(sidebar.getByRole('button',{name:'Light'}));
    expect(document.documentElement.dataset.theme).toBe('light');
    await user.click(screen.getByRole('button',{name:'Menu'}));await user.click(screen.getByRole('menuitem',{name:'Settings'}));
    const settings=within(within(await screen.findByRole('region',{name:'Settings'})).getByRole('group',{name:'Theme'}));
    expect(settings.getByRole('button',{name:'Light'})).toHaveAttribute('aria-pressed','true');
    await user.click(settings.getByRole('button',{name:'Dark'}));
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(sidebar.getByRole('button',{name:'Dark'})).toHaveAttribute('aria-pressed','true');
    await user.click(sidebar.getByRole('button',{name:'Light'}));
    expect(settings.getByRole('button',{name:'Light'})).toHaveAttribute('aria-pressed','true');
    first.unmount();delete document.documentElement.dataset.theme;
    render(<ChatShell api={fakeApi()}/>);
    await screen.findByRole('heading',{name:'Messages'});
    expect(document.documentElement.dataset.theme).toBe('light');
  });
});
