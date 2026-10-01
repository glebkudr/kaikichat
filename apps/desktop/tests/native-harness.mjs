// Shared hidden WKWebView driver. All mutations use the real desktop build.
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {realpathSync} from 'node:fs';
import {writeFile,realpath,rm} from 'node:fs/promises';
import {createServer} from 'node:net';
import {join,dirname} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';

/** A process's command line as ps shows it. Emulated amd64 on Apple silicon
 * (Rosetta under OrbStack) shows the program path twice; drop the repeat. */
function command(pid) {
  const text=execFileSync('ps',['-p',String(pid),'-o','command='],{encoding:'utf8'}).trim();
  const [first,second,...rest]=text.split(' ');
  return first===second?[first,...rest].join(' '):text;
}
export function createNativeHarness({binary,temporary,evidence}) {
  const nodeBinary=join(dirname(binary),'kaiki-agentic-node');
  const clients=[],daemonPids=new Map();
const elementKey = 'element-6066-11e4-a52e-4f735466cecf';
async function unusedPort() {
  const server = createServer();
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  const port = server.address().port;
  await new Promise(r=>server.close(r));
  return port;
}
async function until(label, operation, timeout=20000) {
  const end=Date.now()+timeout; let last;
  while(Date.now()<end) {
    try {const value=await operation();if(value)return value;}catch(error){last=error;}
    await delay(100);
  }
  throw new Error(`Timed out: ${label}${last?`: ${last.message}`:''}`);
}
class Client {
  constructor(name,{data=join(temporary,name),manageDaemon=true}={}) {this.name=name;this.data=data;this.manageDaemon=manageDaemon;clients.push(this);}
  async launch() {
    this.port=await unusedPort();this.session=undefined;this.log='';
    this.child=spawn(binary,[],{env:{...process.env,AIN_E2E_DATA_DIR:this.data,TAURI_WEBDRIVER_PORT:String(this.port)},stdio:['ignore','pipe','pipe']});
    this.child.stdout.on('data',b=>this.log+=b);this.child.stderr.on('data',b=>this.log+=b);
    this.child.on('error',e=>this.launchError=e);
    await until(`${this.name} native driver startup`,async()=>{
      if(this.launchError)throw this.launchError;
      if(this.child.exitCode!==null)throw new Error(`desktop exited ${this.child.exitCode}: ${this.log}`);
      return (await this.http('/status')).ready;
    });
    this.session=(await this.http('/session',{capabilities:{alwaysMatch:{'tauri:options':{windowLabel:'main'}}}})).sessionId;
    await until(`${this.name} React mounted`,()=>this.read('return Boolean(document.querySelector(".app-shell,.wizard"));'));
    // The webview's storage outlives a run (on macOS it belongs to the app):
    // start every window with the default language and theme.
    if(await this.read('const kept=localStorage.getItem("agentic.locale")!==null||localStorage.getItem("agentic.theme")!==null;localStorage.removeItem("agentic.locale");localStorage.removeItem("agentic.theme");return kept;')) {
      await this.read('location.reload();return true;');
      await until(`${this.name} reloaded with defaults`,()=>this.read('return Boolean(document.querySelector(".app-shell,.wizard"))&&document.documentElement.lang==="en";'));
    }
    this.captureDaemons();
    return this;
  }
  captureDaemons() {
    if(!this.manageDaemon)return;
    // Also finds our orphan if the desktop exits before the WebView mounts.
    // Match the executable AND exact isolated profile/IPC arguments, never just a process name.
    try {
      const data=realpathSync(this.data);
      const prefix=`${nodeBinary} serve --profile ${data}/profile.db --ipc ${data}/node.sock --secrets-stdin`;
      // Linux keeps only 15 characters of a process name: match the command line.
      for(const pid of execFileSync('pgrep',['-f','kaiki-agentic-node serve'],{encoding:'utf8'}).trim().split(/\s+/).filter(Boolean)) {
        try {
          const line=command(pid);
          if(line===prefix||line.startsWith(prefix+' '))daemonPids.set(Number(pid),prefix);
        }catch { /* Another unrelated node may have exited during enumeration. */ }
      }
    } catch { /* No profile or daemon was created during this failed launch. */ }
  }
  async http(path,body) {
    const response=await fetch(`http://127.0.0.1:${this.port}${path}`,{method:body===undefined?'GET':'POST',headers:{'Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body),signal:AbortSignal.timeout(15000)});
    const result=await response.json();
    if(!response.ok||result.value?.error)throw new Error(JSON.stringify(result));
    return result.value;
  }
  command(path,body) {return this.http(`/session/${this.session}${path}`,body);}
  read(script,args=[]) {return this.command('/execute/sync',{script,args});}
  async element(selector) {return (await this.command('/element',{using:'css selector',value:selector}))[elementKey];}
  async click(selector) {const id=await this.element(selector);await this.command(`/element/${id}/click`,{});}
  async fill(selector,text) {
    const id=await this.element(selector);
    await this.command(`/element/${id}/clear`,{});
    await this.command(`/element/${id}/value`,{text});
  }
  async xclick(xpath) {
    const id=(await this.command('/element',{using:'xpath',value:xpath}))[elementKey];
    await this.command(`/element/${id}/click`,{});
  }
  async button(text) {
    const id=(await this.command('/element',{using:'xpath',value:`//button[normalize-space(.)='${text}']` }))[elementKey];
    await this.command(`/element/${id}/click`,{});
  }
  /** A screen from the gear menu: `contacts`, `new-group`, `agents`, `wallet` or `settings`. */
  async nav(screen) {
    if(!await this.read('return Boolean(document.getElementById(arguments[0]));',[`nav-${screen}`]))await this.click('#nav-menu');
    await this.click(`#nav-${screen}`);
  }
  textIncludes(text) {return this.read('return document.body.innerText.includes(arguments[0]);',[text]);}
  /** The first run up to the login; without `coins`, past every optional step. */
  async createProfile(name,{coins=false}={}) {
    await until('profile onboarding',()=>this.textIncludes('Let your AI agent talk to your friends’ agents.'));
    await this.button('Get started');
    await until('name step',()=>this.textIncludes('What should friends call you?'));
    await this.fill('input[autocomplete="nickname"]',name);await this.button('Continue');
    await until('login step',()=>this.textIncludes('Get free messages with Google or GitHub'));
    if(!coins)await this.finishOnboarding();
  }
  /** The steps after the login: the agent's instructions and the invitation wait. */
  async finishOnboarding({skipLogin=true}={}) {
    if(skipLogin)await this.button('Skip for now');
    await until('agent step',()=>this.textIncludes('Connect your agent'));
    await this.button('Later');
    await until('invite step',()=>this.textIncludes('Invite your friends'));
    await this.button('Skip');
    await until('profile created',()=>this.read('return Boolean(document.getElementById("nav-menu"));'));
  }
  /** The contacts screen with the invitation block open. */
  async invitations() {
    await this.nav('contacts');
    await until('contacts screen',()=>this.textIncludes('Other ways: my ID and an invitation code'));
    await this.read('document.querySelector("details.agent-config").open=true;return true;');
  }
  async shareInvitation() {
    await this.invitations();await this.button('Create invitation');
    const invitation=await until('real invitation',()=>this.read('return document.querySelector("textarea[aria-label=\\"Your invitation code\\"]")?.value;'));
    await this.button('Back to chat');
    return invitation;
  }
  async addContact(name,invitation) {
    await this.invitations();
    await this.fill('#invitation-name',name);await this.fill('#invitation-code',invitation);await this.button('Save contact');
    await until('contact saved',()=>this.textIncludes('Contact added by invitation.'));
    await this.button('Back to chat');
  }
  async invoke(method,args={}) {
    // A valid product value may itself contain an `error` field (wallet state).
    // Keep it inside an envelope, separate from WebDriver/Tauri transport errors.
    const value=await this.command('/execute/async',{script:'const done=arguments[arguments.length-1]; window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(value=>done({nativeResult:value}),error=>done({nativeError:String(error)}));',args:[method,args]});
    if(Object.hasOwn(value,'nativeError'))throw new Error(value.nativeError);
    return value.nativeResult;
  }
  snapshot() {return this.invoke('snapshot');}
  async screenshot(label) {
    // Hidden WKWebView keeps CSS transition clocks at zero. Capture the actual
    // final style without changing selected elements, content or app state.
    await this.read('for(const animation of document.getAnimations()){if(animation instanceof CSSTransition&&Number.isFinite(animation.effect.getComputedTiming().endTime))animation.finish();}return true;');
    await writeFile(join(evidence,`${label}.png`),Buffer.from(await this.command('/screenshot'),'base64'));
  }
  async stop() {
    this.captureDaemons();
    if(this.child&&!this.launchError&&this.child.exitCode===null&&this.child.signalCode===null) {
      this.child.kill('SIGTERM');
      await until('desktop exit',()=>this.child.exitCode!==null||this.child.signalCode!==null,5000).catch(()=>this.child.kill('SIGKILL'));
      await until('desktop reaped',()=>this.child.exitCode!==null||this.child.signalCode!==null,5000);
    }
  }
}
async function stopOwnedDaemon(pid,prefix) {
    const running=()=>{
      try {
        const line=command(pid);
        const status=execFileSync('ps',['-p',String(pid),'-o','stat='],{encoding:'utf8'}).trim();
        return (line===prefix||line.startsWith(prefix+' '))&&!status.startsWith('Z');
      }catch{return false;}
    };
    if(running())process.kill(pid,'SIGTERM');
    await until('owned daemon exit',()=>!running(),5000).catch(()=>{if(running())process.kill(pid,'SIGKILL');});
    await until('owned daemon stopped before deleting profile',()=>!running(),5000);
}
async function stopDaemon(client) {
  client.captureDaemons();
  const data=await realpath(client.data);
  const prefix=`${nodeBinary} serve --profile ${data}/profile.db --ipc ${data}/node.sock --secrets-stdin`;
  const matches=[...daemonPids].filter(([,command])=>command===prefix);
  assert.equal(matches.length,1,'restart must target exactly this isolated profile daemon');
  const [pid]=matches[0];await stopOwnedDaemon(pid,prefix);daemonPids.delete(pid);
}

async function cleanup() {
  for(const c of clients)await c.stop();
  for(const [pid,prefix] of daemonPids)await stopOwnedDaemon(pid,prefix);
  // E2E keys live only in these private temporary profiles, never login Keychain.
  await rm(temporary,{recursive:true,force:true});
}
return {Client,until,stopDaemon,clients,daemonPids,cleanup};
}
