"""Isolated compiling negative controls for the reviewed bonded registry gates."""
from pathlib import Path
import hashlib,json,os,re,shutil,subprocess,tempfile
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'evidence/reviews'
source=ROOT/'contracts/src/NodeRegistry.sol'
original=source.read_text()
scratch=Path(tempfile.mkdtemp(prefix='l02-registry-mutations-',dir=ROOT.parent))
forge=str(Path(os.environ['AIN_FOUNDRY_BIN'])/'forge')
solc=os.environ['AIN_SOLC']
mutants=[
 ('free_bond',[('if (msg.value != unitBondWei) revert WrongPayment();','')],'testWrongPaymentsDuplicatesAndZeroCommitmentLeaveRegistryAndFundsUntouched'),
 ('short_exit',[('block.timestamp + snapshotLeaseSeconds + maxObligationSeconds','block.timestamp + snapshotLeaseSeconds')],'testExitRemovesFutureEligibilityButLocksPrincipalPastAllSnapshotObligations'),
 ('callback_before_state',[('        entry.state = UnitState.Withdrawn;\n',''),('        if (!paid) revert TransferFailed();','        entry.state = UnitState.Withdrawn;\n        if (!paid) revert TransferFailed();')],'testWithdrawalFailureRollsBackAndReentrantRecipientCannotWithdrawTwice'),
 ('rewrite_epoch',[('if (snapshots[epoch].count != 0) revert EpochAlreadySealed();','')],'testSealedEpochBindsRootCountAndFutureBlockDespiteLaterBondsAndExits'),
 ('seed_ignores_root',[('SEED_TAG, domain, epoch, frozen.root, frozen.count','SEED_TAG, domain, epoch, bytes32(0), frozen.count')],'testSeedUsesExactCommittedFutureBlockAndSavedRetryCannotRenewAdmission'),
 ('partial_count',[('left.count + right.count','left.count')],'testExactFixedBondAuthenticatesEveryEntryAndConservesPrincipal'),
]
report={'passed':False,'baselinePassed':False,'sourceSha256':hashlib.sha256(original.encode()).hexdigest(),'mutants':[],'cleanupErrors':[]}
def run(name,args):
 p=subprocess.run(args,cwd=scratch,env=os.environ,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=180)
 (OUT/f'L02-registry-mutation-{name}.log').write_text(p.stdout)
 return p
base=[forge,'test','--root',str(scratch/'contracts'),'--use',solc,'--match-contract','NodeRegistryTest']
try:
 shutil.copytree(ROOT/'contracts',scratch/'contracts',ignore=shutil.ignore_patterns('out','cache'))
 shutil.copytree(ROOT/'tests/evm',scratch/'tests/evm',ignore=shutil.ignore_patterns('__pycache__'))
 (scratch/'scripts').mkdir();shutil.copy2(ROOT/'scripts/check-evm.sh',scratch/'scripts/check-evm.sh')
 target=scratch/'contracts/src/NodeRegistry.sol'
 p=run('baseline',base);assert p.returncode==0 and '10 passed; 0 failed' in p.stdout,p.stdout[-4000:]
 report['baselinePassed']=True;print('baseline: 10 tests passed',flush=True)
 for name,changes,test in mutants:
  changed=original
  for before,after in changes:
   assert changed.count(before)==1,(name,before,changed.count(before));changed=changed.replace(before,after,1)
  target.write_text(changed)
  p=run(name,base+['--match-test',test])
  failed=re.findall(r'^\[FAIL[^\n]*\]\s+([^\n(]+)\(',p.stdout,re.M)
  killed=p.returncode!=0 and test in failed and 'Compiler run successful' in p.stdout
  report['mutants'].append({'name':name,'exitCode':p.returncode,'killed':killed,'failingTests':failed})
  print(name+': '+('KILLED' if killed else 'INVALID OR SURVIVED'),flush=True)
  assert killed,p.stdout[-4000:]
 # A backdoor can pass ordinary behavior tests, but must fail the exact ABI gate.
 target.write_text(original.replace('    function currentEpoch()', '    function confiscate() external { payable(msg.sender).transfer(address(this).balance); }\n\n    function currentEpoch()',1))
 p=run('admin-behavior',base);assert p.returncode==0 and '10 passed; 0 failed' in p.stdout,p.stdout[-4000:]
 p=run('admin-surface',['python3',str(scratch/'tests/evm/registry.py')])
 killed=p.returncode!=0 and 'AssertionError:' in p.stdout and "('confiscate()', 'nonpayable')" in p.stdout
 report['mutants'].append({'name':'admin_surface','behaviorTestsPassed':True,'exitCode':p.returncode,'killed':killed})
 assert killed,p.stdout[-4000:];print('admin_surface: KILLED after behavior tests passed',flush=True)
 assert source.read_text()==original,'source changed during mutation gate'
 report['passed']=True
finally:
 try:shutil.rmtree(scratch)
 except OSError as error:report['cleanupErrors'].append(str(error))
 (OUT/'L02-registry-mutations.json').write_text(json.dumps(report,indent=2)+'\n')
