// Actual WKWebView -> Tauri -> keychain/daemon -> libp2p/OpenMLS product flow.
// The e2e feature creates hidden windows; the driver is absent from normal builds.
import assert from 'node:assert/strict';
import {spawn, execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {realpathSync} from 'node:fs';
import {mkdtemp, mkdir, writeFile, realpath, rm, stat} from 'node:fs/promises';
import {createServer} from 'node:net';
import {resolve, join, dirname} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
import {McpClient} from 'file:///Users/glebk/Code/chat/apps/desktop/tests/mcp-client.mjs';
import {nativeNetworkScenario} from 'file:///Users/glebk/Code/chat/apps/desktop/tests/native-network.mjs';
import {nativeCheckpointScenario} from 'file:///Users/glebk/Code/chat/apps/desktop/tests/native-checkpoints.mjs';

assert.equal(process.platform, 'darwin', 'This runner exercises the macOS WKWebView build');
const root = "/Users/glebk/Code/chat";
const binary = process.env.AIN_DESKTOP_BINARY ?? join(root, 'target/debug/agentic-desktop');
const nodeBinary = join(dirname(binary), 'agentic-node');
const mcpBinary = join(dirname(binary), 'agentic-mcp');
const temporary = await mkdtemp('/tmp/ain-native-');
const evidence = join(root, 'output/dht-roles-planning/native-overview');
await mkdir(evidence, {recursive:true});
const clients = [];
const mcpClients = [];
const daemonPids = new Map();
const outcomes = [];
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
  constructor(name) {this.name=name;this.data=join(temporary,name);clients.push(this);}
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
    await until(`${this.name} React mounted`,()=>this.read('return Boolean(document.querySelector(".app-shell"));'));
    this.captureDaemons();
    return this;
  }
  captureDaemons() {
    // Also finds our orphan if the desktop exits before the WebView mounts.
    // Match the executable AND exact isolated profile/IPC arguments, never just a process name.
    try {
      const data=realpathSync(this.data);
      const prefix=`${nodeBinary} serve --profile ${data}/profile.db --ipc ${data}/node.sock --secrets-stdin`;
      for(const pid of execFileSync('pgrep',['-x','agentic-node'],{encoding:'utf8'}).trim().split(/\s+/).filter(Boolean)) {
        try {
          const command=execFileSync('ps',['-p',pid,'-o','command='],{encoding:'utf8'}).trim();
          if(command===prefix||command.startsWith(prefix+' '))daemonPids.set(Number(pid),prefix);
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
  async button(text) {
    const id=(await this.command('/element',{using:'xpath',value:`//button[normalize-space(.)='${text}']` }))[elementKey];
    await this.command(`/element/${id}/click`,{});
  }
  textIncludes(text) {return this.read('return document.body.innerText.includes(arguments[0]);',[text]);}
  async createProfile(name) {
    await until('profile onboarding',()=>this.textIncludes('What should friends call you?'));
    await this.fill('input[autocomplete="nickname"]',name);await this.button('Continue');
    await until('profile created',()=>this.textIncludes('Get free messages with Google or GitHub'));
  }
  invoke(method) {
    return this.command('/execute/async',{script:'const done=arguments[arguments.length-1]; window.__TAURI_INTERNALS__.invoke(arguments[0]).then(done,error=>done({error:String(error)}));',args:[method]});
  }
  snapshot() {return this.invoke('snapshot');}
  async screenshot(label) {await writeFile(join(evidence,`${label}.png`),Buffer.from(await this.command('/screenshot'),'base64'));}
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
        const command=execFileSync('ps',['-p',String(pid),'-o','command='],{encoding:'utf8'}).trim();
        const status=execFileSync('ps',['-p',String(pid),'-o','stat='],{encoding:'utf8'}).trim();
        return (command===prefix||command.startsWith(prefix+' '))&&!status.startsWith('Z');
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

// Supplemental manual visual inspection using the unchanged native scenario and
// its existing client/lifecycle helpers. Canonical E2E screenshots remain intact.
const originalScreenshot=Client.prototype.screenshot;
Client.prototype.screenshot=async function(label) {
  if(label.startsWith('dht-')) {
    await this.read('document.querySelector(".network-panel").scrollTop=0; return true;');
    await until('DHT control visible in native viewport',()=>this.read('const c=document.querySelector("#network-dht"); const r=c.getBoundingClientRect(); return document.querySelector(".network-panel").scrollTop===0 && r.top>=0 && r.bottom<=window.innerHeight;'));
  }
  return originalScreenshot.call(this,label);
};
try {
  outcomes.push(await nativeNetworkScenario({Client,until,nodeBinary,temporary,stopDaemon}));
  const bytes=await import('node:fs/promises');
  const hashes={};
  for(const file of ['agentic-desktop','agentic-node'])hashes[file]=createHash('sha256').update(await bytes.readFile(join(dirname(binary),file))).digest('hex');
  await writeFile(join(evidence,'result.json'),JSON.stringify({passed:true,platform:'macOS WKWebView',supplementalVisualInspection:true,canonicalTestsUnchanged:true,binaryHashes:hashes,outcomes},null,2)+'\n');
  console.log('Supplemental native overview screenshots captured');

} finally {
  for(const mcp of mcpClients)await mcp.stop();
  for(const c of clients)await c.stop();
  for(const [pid,prefix] of daemonPids) {
    await stopOwnedDaemon(pid,prefix);
  }
  for(const c of clients) {
    try {
      const profile=join(await realpath(c.data),'profile.db');
      const account=createHash('sha256').update(profile).digest('hex');
      execFileSync('security',['delete-generic-password','-s','net.agenticinternet.desktop.e2e','-a',account],{stdio:'ignore'});
    }catch{}
  }
  await rm(temporary,{recursive:true,force:true});
}
