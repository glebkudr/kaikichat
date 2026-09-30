// Aggregate independently produced evidence for one immutable RC. No tests run here.
// Each result names its actual selector, fixture, evidence owner and hashed files.
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve, dirname} from 'node:path';

const root=resolve(import.meta.dirname,'..');
const read=async path=>JSON.parse(await readFile(path,'utf8'));
const digest=async path=>createHash('sha256').update(await readFile(path)).digest('hex');

export async function aggregate(manifestPath) {
  const manifest=await read(manifestPath);
  const scope=await read(resolve(root,'Docs/agentic_internet_v1_execution_plan/release-scope.json'));
  const required={task:scope.required_v1_task_ids,e2e:scope.required_v1_case_ids,
    suite:scope.required_suite_ids,platform:scope.required_platforms};
  const failures=[];
  const accepted=new Set();
  const hashes=['source','locks','config','genesis','oracle','binaries'];
  const fingerprint=manifest.fingerprints;
  if(!/^[a-f0-9]{40}$/.test(manifest.revision??''))failures.push('Missing RC revision');
  for(const kind of hashes) {
    if(!/^[a-f0-9]{64}$/.test(fingerprint?.[kind]??''))failures.push(`Missing RC fingerprint: ${kind}`);
  }
  const results=[];
  for(const name of manifest.results??[]) {
    try {
      const path=resolve(dirname(manifestPath),name);
      const result=await read(path);
      const problems=[];
      if(result.revision!==manifest.revision || hashes.some(k=>result.fingerprints?.[k]!==fingerprint?.[k]))problems.push('stale RC');
      if(result.status!=='passed' || result.exit_code!==0 || result.failed!==0 || result.skipped_required!==0)problems.push('failed, skipped or incomplete result');
      if(!Number.isSafeInteger(result.executed) || result.executed<1 || !Number.isSafeInteger(result.expected_count) || result.expected_count<1 || result.executed<result.expected_count)problems.push('empty or incomplete selection');
      if(!result.selector || !result.fixture || !result.owner || !Array.isArray(result.command) || !result.command.length)problems.push('missing execution binding');
      if(!Array.isArray(result.artifacts) || !result.artifacts.length)problems.push('missing artifacts');
      for(const artifact of result.artifacts??[]) {
        if(await digest(resolve(dirname(path),artifact.path))!==artifact.sha256)problems.push(`artifact hash mismatch: ${artifact.path}`);
      }
      if(!Array.isArray(result.covers) || !result.covers.length)problems.push('missing coverage');
      for(const cover of result.covers??[]) {
        if(!required[cover.kind]?.includes(cover.id))problems.push(`out of scope: ${cover.kind}/${cover.id}`);
      }
      if(problems.length)failures.push(`${name}: ${problems.join('; ')}`);
      else for(const cover of result.covers)accepted.add(`${cover.kind}/${cover.id}`);
      results.push({path:name,accepted:problems.length===0});
    } catch(error) { failures.push(`${name}: ${error.message}`); }
  }
  for(const [kind,ids] of Object.entries(required))for(const id of ids) {
    if(!accepted.has(`${kind}/${id}`))failures.push(`missing ${kind}/${id}`);
  }
  for(const [id,dependencies] of Object.entries(scope.v1_task_dependencies)) {
    if(accepted.has(`task/${id}`))for(const dependency of dependencies) {
      if(!accepted.has(`task/${dependency}`))failures.push(`${id}: missing dependency ${dependency}`);
    }
  }
  return {revision:manifest.revision,product_validated:failures.length===0,scope_reduced:false,results,failures};
}

if(process.argv[1] && resolve(process.argv[1])===resolve(import.meta.filename)) {
  if(process.argv.length!==3)throw new Error('Usage: release-evidence.mjs RC-manifest.json');
  const report=await aggregate(resolve(process.argv[2]));
  console.log(JSON.stringify(report,null,2));
  process.exitCode=report.product_validated?0:1;
}
