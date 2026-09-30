from pathlib import Path
import hashlib,json
r=Path.cwd();o=r/'output/dht-roles-planning'
source=(r/'apps/desktop/tests/native-e2e.mjs').read_text()
start=source.index('\ntry {\n  assert.ok((await stat(mcpBinary))')
cleanup=source[source.rindex('\n} finally {'):]
prefix=source[:start]
prefix=prefix.replace("from './", "from '"+(r/'apps/desktop/tests').as_uri()+'/')
prefix=prefix.replace("const root = resolve(import.meta.dirname, '../../..');",'const root = '+json.dumps(str(r))+';')
prefix=prefix.replace("join(root, 'output/native-e2e')", "join(root, 'output/dht-roles-planning/native-overview')")
body='''
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
  await writeFile(join(evidence,'result.json'),JSON.stringify({passed:true,platform:'macOS WKWebView',supplementalVisualInspection:true,canonicalTestsUnchanged:true,binaryHashes:hashes,outcomes},null,2)+'\\n');
  console.log('Supplemental native overview screenshots captured');
'''
(o/'capture-overview.mjs').write_text(prefix+body+cleanup)
(o/'capture-overview-inputs.json').write_text(json.dumps({str(p.relative_to(r)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [r/'apps/desktop/tests/native-e2e.mjs',r/'apps/desktop/tests/native-network.mjs',o/'capture-overview.mjs']},indent=2)+'\n')
