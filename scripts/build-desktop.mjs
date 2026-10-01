// Run with Node >=26 from any directory. Builds the current host architecture.
import {execFileSync} from 'node:child_process';
import {copyFileSync, mkdirSync} from 'node:fs';
import {resolve, join} from 'node:path';
const root=resolve(import.meta.dirname,'..');
if(process.platform!=='darwin')throw new Error('This producer builds the macOS app bundle; Linux/Windows installers require their own platform gate');
execFileSync('python3',[join(root,'scripts/build-storage.py'),'--profile',process.env.AIN_BUILD_STORAGE_PROFILE||'mac-apfs','check'],{stdio:'inherit'});
const desktop=join(root,'apps/desktop');
const debug=process.argv.includes('--debug');
const e2e=process.argv.includes('--e2e');
if(e2e&&!debug)throw new Error('Automation bundles require --debug; release builds exclude the driver');
const outputAt=process.argv.indexOf('--output');
if(outputAt>=0&&!process.argv[outputAt+1])throw new Error('--output requires a fresh build attempt directory');
const helper=join(root,'scripts/build_evidence.py');
const begun=JSON.parse(execFileSync('python3',[helper,'desktop-begin','--node',process.execPath,...(debug?['--debug']:[]),...(e2e?['--e2e']:[]),...(outputAt>=0?['--output',resolve(process.argv[outputAt+1])]:[])],{cwd:root,encoding:'utf8'}));
let phase='typescript';
const run=(name,program,args,cwd=root)=>{phase=name;return execFileSync('python3',[helper,'stage','--output',begun.output,'--name',name,'--cwd',cwd,'--',program,...args],{cwd:root,stdio:'inherit'});};
try {
run('typescript',process.execPath,['node_modules/typescript/bin/tsc','--noEmit'],desktop);
run('vite',process.execPath,['node_modules/vite/bin/vite.js','build'],desktop);
run('native-node','cargo',['build','--locked','-p','agentic-node',...(debug?[]:['--release'])]);
phase='host-target';
const target=execFileSync('rustc',['--print','host-tuple'],{encoding:'utf8'}).trim();
phase='sidecar-copy';
const binaries=join(desktop,'src-tauri/binaries');mkdirSync(binaries,{recursive:true});
for(const name of ['kaiki-agentic-node','agentic-mcp','agentic-cli','kaiki'])copyFileSync(join(root,`target/${debug?'debug':'release'}/${name}`),join(binaries,`${name}-${target}`));
run('tauri',process.execPath,['node_modules/@tauri-apps/cli/tauri.js','build','--ci','--bundles','app','--config','src-tauri/tauri.bundle.conf.json',...(debug?['--debug']:[]),...(e2e?['--features','e2e']:[]),'--','--locked'],desktop);
phase='seal';
execFileSync('python3',[helper,'seal','--output',begun.output],{cwd:root,stdio:'inherit'});
} catch(error) {
  execFileSync('python3',[helper,'failure','--output',begun.output,'--stage',phase,'--error',String(error)],{cwd:root,stdio:'inherit'});
  throw error;
}
