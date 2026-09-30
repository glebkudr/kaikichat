import type {DesktopApi} from './types';
import {AutostartSection} from './Autostart';
import {LanguageSelect,useT} from './i18n';
import {IntroPolicyPanel} from './IntroPolicyPanel';
import {NetworkPanel} from './NetworkPanel';
import {NetworkPresetSection} from './NetworkPreset';
import {ReleaseSection} from './Release';
import {ThemeSwitch} from './theme';

/** The window's own preferences, who may write by ID, then the node's network settings. */
export function SettingsPanel({api,onBack}:{api:DesktopApi;onBack:()=>void}) {
  const t=useT();
  return <section className="agent-panel settings-panel" aria-label={t.settings.title}>
    <header className="agent-header"><div><span className="eyebrow">{t.settings.eyebrow}</span><h2>{t.settings.title}</h2></div><button className="secondary" onClick={onBack}>{t.common.back}</button></header>
    <section className="agent-form appearance" aria-label={t.settings.appearance}>
      <h3>{t.settings.appearance}</h3>
      <div className="setting-row"><span>{t.settings.theme}</span><ThemeSwitch/></div>
      <div className="setting-row"><span>{t.settings.language}</span><LanguageSelect/></div>
    </section>
    <AutostartSection api={api}/>
    <NetworkPresetSection api={api}/>
    <ReleaseSection api={api}/>
    <IntroPolicyPanel api={api}/>
    <NetworkPanel api={api} onBack={onBack} embedded/>
  </section>;
}
