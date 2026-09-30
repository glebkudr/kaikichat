// Actual WKWebView -> Tauri -> isolated test vault/daemon -> libp2p/OpenMLS flow.
// The e2e feature creates hidden windows; the driver is absent from normal builds.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {mkdtemp, mkdir, readFile, writeFile, realpath, stat} from 'node:fs/promises';
import {resolve, join, dirname} from 'node:path';
import {createNativeHarness} from './native-harness.mjs';
import {McpClient} from './mcp-client.mjs';
import {nativeNetworkScenario} from './native-network.mjs';

assert.ok(['darwin','linux'].includes(process.platform), 'This runner exercises the macOS WKWebView or the Linux WebKitGTK build');
const args=process.argv.slice(2);
const available=['chat','network'];
assert.ok(args.length===0 || (args.length===2 && args[0]==='--case' && available.includes(args[1])), 'Unknown or empty native case selection');
const selected=args.length?[args[1]]:available;
const root = resolve(import.meta.dirname, '../../..');
const binary = process.env.AIN_DESKTOP_BINARY ?? join(root, 'target/debug/agentic-desktop');
const nodeBinary = join(dirname(binary), 'agentic-node');
const mcpBinary = join(dirname(binary), 'agentic-mcp');
const cliBinary = join(dirname(binary), 'agentic-cli');
const ownerBinary = join(dirname(binary), 'kaiki');
const temporary = await mkdtemp('/tmp/ain-native-');
const evidence = join(root, 'output/native-e2e',`${new Date().toISOString().replaceAll(':','-')}-${selected.join('-')}`);
await mkdir(evidence, {recursive:true});
const {Client,until,stopDaemon,clients,daemonPids,cleanup}=createNativeHarness({binary,temporary,evidence});
const mcpClients=[];
const outcomes=[];
const completed=[];
const fingerprints={};

try {
  assert.ok((await stat(mcpBinary)).isFile(),'selected app must bundle agentic-mcp next to its daemon');
  assert.ok((await stat(cliBinary)).isFile(),'selected app must bundle agentic-cli next to its daemon');
  assert.ok((await stat(ownerBinary)).isFile(),'selected app must bundle the owner CLI kaiki next to its daemon');
  const ownerVersion=spawnSync(ownerBinary,['--version'],{encoding:'utf8'});
  assert.equal(ownerVersion.status,0,'the bundled owner CLI must run');
  for(const path of [binary,nodeBinary,mcpBinary,cliBinary,ownerBinary])fingerprints[path]=createHash('sha256').update(await readFile(path)).digest('hex');
  if(selected.includes('chat')) {
  const a=await new Client('alice').launch();const b=await new Client('bob').launch();
  assert.equal(daemonPids.size,2,'each fresh native profile must have its own actual daemon');
  await a.createProfile('Alice');await b.createProfile('Bob');
  const original=(await a.snapshot()).identity;
  assert.ok(original.networkId.startsWith('ain1'));
  const invitation=await b.shareInvitation();
  assert.ok(invitation.startsWith('ain-invite1:'));
  await a.addContact('Bob',invitation);
  await until('Bob receives contact without reload',()=>b.textIncludes('No messages on this device yet.'));
  const first='Hi, Bob! This is a real desktop → MLS → desktop path.';
  await a.fill('#message',first);await a.click('button[aria-label="Send message"]');
  await until('incoming message rendered without refresh',()=>b.read('return [...document.querySelectorAll(".message.received > p")].some(p=>p.textContent===arguments[0]);',[first]));
  await until('signed receipt rendered without refresh',()=>a.textIncludes('Delivered'));
  const reply='Hi, Alice! A reply from the second app.';
  await b.fill('#message',reply);await b.click('button[aria-label="Send message"]');
  await until('reply rendered',()=>a.read('return [...document.querySelectorAll(".message.received > p")].some(p=>p.textContent===arguments[0]);',[reply]));
  await until('reply receipt rendered',()=>b.textIncludes('Delivered'));
  assert.deepEqual((await a.snapshot()).conversations[0].messages.map(m=>m.text),[first,reply]);
  assert.deepEqual((await b.snapshot()).conversations[0].messages.map(m=>m.text),[first,reply]);
  await a.screenshot('alice-chat');await b.screenshot('bob-chat');
  outcomes.push('Two native webviews: UI onboarding, invitation, MLS contact, message, reply and signed delivery receipts with automatic updates');
  await a.stop();
  assert.ok(a.child.exitCode!==null||a.child.signalCode!==null,'Alice UI process must exit before sending');
  const whileClosed='A message sent while the Alice window is closed.';
  await b.fill('#message',whileClosed);await b.click('button[aria-label="Send message"]');
  await until('detached Alice daemon acknowledges',()=>b.read('return [...document.querySelectorAll(".message.own")].some(m=>m.querySelector("p")?.textContent===arguments[0] && m.querySelector(".delivery-delivered"));',[whileClosed]));
  assert.ok(a.child.exitCode!==null||a.child.signalCode!==null,'receipt was observed before Alice UI relaunch');
  await a.launch();
  await until('reopened UI has incoming history',()=>a.textIncludes(whileClosed));
  const restored=await a.snapshot();assert.deepEqual(restored.identity,original);
  assert.deepEqual(restored.conversations[0].messages.map(m=>m.text),[first,reply,whileClosed]);
  await a.screenshot('alice-reopened');
  outcomes.push('Closing/reopening the UI preserves the same isolated profile identity and complete history; independent daemon receives while UI is closed');
  const group=restored.conversations[0].id;
  await a.nav('agents');await until('native agent permissions',()=>a.textIncludes('Agent access'));
  await a.fill('#agent-name','E2 helper');
  await a.click(`input[type="checkbox"][value="${group}"]`);await a.click('#agent-allow-send');
  await a.button('Grant access');
  const config=JSON.parse(await until('real MCP config from owner UI',()=>a.read('return document.querySelector("textarea[aria-label=\\"MCP configuration\\"]")?.value;')));
  const entries=Object.values(config.mcpServers);assert.equal(entries.length,1);
  const mcpConfig=entries[0];assert.deepEqual(Object.keys(mcpConfig).sort(),['args','command']);
  assert.equal(mcpConfig.command,mcpBinary,'UI must return the selected app bundled MCP, not a build-tree binary');
  assert.equal(mcpConfig.args[0],'--credentials');assert.equal(mcpConfig.args.length,2);
  assert.ok(mcpConfig.args[1].startsWith((await realpath(a.data))+'/runtimes/'));
  const skillText=await readFile(join(root,'integrations/agent-skill/agentic-messaging/SKILL.md'),'utf8');
  assert.equal(await a.read('return document.querySelector("textarea[aria-label=\\"Skill text\\"]")?.value;'),skillText);
  assert.equal(await a.read('return document.querySelector("textarea[aria-label=\\"Skill text\\"]").readOnly;'),true);
  await a.click('details.agent-config > summary');
  await a.screenshot('alice-agent-skill');
  await a.click('details.agent-config > summary');
  await a.screenshot('alice-agents-active');
  const mcp=new McpClient(mcpConfig);mcpClients.push(mcp);await mcp.initialize();
  const runtime=await mcp.runtimeContext();
  const peerNetworkId=(await b.snapshot()).identity.networkId;
  assert.equal(runtime.networkId,original.networkId);
  assert.deepEqual(runtime.conversations,[{id:group,title:restored.conversations[0].title,networkId:peerNetworkId}]);
  assert.deepEqual(runtime.actions,['read_inbox','send_message']);assert.equal(runtime.maxDataBytes,4096);
  assert.deepEqual(Object.keys(runtime).sort(),['actions','conversations','expiresAt','grantId','maxDataBytes','principal','serviceId','agentId','version','networkId'].sort());
  const displayedCli=await until('real CLI command from owner UI',()=>a.read('return document.querySelector("textarea[aria-label=\\"CLI command\\"]")?.value;'));
  assert.ok(displayedCli.includes(cliBinary),'UI command must use the selected bundled CLI');
  const contextCommand=spawnSync('/bin/sh',['-c',displayedCli],{encoding:'utf8',timeout:15000,env:{PATH:temporary}});
  assert.equal(contextCommand.status,0,contextCommand.stdout);assert.equal(contextCommand.stderr,'');
  assert.deepEqual(JSON.parse(contextCommand.stdout),{result:runtime},'the displayed shell command works with the actual private credential path');
  const cli=(args,text='',code=0)=>{
    const result=spawnSync(cliBinary,[...mcpConfig.args,...args],{input:text,encoding:'utf8',timeout:15000,env:{PATH:temporary}});
    assert.equal(result.status,code,result.stdout);assert.equal(result.stderr,'');
    const value=JSON.parse(result.stdout);assert.deepEqual(Object.keys(value),[code?'error':'result']);
    return value[code?'error':'result'];
  };
  const agentGroup=runtime.conversations[0].id;
  const agentText='A message from an MCP server connected through desktop.';
  const sent=await mcp.success('messages.send',{conversationId:agentGroup,text:agentText,operationId:'native-mcp-send'});
  await until('MCP message rendered in peer UI',()=>b.read('return [...document.querySelectorAll(".message.received > p")].some(p=>p.textContent===arguments[0]);',[agentText]));
  assert.equal((await b.snapshot()).conversations[0].messages.at(-1).id,sent.id);
  assert.equal(cli(['messages','send','--to',peerNetworkId,'--operation-id','native-mcp-send','--text-stdin'],agentText).id,sent.id);
  const delivery=await until('agent observes signed recipient receipt through MCP',async()=>{
    const state=await mcp.success('delivery.get',{conversationId:agentGroup,operationId:'native-mcp-send'});
    return state.delivery.phase==='delivered'?state:null;
  });
  assert.deepEqual(delivery,{conversationId:agentGroup,operationId:'native-mcp-send',messageId:sent.id,delivery:{phase:'delivered',replicas:0,target:10}});
  assert.deepEqual(cli(['delivery','get','--to',peerNetworkId,'--operation-id','native-mcp-send']),delivery);
  const agentReply='A reply for the agent through a real conversation.';
  await b.fill('#message',agentReply);await b.click('button[aria-label="Send message"]');
  await until('agent reply persisted at local daemon',async()=>(await a.snapshot()).conversations[0].messages.some(m=>m.text===agentReply));
  const page=await mcp.success('inbox.poll',{conversationId:agentGroup,operationId:'native-mcp-page',limit:10,maxBytes:runtime.maxDataBytes,leaseSeconds:30});
  assert.deepEqual(page.items.map(item=>item.text),[reply,whileClosed,agentReply]);assert.equal(new Set(page.items.map(item=>item.id)).size,3);
  const ack=await mcp.success('inbox.ack',{conversationId:agentGroup,leaseId:page.leaseId});assert.equal(ack.cursor,page.cursor);
  const empty=await mcp.success('inbox.poll',{conversationId:agentGroup,operationId:'native-mcp-next',limit:10,maxBytes:runtime.maxDataBytes,leaseSeconds:30});assert.deepEqual(empty.items,[]);
  await a.click('button[aria-label="Revoke access E2 helper"]');await until('native revocation shown',()=>a.textIncludes('Revoked'));
  const cliDenied=cli(['messages','send','--to',peerNetworkId,'--operation-id','native-mcp-send','--text-stdin'],agentText,3);
  assert.equal(cliDenied.code,'unauthorized');assert.equal(cliDenied.retryable,false);
  assert.equal(await a.read('return document.querySelector("textarea[aria-label=\\"CLI command\\"]") === null;'),true);
  await assert.rejects(()=>mcp.runtimeContext(),/unauthorized/);
  const denied=await mcp.tool('messages.send',{conversationId:agentGroup,text:'Revoked client must not send',operationId:'native-mcp-denied'});
  assert.equal(denied.isError,true);assert.equal(denied.structuredContent.error.code,'unauthorized');
  const deniedDelivery=await mcp.tool('delivery.get',{conversationId:agentGroup,operationId:'native-mcp-send'});
  assert.equal(deniedDelivery.isError,true);assert.equal(deniedDelivery.structuredContent.error.code,'unauthorized');
  assert.deepEqual((await a.snapshot()).conversations[0].messages.map(m=>m.text),[first,reply,whileClosed,agentText,agentReply]);
  assert.deepEqual((await b.snapshot()).conversations[0].messages.map(m=>m.text),[first,reply,whileClosed,agentText,agentReply]);
  await a.screenshot('alice-agents-revoked');await mcp.stop(true);
  outcomes.push('Native permissions UI creates a scoped runtime; displayed MCP and CLI commands launch bundled executables, share send idempotency and delivery status, and UI revoke denies both clients');
  completed.push('chat');
  }
  const scenarios={
    network:()=>nativeNetworkScenario({Client,until,nodeBinary,temporary,stopDaemon}),
  };
  for(const name of selected.filter(name=>name!=='chat')) {
    outcomes.push(await scenarios[name]());
    completed.push(name);
  }
  assert.deepEqual([...completed].sort(),[...selected].sort());
  assert.ok(completed.length>0);
  await writeFile(join(evidence,'result.json'),JSON.stringify({passed:true,platform:process.platform==='darwin'?'macOS WKWebView':'Linux WebKitGTK',selected,completed,fingerprints,outcomes},null,2)+'\n');
  console.log(JSON.stringify({passed:true,outcomes,evidence},null,2));
} catch(error) {
  for(const c of clients) {
    await writeFile(join(evidence,`${c.name}.log`),c.log??'');
    if(c.session)await c.screenshot(`${c.name}-failure`).catch(()=>{});
  }
  await writeFile(join(evidence,'result.json'),JSON.stringify({passed:false,selected,completed,fingerprints,outcomes,error:error.stack},null,2)+'\n');
  throw error;
} finally {
  for(const mcp of mcpClients)await mcp.stop();
  await cleanup();
}
