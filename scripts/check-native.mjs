// macOS native gate: real commands + renderer tests, packaged hidden WKWebViews,
// then an ad-hoc signed release bundle without the automation plugin.
import {execFileSync} from 'node:child_process';
import {resolve, join} from 'node:path';
const root=resolve(import.meta.dirname,'..');
if(process.platform!=='darwin')throw new Error('The native macOS bundle gate is unsupported on this host');
const args=process.argv.slice(2);
const full=args.length===0 || (args.length===1 && args[0]==='--full');
const cases=['chat','network'];
if(!full && (args.length!==2 || args[0]!=='--case' || !cases.includes(args[1]))) {
  throw new Error(`Usage: check-native.mjs --full | --case ${cases.join('|')}`);
}
execFileSync('python3',[join(root,'scripts/build-storage.py'),'--profile',process.env.AIN_BUILD_STORAGE_PROFILE||'mac-apfs','check'],{stdio:'inherit'});
const run=(program,args,extra={})=>execFileSync(program,args,{cwd:root,stdio:'inherit',env:{...process.env,AIN_NODE:process.execPath,...extra}});
if(full)run(join(root,'scripts/check.sh'),[]);
run(process.execPath,['scripts/build-desktop.mjs','--debug','--e2e']);
run(process.execPath,['apps/desktop/tests/native-e2e.mjs',...(full?[]:args)],{AIN_DESKTOP_BINARY:join(root,'target/debug/bundle/macos/Kaiki Chat.app/Contents/MacOS/agentic-desktop')});
if(full) {
run(process.execPath,['scripts/build-desktop.mjs']);
run('codesign',['--verify','--deep','--strict',join(root,'target/release/bundle/macos/Kaiki Chat.app')]);
const dependencies=execFileSync('cargo',['tree','-p','agentic-desktop','-e','normal','--prefix','none'],{cwd:root,encoding:'utf8'});
if(dependencies.includes('tauri-plugin-wdio-webdriver'))throw new Error('Automation plugin leaked into default dependency graph');
}
console.log('Native macOS gate passed. Full V1 acceptance is tracked separately in IMPLEMENTATION_STATUS.md.');
