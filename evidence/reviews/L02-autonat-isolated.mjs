// Diagnostic subset: original AutoNAT assertions, extra read-only status trace.
// Actual Linux network E2E in an isolated OrbStack/Docker topology. No host firewall changes.
import assert from 'node:assert/strict';
import {lanDiscoveryCase} from '../../tests/network/lan-discovery.mjs';
import {execFile, spawn} from 'node:child_process';
import {promisify} from 'node:util';
import {createHash, randomUUID} from 'node:crypto';
import {readFile, readdir, mkdir, writeFile} from 'node:fs/promises';
import {resolve, join} from 'node:path';
const exec = promisify(execFile);
const root = resolve(import.meta.dirname, '../..');
const output = join(root, 'evidence/reviews/L02-autonat-isolated', randomUUID());
const image = 'agentic-internet-network-e2e:local';
const runId = `ain-nat-${randomUUID().slice(0, 8)}`;
const containers = [], networks = [];
const evidence = {passed: false, runId, outcomes: [], topology: {}};
let verifiedImage, interruption, cleaning = false, runningBuild;
const checkStop = () => {if (interruption && !cleaning) throw interruption;};
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => {
  interruption ??= new Error(`Network verifier interrupted by ${signal}`);
  evidence.passed = false;
  evidence.error = String(interruption);
  process.exitCode = 130;
  runningBuild?.kill('SIGTERM');
  // Finish the current bounded Docker operation so its created resource can be recorded.
  // Every subsequent operation checks interruption and enters the common cleanup path.
});
const docker = async (args, input) => {
  checkStop();
  if (input === undefined) return (await exec('docker', args, {cwd: root, timeout: 60000, maxBuffer: 20 * 1024 * 1024})).stdout.trim();
  return new Promise((resolve, reject) => {
    const child = spawn('docker', args, {cwd: root, stdio: ['pipe', 'pipe', 'pipe']});
    const chunks = [], errors = []; let size = 0;
    const timer = setTimeout(() => {child.kill('SIGKILL'); reject(new Error('Docker fixture timeout'));}, 45000);
    child.stdout.on('data', b => {size += b.length; if (size > 20 * 1024 * 1024) {child.kill('SIGKILL'); reject(new Error('Unbounded fixture output'));} else chunks.push(b);});
    child.stderr.on('data', b => errors.push(b));
    child.on('error', error => {clearTimeout(timer); reject(error);});
    child.on('exit', code => {clearTimeout(timer); code === 0 ? resolve(Buffer.concat(chunks).toString().trim()) : reject(new Error(`Docker fixture failed (${code}): ${Buffer.concat(errors).toString()}`));});
    child.stdin.on('error', () => {});
    child.stdin.end(JSON.stringify(input));
  });
};
async function sourceHash() {
  const hash = createHash('sha256');
  async function add(path) {
    const entries = await readdir(join(root, path), {withFileTypes: true});
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      if (entry.name.startsWith('.') || ['target', '__pycache__'].includes(entry.name)) continue;
      const child = `${path}/${entry.name}`;
      if (entry.isDirectory()) await add(child);
      else if (entry.isFile()) hash.update(child).update(await readFile(join(root, child)));
    }
  }
  for (const name of ['Cargo.toml', 'Cargo.lock', 'apps/desktop/src-tauri/Cargo.toml', 'apps/desktop/src-tauri/build.rs', 'scripts/check-network.mjs']) hash.update(name).update(await readFile(join(root, name)));
  for (const name of ['crates', 'apps/desktop/src-tauri/src', 'tests/network']) await add(name);
  return hash.digest('hex');
}
async function build(hash) {
  checkStop();
  await new Promise((resolve, reject) => {
    const child = spawn('docker', ['build', '--progress=plain', '-f', 'tests/network/Dockerfile', '--build-arg', `AIN_SOURCE_HASH=${hash}`, '-t', image, '.'], {cwd: root, stdio: 'inherit'});
    runningBuild = child;
    const timer = setTimeout(() => {child.kill('SIGKILL'); reject(new Error('Linux image build timeout'));}, 20 * 60 * 1000);
    child.on('error', error => {clearTimeout(timer); runningBuild = undefined; reject(error);});
    child.on('exit', code => {clearTimeout(timer); runningBuild = undefined; code === 0 ? resolve() : reject(new Error(`Linux build failed (${code})`));});
  });
}
const fixture = async (container, kind, input = {}) => JSON.parse(await docker(['exec', '-i', container.id, 'python3', '/usr/local/lib/ain-network-fixture.py', kind], input));
async function freeSubnet() {
  const ids = (await docker(['network', 'ls', '-q'])).split('\n').filter(Boolean);
  const all = ids.length ? JSON.parse(await docker(['network', 'inspect', ...ids])) : [];
  function interval(cidr) {
    const [address, prefix] = cidr.split('/');
    const numeric = address.split('.').reduce((value, octet) => value * 256 + Number(octet), 0);
    const size = 2 ** (32 - Number(prefix));
    const start = Math.floor(numeric / size) * size;
    return [start, start + size - 1];
  }
  const used = all.flatMap(net => net.IPAM?.Config ?? []).map(c => c.Subnet).filter(c => c && !c.includes(':')).map(interval);
  // IANA198.18/15 is non-global benchmarking space; never change the daemon's shared address pools.
  const offset = parseInt(runId.slice(-4), 16) % 512;
  for (let step = 0; step < 512; step++) {
    const slot = (offset + step) % 512;
    const cidr = `198.${18 + Math.floor(slot / 256)}.${slot % 256}.0/24`;
    const [lo, hi] = interval(cidr);
    if (!used.some(([start, end]) => lo <= end && start <= hi)) return cidr;
  }
  throw new Error('No free isolated benchmarking subnet; existing networks remain untouched');
}
async function network(name, internal = false) {
  const resourceName = `${runId}-${name}`;
  const subnet = await freeSubnet();
  networks.push(resourceName);
  const id = await docker(['network', 'create', '--subnet', subnet, '--label', `org.agentic.network-test.run=${runId}`, ...(internal ? ['--internal'] : []), resourceName]);
  const details = JSON.parse(await docker(['network', 'inspect', id]))[0];
  return {id, name: details.Name, subnet: details.IPAM.Config[0].Subnet};
}
async function container(name, net, routing = false) {
  const resourceName = `${runId}-${name}`;
  containers.push(resourceName);
  const id = await docker(['run', '-d', '--init', '--name', resourceName, '--label', `org.agentic.network-test.run=${runId}`, '--network', net.id,
    ...(routing ? ['--cap-add=NET_ADMIN', '--sysctl', 'net.ipv4.ip_forward=1'] : []), verifiedImage]);
  return {id, name, ip: await address(id, net)};
}
async function address(id, net) {
  const details = JSON.parse(await docker(['inspect', id]))[0];
  return details.NetworkSettings.Networks[net.name].IPAddress;
}
async function call(node, method, request = {}) {
  const result = await fixture(node, 'rpc', {method, request});
  assert.equal(result.error, undefined, `Owner command ${method}: ${JSON.stringify(result)}`);
  return result.result;
}
const info = async node => {
  const state = await call(node, 'node_info');
  if (node.name?.startsWith('autonat-')) {
    evidence.statusTrace ??= [];
    if (evidence.statusTrace.length < 1600) evidence.statusTrace.push({at: Date.now(), name: node.name, nat: state.autoNat, advertisedAddresses: state.advertisedAddresses});
  }
  return state;
};
const snapshot = node => call(node, 'snapshot');
async function until(description, fn, timeout = 35000) {
  const deadline = Date.now() + timeout; let last;
  do {last = await fn(); if (last) return last; await new Promise(r => setTimeout(r, 100));} while (Date.now() < deadline);
  throw new Error(`Timed out: ${description}`);
}
async function start(node, arguments_ = [], transports = ['tcp', 'quic']) {
  const listen = transports.map(t => `/ip4/${node.ip}/${t === 'tcp' ? 'tcp/4001' : 'udp/4001/quic-v1'}`);
  const state = await fixture(node, 'start', {listen, arguments: arguments_});
  node.peer = state.peerId;
  return state;
}
const rows = (state, group) => state.conversations.find(c => c.id === group)?.messages ?? [];
async function connect(a, b) {
  await call(a, 'create_identity', {name: 'Alice'});
  await call(b, 'create_identity', {name: 'Bob'});
  const invitation = await call(b, 'create_invitation');
  const conversation = await call(a, 'add_contact', {name: 'Bob', invitation});
  await until('MLS Welcome reaches recipient', async () => (await snapshot(b)).conversations.some(c => c.id === conversation.id));
  return conversation.id;
}
async function delivered(a, b, group, message, expectedCount) {
  await until('recipient exact original message', async () => rows(await snapshot(b), group).some(m => m.id === message.id && m.text === message.text));
  await until('signed recipient receipt', async () => rows(await snapshot(a), group).some(m => m.id === message.id && m.delivery.phase === 'delivered'));
  assert.equal(rows(await snapshot(a), group).length, expectedCount);
  assert.equal(rows(await snapshot(b), group).length, expectedCount);
  assert.equal((await info(a)).pendingOutbox, 0);
}
async function send(a, b, group, text, operationId, count) {
  const message = await call(a, 'send_message', {conversationId: group, text, operationId});
  assert.equal(message.text, text, 'send response must preserve the exact requested text');
  try {await delivered(a, b, group, message, count);}
  catch (error) {
    evidence.deliveryFailure = {operationId, diagnostics: await Promise.allSettled([info(a), info(b), snapshot(a), snapshot(b)])};
    throw error;
  }
  return message;
}
async function reserved(node, count) {
  return until(`${node.name} has ${count} confirmed relay routes`, async () => {const v = await info(node); return v.relayRoutes.length === count && v;});
}
function applicationConnection(state, peer) {
  const matches = state.peerConnections.filter(c => c.peerId === peer);
  assert.equal(matches.length, 1, 'active application connection must be unambiguous');
  return matches[0];
}
function countRule(rules, marker) {
  const line = rules.split('\n').find(line => line.includes(marker));
  assert.ok(line, `missing actual iptables rule: ${marker}`);
  return Number(/^\[(\d+):\d+\]/.exec(line)?.[1] ?? NaN);
}
async function directCase(publicNet) {
  const a = await container('direct-a', publicNet), b = await container('direct-b', publicNet);
  await start(a); await start(b);
  assert.equal((await fixture(a, 'probe', {host: b.ip, port: 4001})).reachable, true);
  const group = await connect(a, b);
  await send(a, b, group, 'Direct Linux delivery', 'direct-send', 1);
  await send(b, a, group, 'Direct Linux reply', 'direct-reply', 2);
  assert.equal(applicationConnection(await info(a), b.peer).relayed, false);
  evidence.outcomes.push('Direct Linux peers exchange MLS messages and signed receipts over an actual network');
  await fixture(a, 'stop'); await fixture(b, 'stop');
}
async function privatePair(publicNet, prefix, blockUpgrade = false) {
  const lanA = await network(`${prefix}-private-a`, true), lanB = await network(`${prefix}-private-b`, true);
  const routerA = await container(`${prefix}-router-a`, lanA, true), routerB = await container(`${prefix}-router-b`, lanB, true);
  for (const router of [routerA, routerB]) {await docker(['network', 'connect', publicNet.id, router.id]); router.wan = await address(router.id, publicNet);}
  await fixture(routerA, 'router', {lan: routerA.ip, wan: routerA.wan, blockedSubnet: lanB.subnet, blockedPublic: blockUpgrade ? routerB.wan : undefined});
  await fixture(routerB, 'router', {lan: routerB.ip, wan: routerB.wan, blockedSubnet: lanA.subnet, blockedPublic: blockUpgrade ? routerA.wan : undefined});
  const a = await container(`${prefix}-a`, lanA, true), b = await container(`${prefix}-b`, lanB, true);
  await fixture(a, 'route', {gateway: routerA.ip}); await fixture(b, 'route', {gateway: routerB.ip});
  return {a, b, routerA, routerB};
}
async function natCase(publicNet) {
  const {a, b, routerA, routerB} = await privatePair(publicNet, 'nat');
  const r1 = await container('relay-tcp', publicNet), r2 = await container('relay-quic', publicNet);
  const s1 = await start(r1, ['--relay-server'], ['tcp']), s2 = await start(r2, ['--relay-server'], ['quic']);
  const relayArguments = ['--relay-only', '--relay', s1.listeners[0], '--relay', s2.listeners[0]];
  await start(a, relayArguments); await start(b, relayArguments);
  await reserved(a, 2); await reserved(b, 2);
  // Negative control: both daemons listen on TCP4001, but actual router policy drops direct traffic.
  assert.equal((await fixture(a, 'probe', {host: b.ip, port: 4001})).reachable, false);
  assert.equal((await fixture(b, 'probe', {host: a.ip, port: 4001})).reachable, false);
  const metricsA = await fixture(routerA, 'metrics'), metricsB = await fixture(routerB, 'metrics');
  evidence.networkProof = {routerA: metricsA, routerB: metricsB};
  for (const metrics of [metricsA, metricsB]) {
    assert.ok(countRule(metrics.rules, 'ain-block-direct') > 0, 'direct block must actually see packets');
    assert.ok(countRule(metrics.rules, '-j MASQUERADE') > 0, 'relay connections must cross real source NAT');
  }
  // Relay endpoint observations independently prove translation to each router's public address.
  for (const relay of [r1, r2]) {
    const state = await info(relay);
    for (const [node, router] of [[a, routerA], [b, routerB]]) {
      const connection = applicationConnection(state, node.peer);
      assert.equal(connection.relayed, false);
      assert.ok(connection.remoteAddress.startsWith(`/ip4/${router.wan}/`), `relay did not observe translated source: ${connection.remoteAddress}`);
    }
  }
  const group = await connect(a, b);
  const first = await send(a, b, group, 'Encrypted behind different NATs', 'nat-first', 1);
  await send(b, a, group, 'Reply behind different NATs', 'nat-reply', 2);
  const route = applicationConnection(await info(a), b.peer);
  assert.equal(route.relayed, true);
  const activePeer = /\/p2p\/([^/]+)\/p2p-circuit/.exec(route.remoteAddress)?.[1];
  assert.ok(activePeer === r1.peer || activePeer === r2.peer);
  const failed = activePeer === r1.peer ? r1 : r2, survivor = failed === r1 ? r2 : r1;
  await fixture(failed, 'stop');
  await reserved(a, 1); await reserved(b, 1);
  const afterFailure = await send(a, b, group, 'Replacement relay behind NAT', 'nat-failover', 3);
  assert.ok(applicationConnection(await info(a), b.peer).remoteAddress.includes(`/p2p/${survivor.peer}/p2p-circuit/`));
  await fixture(survivor, 'stop');
  await reserved(a, 0); await reserved(b, 0);
  const originalPeer = a.peer;
  const queuedText = 'Autonomous NAT recovery after restart';
  const queued = await call(a, 'send_message', {conversationId: group, text: queuedText, operationId: 'nat-restart'});
  assert.equal(queued.text, queuedText);
  assert.equal(queued.delivery.phase, 'queued');
  await fixture(a, 'stop');
  const reopened = await fixture(a, 'start');
  assert.equal(reopened.peerId, originalPeer);
  assert.equal(reopened.pendingOutbox, 1);
  assert.equal(rows(await snapshot(a), group).at(-1).id, queued.id);
  await fixture(survivor, 'start');
  await reserved(a, 1); await reserved(b, 1);
  // No repeated send before observing autonomous delivery of the original durable operation.
  await delivered(a, b, group, queued, 4);
  const retry = await call(a, 'send_message', {conversationId: group, text: queuedText, operationId: 'nat-restart'});
  assert.equal(retry.id, queued.id);
  assert.equal(rows(await snapshot(b), group).length, 4);
  assert.deepEqual((await snapshot(survivor)).conversations, []);
  evidence.topology = {clientA: a.ip, clientB: b.ip, routerA: routerA.wan, routerB: routerB.wan, relayTCP: r1.ip, relayQUIC: r2.ip};
  evidence.networkProof = {routerA: metricsA, routerB: metricsB, initialRoute: route, survivingRelay: survivor.peer};
  evidence.messageIds = [first.id, afterFailure.id, queued.id];
  evidence.outcomes.push('Different private networks: direct probes are dropped, relay sees NAT-translated sources, MLS messages and receipts arrive');
  evidence.outcomes.push('Actual active TCP/QUIC relay stops; independent relay replaces it; sender restart and provider recovery drain the original outbox without another send');
}
async function holePunchCase(publicNet, blockUpgrade) {
  const prefix = blockUpgrade ? 'blocked-upgrade' : 'hole-punch';
  const {a, b, routerA, routerB} = await privatePair(publicNet, prefix, blockUpgrade);
  const relay = await container(`${prefix}-relay`, publicNet);
  const relayState = await start(relay, ['--relay-server'], ['quic']);
  const args = ['--relay', relayState.listeners[0]];
  await start(a, args, ['quic']); await start(b, args, ['quic']);
  await reserved(a, 1); await reserved(b, 1);
  // The independent provider cannot reach a client's public mapping unsolicited.
  // The clients cannot bypass the NAT routers using their private listener addresses either.
  for (const router of [routerA, routerB]) await fixture(relay, 'udp_probe', {host: router.wan, port: 4001});
  await fixture(a, 'udp_probe', {host: b.ip, port: 4001});
  await fixture(b, 'udp_probe', {host: a.ip, port: 4001});
  for (const [node, router] of [[a, routerA], [b, routerB]]) {
    const metrics = await fixture(router, 'metrics');
    assert.ok(countRule(metrics.rules, 'ain-unsolicited-udp') > 0, 'unsolicited public probe must hit router INPUT drop');
    assert.ok(countRule(metrics.rules, 'ain-block-direct') > 0, 'private address bypass must be blocked');
    assert.ok(countRule(metrics.rules, '-j MASQUERADE') > 0);
    assert.ok(applicationConnection(await info(relay), node.peer).remoteAddress.startsWith(`/ip4/${router.wan}/`));
  }
  const group = await connect(a, b);
  await send(a, b, group, `${prefix}: original encrypted request`, `${prefix}-request`, 1);
  if (blockUpgrade) {
    // The protocol must actually try and fail, while preserving the useful relay connection.
    await until('hole-punch attempts fail with peer-to-peer UDP blocked', async () => {
      const states = await Promise.all([info(a), info(b)]);
      return states.some(s => s.holePunch?.failed > 0);
    });
    for (const [node, peer] of [[a, b], [b, a]]) {
      const state = await info(node);
      assert.equal(state.holePunch.enabled, true);
      assert.equal(state.holePunch.succeeded, 0);
      assert.equal(applicationConnection(state, peer.peer).relayed, true);
    }
    assert.ok(countRule((await fixture(routerA, 'metrics')).rules, 'ain-block-upgrade') + countRule((await fixture(routerB, 'metrics')).rules, 'ain-block-upgrade') > 0);
    await send(b, a, group, 'Relay reply after failed hole punch', `${prefix}-reply`, 2);
    evidence.blockedUpgrade = {nodes: [await info(a), await info(b)], routerA: await fixture(routerA, 'metrics'), routerB: await fixture(routerB, 'metrics')};
    evidence.outcomes.push('Blocked QUIC hole punching reports failure while encrypted relay delivery and signed receipts keep working');
  } else {
    const states = await until('actual DCUtR upgrade to direct QUIC', async () => {
      const states = await Promise.all([info(a), info(b)]);
      return states.every((s, i) => s.peerConnections.some(c => c.peerId === [b, a][i].peer && !c.relayed)) && states;
    });
    assert.ok(states.some(s => s.holePunch?.succeeded > 0), 'direct connection must have an actual DCUtR success event');
    for (const [index, peerRouter] of [[0, routerB], [1, routerA]]) {
      const direct = states[index].peerConnections.filter(c => c.peerId === [b, a][index].peer && !c.relayed);
      assert.equal(direct.length, 1);
      assert.ok(direct[0].remoteAddress.startsWith(`/ip4/${peerRouter.wan}/udp/4001/quic-v1`), 'upgrade must use the peer NAT public mapping');
    }
    await fixture(relay, 'stop');
    await reserved(a, 0); await reserved(b, 0);
    await until('both peers retain only their direct connection after relay death', async () => {
      const states = await Promise.all([info(a), info(b)]);
      return states.every((s, i) => s.peerConnections.length === 1 && s.peerConnections[0].peerId === [b, a][i].peer && !s.peerConnections[0].relayed);
    });
    await send(b, a, group, 'Direct QUIC reply after relay shutdown', `${prefix}-reply`, 2);
    await send(a, b, group, 'Direct QUIC continues without relay', `${prefix}-followup`, 3);
    evidence.holePunch = {beforeRelayDeath: states, afterRelayDeath: [await info(a), await info(b)], routerA: await fixture(routerA, 'metrics'), routerB: await fixture(routerB, 'metrics')};
    evidence.outcomes.push('DCUtR upgrades actual NAT mappings to direct QUIC; killing the only relay preserves bidirectional MLS delivery and signed receipts');
  }
  await fixture(a, 'stop'); await fixture(b, 'stop');
  if (blockUpgrade) await fixture(relay, 'stop');
}
async function invitationRoutes(node) {
  const invitation = await call(node, 'create_invitation');
  return fixture(node, 'inspect_invitation', {invitation});
}
async function noRoutableInvitation(node) {
  const response = await fixture(node, 'rpc', {method: 'create_invitation'});
  assert.equal(response.result, undefined);
  assert.equal(response.error?.code, 'network_unavailable', 'no verified route must produce a specific actionable refusal');
}
async function autoNatCase(publicNet) {
  const service = await container('autonat-service', publicNet);
  const publicClient = await container('autonat-public', publicNet, true);
  const observer = await container('autonat-observer', publicNet);
  const serviceState = await start(service, ['--autonat-server', '--autonat-allow-local'], ['tcp']);
  const probeArgs = ['--autonat-peer', serviceState.listeners[0], '--autonat-probe-seconds', '10'];
  await start(publicClient, probeArgs, ['tcp']); await start(observer, [], ['tcp']);
  const publicRoute = `/ip4/${publicClient.ip}/tcp/4001/p2p/${publicClient.peer}`;
  const reachable = await until('AutoNAT verifies fresh external TCP dial back', async () => {
    const s = await info(publicClient);
    return s.autoNat?.status === 'public' && s.autoNat.publicAddress === publicRoute && s;
  });
  assert.ok(reachable.autoNat.successfulProbes > 0);
  assert.ok(reachable.advertisedAddresses.includes(publicRoute));
  const group = await connect(observer, publicClient);
  assert.deepEqual(await invitationRoutes(publicClient), [publicRoute]);
  await send(observer, publicClient, group, 'Fresh peer uses AutoNAT-confirmed invitation', 'autonat-public', 1);
  // Existing authenticated TCP connections keep working, but fresh inbound connections are denied.
  await fixture(publicClient, 'reachability_firewall', {blocked: true});
  const privateState = await until('previously public address withdrawn after firewall change', async () => {
    const s = await info(publicClient);
    return s.autoNat?.status === 'private' && s;
  });
  assert.equal(privateState.autoNat.publicAddress, null);
  assert.ok(!privateState.advertisedAddresses.includes(publicRoute));
  await noRoutableInvitation(publicClient);
  const blockedMetrics = await fixture(publicClient, 'metrics');
  assert.ok(countRule(blockedMetrics.rules, 'ain-autonat-new-inbound') > 0, 'a fresh AutoNAT dial must actually hit the firewall');
  await send(publicClient, observer, group, 'Existing chat survives failed external reachability probe', 'autonat-existing', 2);
  await fixture(publicClient, 'reachability_firewall', {blocked: false});
  const restored = await until('AutoNAT rechecks and restores the route after firewall recovery', async () => {
    const s = await info(publicClient);
    return s.autoNat?.status === 'public' && s.advertisedAddresses.includes(publicRoute) && s;
  });
  assert.ok(restored.autoNat.successfulProbes > reachable.autoNat.successfulProbes);
  assert.deepEqual(await invitationRoutes(publicClient), [publicRoute]);
  await fixture(service, 'stop');
  const unavailable = await until('unavailable probe service does not keep stale public success', async () => {
    const s = await info(publicClient);
    return s.autoNat?.status === 'unknown' && s.autoNat.publicAddress === null && s;
  });
  assert.ok(!unavailable.advertisedAddresses.includes(publicRoute));
  await noRoutableInvitation(publicClient);
  evidence.autoNatPublic = {reachable, privateState, restored, unavailable, blockedMetrics};
  await fixture(publicClient, 'stop'); await fixture(observer, 'stop');

  const {a, b, routerA, routerB} = await privatePair(publicNet, 'autonat');
  const relay = await container('autonat-relay', publicNet);
  const relayState = await start(relay, ['--relay-server', '--autonat-server', '--autonat-allow-local'], ['tcp']);
  const args = ['--relay', relayState.listeners[0], '--autonat-peer', relayState.listeners[0], '--autonat-probe-seconds', '10'];
  await start(a, args, ['tcp']); await start(b, args, ['tcp']);
  await reserved(a, 1); await reserved(b, 1);
  const privateClients = await until('two closed NAT clients remain private despite live relay connections', async () => {
    const states = await Promise.all([info(a), info(b)]);
    return states.every(s => s.autoNat?.status === 'private' && s.autoNat.publicAddress === null) && states;
  });
  for (const state of privateClients) {
    assert.deepEqual(state.advertisedAddresses, state.relayRoutes, 'only confirmed relay routes may be published while direct reachability is disproven');
  }
  const natGroup = await connect(a, b);
  assert.deepEqual(await invitationRoutes(a), (await info(a)).relayRoutes);
  assert.deepEqual(await invitationRoutes(b), (await info(b)).relayRoutes);
  await send(a, b, natGroup, 'AutoNAT-private peers deliver using relay', 'autonat-nat', 1);
  await send(b, a, natGroup, 'AutoNAT-private relay reply', 'autonat-nat-reply', 2);
  evidence.autoNatPrivate = {nodes: [await info(a), await info(b)], service: await info(relay), routerA: await fixture(routerA, 'metrics'), routerB: await fixture(routerB, 'metrics')};
  assert.ok(evidence.autoNatPrivate.service.autoNat.inboundProbes > 0);
  // Service opt-in is independent from ordinary messaging. A default daemon must never dial back.
  const beforeRestart = [await info(a), await info(b)];
  await fixture(relay, 'stop');
  await fixture(relay, 'start', {listen: [`/ip4/${relay.ip}/tcp/4001`], arguments: ['--relay-server']});
  await reserved(a, 1); await reserved(b, 1);
  const disabled = await until('ordinary relay refuses reachability service', async () => {
    const states = await Promise.all([info(a), info(b)]);
    return states.every((s, i) => s.autoNat?.status === 'unknown' && s.autoNat.refusedProbes > beforeRestart[i].autoNat.refusedProbes) && states;
  });
  const disabledService = await info(relay);
  assert.equal(disabledService.autoNat.serverEnabled, false);
  assert.equal(disabledService.autoNat.inboundProbes, 0);
  assert.ok(disabledService.autoNat.deniedProbes >= 2);
  assert.deepEqual(await invitationRoutes(a), (await info(a)).relayRoutes);
  assert.deepEqual(await invitationRoutes(b), (await info(b)).relayRoutes);
  await send(a, b, natGroup, 'Relay chat still works with AutoNAT service disabled', 'autonat-service-disabled', 3);
  evidence.autoNatDisabled = {nodes: disabled, service: disabledService};
  for (const node of [a, b, relay]) await fixture(node, 'stop');
  // A public client holds a relay reservation and also receives a fresh callback from that same peer.
  await fixture(relay, 'start', {listen: [`/ip4/${relay.ip}/tcp/4001`], arguments: ['--relay-server', '--autonat-server', '--autonat-allow-local']});
  await start(publicClient, args, ['tcp']); await fixture(observer, 'start');
  await reserved(publicClient, 1);
  const combined = await until('combined service verifies public client without losing its relay reservation', async () => {
    const s = await info(publicClient);
    return s.autoNat?.status === 'public' && s.autoNat.publicAddress === publicRoute && s.relayRoutes.length === 1 && s;
  });
  assert.deepEqual(await invitationRoutes(publicClient), [...combined.relayRoutes, publicRoute]);
  await send(observer, publicClient, group, 'Public client uses combined relay and AutoNAT provider after restart', 'autonat-combined', 3);
  const combinedService = await info(relay);
  assert.equal(combinedService.relayServer.activeReservations, 1, 'closing an AutoNAT callback must preserve the live relay reservation and capacity accounting');
  assert.deepEqual(combinedService.relayServer.reservationPeers.map(p => p.peerId), [publicClient.peer]);
  evidence.autoNatCombined = {client: combined, service: combinedService};
  for (const node of [publicClient, observer, relay]) await fixture(node, 'stop');
  evidence.outcomes.push('AutoNAT verifies and withdraws/restores actual public routes across firewall and service changes; private NAT peers retain working relay delivery and server opt-in is enforced');
}
await mkdir(output, {recursive: true});
const saveReport = () => writeFile(join(output, 'result.json'), JSON.stringify({...evidence, resources: {containers, networks}}, null, 2) + '\n');
if (!process.argv.includes('--build-only')) await saveReport(); // Invalidate any preceding success before work starts.
try {
  evidence.docker = await docker(['info', '--format', '{{.OSType}} {{.Architecture}}']);
  assert.match(evidence.docker, /^linux /);
  evidence.sourceHash = await sourceHash();
  if (!process.argv.includes('--no-build')) await build(evidence.sourceHash);
  checkStop();
  assert.equal(await sourceHash(), evidence.sourceHash, 'source changed during Linux build; rerun on stable inputs');
  const imageInfo = JSON.parse(await docker(['image', 'inspect', image]))[0];
  assert.equal(imageInfo.Config.Labels['org.agentic.network-test.source-hash'], evidence.sourceHash, 'test image is stale; rebuild the actual source');
  evidence.imageId = imageInfo.Id;
  assert.match(evidence.imageId, /^sha256:[a-f0-9]{64}$/);
  verifiedImage = evidence.imageId;
  if (process.argv.includes('--build-only')) {console.log('Linux network fixture built from current source.');}
  else {

    const publicNet = await network('providers');

    await autoNatCase(publicNet);
    checkStop();
    assert.equal(await sourceHash(), evidence.sourceHash, 'source changed during E2E; result does not verify current sources');
    evidence.passed = true;
  }
} catch (error) {
  evidence.error = String(error);
  process.exitCode = 1;
  console.error(error);
} finally {
  cleaning = true;
  evidence.diagnostics = {};
  for (const name of containers.filter(n => /-autonat-(public|service)$/.test(n))) {
    evidence.diagnostics[name] = await Promise.allSettled([
      fixture({id:name}, 'rpc', {method:'node_info'}),
      fixture({id:name}, 'metrics'),
      docker(['exec',name,'cat','/tmp/ain-network-profile/daemon.log'])
    ]);
  }
  const cleanup = [];
  async function remove(kind, name) {
    let details;
    try {details = JSON.parse(await docker([kind, 'inspect', name]))[0];}
    catch (error) {if (/(No such (object|container|network)|network .* not found)/i.test(String(error))) return; throw error;}
    const labels = kind === 'container' ? details.Config.Labels : details.Labels;
    assert.equal(labels['org.agentic.network-test.run'], runId, 'cleanup ownership mismatch');
    await docker(kind === 'container' ? ['rm', '-f', details.Id] : ['network', 'rm', details.Id]);
  }
  for (const name of [...containers].reverse()) try {await remove('container', name);} catch (e) {cleanup.push(String(e));}
  for (const name of [...networks].reverse()) try {await remove('network', name);} catch (e) {cleanup.push(String(e));}
  evidence.cleanupErrors = cleanup;
  if (cleanup.length) {evidence.passed = false; process.exitCode = 1;}
  if (interruption) {evidence.passed = false; evidence.error = String(interruption);}
  if (!process.argv.includes('--build-only')) {
    await saveReport();
    console.log(JSON.stringify({passed: evidence.passed, outcomes: evidence.outcomes, cleanupErrors: cleanup, evidence: output}, null, 2));
  }
}
