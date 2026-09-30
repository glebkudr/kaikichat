from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile
root=Path('/Users/glebk/Library/Caches/agentic-internet/worktree')
scratch=Path(tempfile.mkdtemp(prefix='l01-proof-mutations-',dir='/Users/glebk/Library/Caches/agentic-internet'))
shutil.copytree(root/'crates/l2-types',scratch/'crates/l2-types')
manifest=(root/'Cargo.toml').read_text()
(scratch/'Cargo.toml').write_text(re.sub(r'members = \[[^\n]+\]', 'members = ["crates/l2-types"]',manifest,count=1))
shutil.copy2(root/'Cargo.lock',scratch/'Cargo.lock')
source=scratch/'crates/l2-types/src/lib.rs'
original=source.read_text()
account='''    verify_proof(
        state_root,
        Nibbles::unpack(keccak256(proof.address)),
        Some(alloy_rlp::encode(account)),
        &proof.account_proof,
    )
    .map_err(|_| FundingError::Account)?;'''
storage='''        verify_proof(
            proof.storage_hash,
            Nibbles::unpack(keccak256(slot.to_be_bytes::<32>())),
            expected,
            &entry.proof,
        )
        .map_err(|_| FundingError::Storage)?;'''
payment='''        || words[3] != U256::from(resource.unit_price_wei.to::<u128>()) * U256::from(ticket_count)'''
mutants=[
('account_membership',account,'    let _ = (state_root, account, &proof.account_proof);'),
('storage_membership',storage,'        let _ = (slot, expected, &entry.proof);'),
('exact_payment',payment,''),
('domain_binding','    if domain != profile.domain {','    if false {'),
('expiry','    if valid_until <= checked_at {\n        return Err(FundingError::Expired);\n    }\n    if valid_until - checked_at > u64::from(profile.config.max_ticket_lifetime_seconds) {','    if valid_until.saturating_sub(checked_at) > u64::from(profile.config.max_ticket_lifetime_seconds) {'),
('duplicate_keys','''    let proof: AccountProof =
        serde_json::from_slice(proof_json).map_err(|_| FundingError::Input)?;''','''    let value: serde_json::Value = serde_json::from_slice(proof_json).map_err(|_| FundingError::Input)?;
    let proof: AccountProof = serde_json::from_value(value).map_err(|_| FundingError::Input)?;'''),
]
env={**os.environ,'CARGO_TARGET_DIR':str(scratch/'target')}
evidence=root/'evidence/reviews'
report={'sourceSha256':hashlib.sha256(original.encode()).hexdigest(),'scratch':str(scratch),'baselinePassed':False,'mutants':[],'passed':False}
reportpath=evidence/'L01-proof-mutations.json'
def run(name):
    p=subprocess.run(['cargo','test','--offline','-p','agentic-l2','--test','funded_proof'],cwd=scratch,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=300)
    (evidence/f'L01-proof-mutation-{name}.log').write_text(p.stdout)
    return p
try:
    baseline=run('baseline')
    assert baseline.returncode==0 and '10 passed; 0 failed' in baseline.stdout,baseline.stdout[-4000:]
    report['baselinePassed']=True
    print('baseline: 10 passed',flush=True)
    for name,before,after in mutants:
        assert original.count(before)==1,name
        source.write_text(original.replace(before,after,1))
        p=run(name)
        failures=re.findall(r'^test ([^\n]+) \.\.\. FAILED$',p.stdout,re.M)
        killed=p.returncode!=0 and bool(failures) and 'test result: FAILED' in p.stdout
        report['mutants'].append({'name':name,'exitCode':p.returncode,'killed':killed,'failingTests':failures})
        print(f'{name}: {"KILLED" if killed else "INVALID OR SURVIVED"}: {failures}',flush=True)
        assert killed,p.stdout[-4000:]
    report['passed']=True
finally:
    source.write_text(original)
    reportpath.write_text(json.dumps(report,indent=2)+'\n')
