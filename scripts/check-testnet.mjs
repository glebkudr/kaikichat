// Public-testnet acceptance from isolated OrbStack containers (V1-AF07,
// V1-AF06, the double spend of V1-AF05). Clients sit behind NAT routers of
// their own and reach the Base Sepolia testnet's nodes, whose relays carry
// their circuits. A relay "disappears" only for them: their routers stop
// forwarding to it. Nothing changes on the host, its firewall or the server.
//
//   node scripts/check-testnet.mjs [--case af07|af06|double-spend|stolen-key]… [--pay] [--keep] [--attach RUN_ID] [--no-build]
//
// Without --pay the clients join, reserve relays and stop before anything
// costs money. With --pay each client buys a book of 1000 stamps ($1 in ETH
// at the Chainlink rate) from the deployer's key in .local/testnet, which is
// read here and never printed. af06 and double-spend need af07's clients.
import assert from 'node:assert/strict';
import {execFile, spawn} from 'node:child_process';
import {promisify} from 'node:util';
import {createHash, randomUUID} from 'node:crypto';
import {readFile, readdir, mkdir, writeFile} from 'node:fs/promises';
import {resolve, join, dirname} from 'node:path';
const exec = promisify(execFile);
const root = resolve(import.meta.dirname, '..');
const argv = process.argv.slice(2);
const pay = argv.includes('--pay');
// Leave a failed run's containers up, with their profiles and books.
const keep = argv.includes('--keep');
// Clients whose state goes into the report when a step fails.
const watched = [];
// --attach RUN_ID goes on with a kept run's containers (--keep): its
// newcomer's double spend, without buying its books again.
const attach = argv.flatMap((a, i) => (a === '--attach' ? [argv[i + 1]] : []))[0];
const cases = argv.flatMap((a, i) => (a === '--case' ? [argv[i + 1]] : []));
const selected = cases.length ? cases : ['af07'];
for (const name of selected) assert.ok(['af07', 'af06', 'double-spend', 'stolen-key'].includes(name), `unknown case ${name}`);
if (!attach && selected.some(name => ['af06', 'double-spend'].includes(name)) && !selected.includes('af07')) throw new Error('af06 and double-spend run with af07: its clients are their peers');
const image = 'agentic-internet-network-e2e:local';
const runId = attach ?? `ain-testnet-${randomUUID().slice(0, 8)}`;
const output = join(root, 'output/testnet', attach ? `${runId}-attach-${Date.now()}` : runId);
const deployment = JSON.parse(await readFile(join(root, 'deployments/base-sepolia.json'), 'utf8'));
// Keys and tools live in the main checkout's .local, also for a worktree.
const common = (await exec('git', ['rev-parse', '--path-format=absolute', '--git-common-dir'], {cwd: root})).stdout.trim();
const local = join(dirname(common), '.local');
const cast = join(local, 'toolchains/foundry-v1.8.1-darwin-arm64/cast');
const SERVER = '51.91.126.3';
const RESOLVERS = ['1.1.1.1', '8.8.8.8'];
const containers = [], networks = [];
const evidence = {passed: false, runId, selected, pay, outcomes: [], seconds: {}, topology: {}};
const started = Date.now();
const mark = (name, since) => {evidence.seconds[name] = (Date.now() - since) / 1000;};

const docker = async (args, input) => {
  if (input === undefined) return (await exec('docker', args, {cwd: root, timeout: 60000, maxBuffer: 20 * 1024 * 1024})).stdout.trim();
  return new Promise((resolve, reject) => {
    const child = spawn('docker', args, {cwd: root, stdio: ['pipe', 'pipe', 'pipe']});
    const out = [], errors = [];
    const timer = setTimeout(() => {child.kill('SIGKILL'); reject(new Error('Docker fixture timeout'));}, 45000);
    child.stdout.on('data', b => out.push(b));
    child.stderr.on('data', b => errors.push(b));
    child.on('error', error => {clearTimeout(timer); reject(error);});
    child.on('exit', code => {clearTimeout(timer); code === 0 ? resolve(Buffer.concat(out).toString().trim()) : reject(new Error(`Docker fixture failed (${code}): ${Buffer.concat(errors)}`));});
    child.stdin.on('error', () => {});
    child.stdin.end(JSON.stringify(input));
  });
};
// The same image and source hash as scripts/check-network.mjs.
async function sourceHash() {
  const hash = createHash('sha256');
  async function add(path) {
    for (const entry of (await readdir(join(root, path), {withFileTypes: true})).sort((a, b) => a.name.localeCompare(b.name))) {
      if (entry.name.startsWith('.') || ['target', '__pycache__'].includes(entry.name)) continue;
      const child = `${path}/${entry.name}`;
      if (entry.isDirectory()) await add(child);
      else if (entry.isFile()) hash.update(child).update(await readFile(join(root, child)));
    }
  }
  for (const name of ['Cargo.toml', 'Cargo.lock', 'apps/desktop/src-tauri/Cargo.toml', 'apps/desktop/src-tauri/build.rs', 'scripts/check-network.mjs']) hash.update(name).update(await readFile(join(root, name)));
  for (const name of ['crates', 'integrations/agent-skill', 'services', 'vendor', 'apps/desktop/src-tauri/src', 'tests/network']) await add(name);
  return hash.digest('hex');
}
async function build(hash) {
  await new Promise((resolve, reject) => {
    const child = spawn('docker', ['build', '--progress=plain', '-f', 'tests/network/Dockerfile', '--build-arg', `AIN_SOURCE_HASH=${hash}`, '-t', image, '.'], {cwd: root, stdio: 'inherit'});
    child.on('error', reject);
    child.on('exit', code => (code === 0 ? resolve() : reject(new Error(`Linux build failed (${code})`))));
  });
}
const fixture = async (container, kind, input = {}) => JSON.parse(await docker(['exec', '-i', container.id, 'python3', '/usr/local/lib/ain-network-fixture.py', kind], input));
async function freeSubnet() {
  const ids = (await docker(['network', 'ls', '-q'])).split('\n').filter(Boolean);
  const all = ids.length ? JSON.parse(await docker(['network', 'inspect', ...ids])) : [];
  const interval = cidr => {
    const [address, prefix] = cidr.split('/');
    const numeric = address.split('.').reduce((value, octet) => value * 256 + Number(octet), 0);
    const size = 2 ** (32 - Number(prefix));
    const start = Math.floor(numeric / size) * size;
    return [start, start + size - 1];
  };
  const used = all.flatMap(net => net.IPAM?.Config ?? []).map(c => c.Subnet).filter(c => c && !c.includes(':')).map(interval);
  const offset = parseInt(runId.slice(-4), 16) % 512;
  for (let step = 0; step < 512; step++) {
    const slot = (offset + step) % 512;
    const cidr = `198.${18 + Math.floor(slot / 256)}.${slot % 256}.0/24`;
    const [lo, hi] = interval(cidr);
    if (!used.some(([start, end]) => lo <= end && start <= hi)) return cidr;
  }
  throw new Error('No free benchmarking subnet');
}
async function network(name, internal = false) {
  const resourceName = `${runId}-${name}`;
  const subnet = await freeSubnet();
  networks.push(resourceName);
  const id = await docker(['network', 'create', '--subnet', subnet, '--label', `org.agentic.network-test.run=${runId}`, ...(internal ? ['--internal'] : []), resourceName]);
  const details = JSON.parse(await docker(['network', 'inspect', id]))[0];
  return {id, name: details.Name, subnet: details.IPAM.Config[0].Subnet};
}
async function address(id, net) {
  return JSON.parse(await docker(['inspect', id]))[0].NetworkSettings.Networks[net.name].IPAddress;
}
async function container(name, net, routing = false) {
  const resourceName = `${runId}-${name}`;
  containers.push(resourceName);
  const id = await docker(['run', '-d', '--init', '--name', resourceName, '--label', `org.agentic.network-test.run=${runId}`, '--network', net.id,
    ...(routing ? ['--cap-add=NET_ADMIN', '--sysctl', 'net.ipv4.ip_forward=1'] : []), evidence.imageId]);
  return {id, name, ip: await address(id, net)};
}
async function call(node, method, request = {}) {
  const result = await fixture(node, 'rpc', {method, request});
  assert.equal(result.error, undefined, `${node.name} ${method}: ${JSON.stringify(result)}`);
  return result.result;
}
const rpc = (node, method, request = {}) => fixture(node, 'rpc', {method, request});
const info = node => call(node, 'node_info');
const snapshot = node => call(node, 'snapshot');
async function until(description, fn, timeout = 120000) {
  const deadline = Date.now() + timeout; let last;
  do {last = await fn(); if (last) return last; await new Promise(r => setTimeout(r, 1000));} while (Date.now() < deadline);
  throw new Error(`Timed out: ${description}`);
}
async function start(node, args) {
  if (!watched.includes(node)) watched.push(node);
  const listen = [`/ip4/${node.ip}/udp/4001/quic-v1`, `/ip4/${node.ip}/tcp/4001`];
  const state = await fixture(node, 'start', {listen, arguments: args});
  node.peer = state.peerId;
  return state;
}
const texts = (state, id) => (state.conversations.find(c => c.id === id)?.messages ?? []).map(m => m.text);
function connectionsTo(state, peer) {return state.peerConnections.filter(c => c.peerId === peer);}
// Packets that hit every rule carrying `marker` (a block is one rule per protocol).
function countRule(rules, marker) {
  const lines = rules.split('\n').filter(line => line.includes(marker));
  assert.ok(lines.length, `missing iptables rule: ${marker}`);
  return lines.reduce((sum, line) => sum + Number(/^\[(\d+):\d+\]/.exec(line)?.[1] ?? NaN), 0);
}

// The testnet: the nodes' flags from the deployment, routes from the signed
// preset the app uses (the daemon checks its signature when it fetches it;
// here only its routes are taken).
async function testnet() {
  const response = await fetch('https://kaikichat.com/network.json');
  assert.equal(response.status, 200);
  const preset = JSON.parse((await response.json()).preset);
  assert.equal(preset.chainId, deployment.chainId);
  const routes = preset.bootstrap.map(route => ({route, port: Number(/\/(?:udp|tcp)\/(\d+)\//.exec(route)[1]), peer: route.split('/p2p/')[1]}));
  assert.equal(routes.length, 10);
  return {preset, routes, flags: deployment.nodeFlags};
}
const bootstrapArgs = routes => routes.slice(0, 4).flatMap(r => ['--bootstrap', r.route]);

// Two clients behind routers of their own. Each router drops its LAN's
// traffic to the other LAN and to the other router's public side, so the
// clients never reach each other but through a relay.
async function natPair(publicNet, prefix) {
  const lanA = await network(`${prefix}-lan-a`, true), lanB = await network(`${prefix}-lan-b`, true);
  const routerA = await container(`${prefix}-router-a`, lanA, true), routerB = await container(`${prefix}-router-b`, lanB, true);
  for (const router of [routerA, routerB]) {await docker(['network', 'connect', publicNet.id, router.id]); router.wan = await address(router.id, publicNet);}
  await fixture(routerA, 'router', {lan: routerA.ip, wan: routerA.wan, blockedSubnet: lanB.subnet});
  await fixture(routerB, 'router', {lan: routerB.ip, wan: routerB.wan, blockedSubnet: lanA.subnet});
  for (const [router, other] of [[routerA, routerB], [routerB, routerA]]) {
    await fixture(router, 'block', {lan: router.ip, host: other.wan, comment: 'ain-block-upgrade'});
  }
  const a = await container(`${prefix}-a`, lanA, true), b = await container(`${prefix}-b`, lanB, true);
  await fixture(a, 'route', {gateway: routerA.ip}); await fixture(b, 'route', {gateway: routerB.ip});
  for (const client of [a, b]) await fixture(client, 'dns', {servers: RESOLVERS});
  return {a, b, routerA, routerB};
}
async function natSingle(publicNet, prefix) {
  const lan = await network(`${prefix}-lan`, true);
  const router = await container(`${prefix}-router`, lan, true);
  await docker(['network', 'connect', publicNet.id, router.id]); router.wan = await address(router.id, publicNet);
  await fixture(router, 'router', {lan: router.ip, wan: router.wan, blockedSubnet: '203.0.113.0/24'});
  const client = await container(`${prefix}-client`, lan, true);
  await fixture(client, 'route', {gateway: router.ip});
  await fixture(client, 'dns', {servers: RESOLVERS});
  return {client, router};
}

// A book for `node` from the deployer's key: the node's own payment request,
// paid in ETH as `coins buy` hands it to an owner.
// A command given a secret: its failure never carries it into logs or the
// report (a failed exec's message repeats its whole command line).
async function secretExec(program, args, secret) {
  try {
    return await exec(program, args, {timeout: 120000});
  } catch (error) {
    const clean = text => String(text ?? '').replaceAll(secret, '<redacted>');
    throw new Error(`${program.split('/').at(-1)} failed: ${clean(error.stdout)} ${clean(error.stderr)}`.trim());
  }
}

// The node's own payment request, once it read the shop's terms from the chain.
function paymentRequest(node) {
  return until(`${node.name}'s payment request`, async () => {
    const answer = await rpc(node, 'coins_buy');
    if (answer.result?.eth) return answer.result;
    assert.ok(['chain_pending', undefined].includes(answer.error?.code), JSON.stringify(answer));
    return null;
  });
}
async function buyBook(node) {
  const key = (await readFile(join(local, 'testnet/deployer.key'), 'utf8')).trim();
  // Base Sepolia's ETH/USD feed sometimes goes longer than the shop's
  // hour without an update: the shop refuses the price (StalePrice, before
  // any gas is spent) until the feed moves again.
  const deadline = Date.now() + 45 * 60 * 1000;
  let payment, stdout, stale = 0;
  for (;;) {
    payment = await paymentRequest(node);
    try {
      ({stdout} = await secretExec(cast, ['send', payment.eth.to, payment.eth.calldata, '--value', payment.eth.value, '--private-key', key, '--rpc-url', deployment.rpc, '--json'], key));
      break;
    } catch (error) {
      if (!String(error).includes('StalePrice') || Date.now() > deadline) throw error;
      stale++;
      await new Promise(r => setTimeout(r, 60000));
    }
  }
  const since = Date.now();
  const receipt = JSON.parse(stdout);
  assert.equal(receipt.status, '0x1', 'the purchase must succeed');
  await until(`${node.name} sees its book`, async () => (await call(node, 'coins_balance')).remaining === 1000, 300000);
  return {tx: receipt.transactionHash, valueWei: payment.eth.value, quoteWei: payment.eth.quote, book: payment.book, stalePriceRetries: stale, noticedSeconds: (Date.now() - since) / 1000};
}
async function treasuryCredit() {
  const {stdout} = await exec(cast, ['call', deployment.contracts.royaltySplitter.address, 'credits(address)(uint256)', deployment.treasury, '--rpc-url', deployment.rpc]);
  return BigInt(stdout.trim().split(/\s/)[0]);
}
async function joined(node) {
  await until(`${node.name}'s swarm directory`, async () => (await info(node)).mailboxSwarm.holders === 10, 300000);
  await until(`${node.name}'s card published`, async () => typeof (await info(node)).mailboxSwarm.intro.published === 'string', 300000);
}
async function requestContact(from, to, name, op) {
  const id = (await snapshot(to)).identity.networkId;
  return until(`${from.name} finds ${to.name}'s card`, async () => {
    const answer = await rpc(from, 'request_contact', {networkId: id, name, operationId: op});
    if (answer.result) return answer.result.conversationId;
    assert.equal(answer.error?.code, 'card_pending', JSON.stringify(answer));
    return null;
  }, 180000);
}
async function sendAndRead(from, to, conversation, text, op) {
  await call(from, 'send_message', {conversationId: conversation, text, operationId: op});
  const since = Date.now();
  await until(`${to.name} reads "${text}"`, async () => texts(await snapshot(to), conversation).includes(text), 180000);
  return (Date.now() - since) / 1000;
}

async function af07(net, publicNet) {
  const {a, b, routerA, routerB} = await natPair(publicNet, 'af07');
  a.name = 'alice'; b.name = 'bob';
  // Two of the testnet's nodes as relays, one over QUIC and one over TCP.
  const [first, second] = [net.routes[2], net.routes[6]];
  const relays = [first.route, second.route.replace('/udp/', '/tcp/').replace('/quic-v1', '')];
  const args = [...net.flags, ...bootstrapArgs(net.routes), '--relay', relays[0], '--relay', relays[1]];
  let since = Date.now();
  await start(a, args); await start(b, args);
  await call(a, 'create_identity', {name: 'Alice behind NAT'}); await call(b, 'create_identity', {name: 'Bob behind NAT'});
  await until('both reserve both relays', async () => (await info(a)).relayRoutes.length === 2 && (await info(b)).relayRoutes.length === 2);
  mark('relaysReserved', since);
  assert.equal((await fixture(a, 'probe', {host: b.ip, port: 4001})).reachable, false);
  assert.equal((await fixture(a, 'probe', {host: routerB.wan, port: 4001})).reachable, false);
  evidence.topology.af07 = {alice: a.ip, bob: b.ip, routerA: routerA.wan, routerB: routerB.wan, relays};
  evidence.outcomes.push('Two clients behind separate NAT routers reserve circuits at two of the testnet\'s nodes; they cannot reach each other directly');
  since = Date.now();
  for (const node of [a, b]) await paymentRequest(node);
  mark('shopTermsRead', since);
  if (!pay) return {a, b};
  const before = await treasuryCredit();
  evidence.purchases = [await buyBook(a), await buyBook(b)];
  const after = await treasuryCredit();
  const paid = evidence.purchases.reduce((sum, p) => sum + BigInt(p.valueWei), 0n);
  const quoted = evidence.purchases.reduce((sum, p) => sum + BigInt(p.quoteWei), 0n);
  evidence.treasury = {before: before.toString(), after: after.toString(), paidWei: paid.toString(), quotedWei: quoted.toString()};
  // A tenth of what the shop kept (the price at the rate; the rest of the 1 % margin is refunded).
  const share = after - before;
  assert.ok(share * 10n <= paid && share * 100n >= quoted * 9n, `treasury share ${share} of ${paid} paid`);
  evidence.outcomes.push(`Two books bought on Base Sepolia; the treasury's credit grew by ${share} wei, a tenth of the price`);
  since = Date.now();
  await joined(a); await joined(b);
  mark('directoriesAndCards', since);
  since = Date.now();
  const conversation = await requestContact(a, b, 'Bob', 'af07-contact');
  await until('Bob has Alice', async () => (await snapshot(b)).conversations.some(c => c.id === conversation), 180000);
  mark('contactByIdThroughNat', since);
  evidence.seconds.firstMessage = await sendAndRead(a, b, conversation, 'Hi through the relay', 'af07-m1');
  evidence.seconds.reply = await sendAndRead(b, a, conversation, 'A reply through the relay', 'af07-m2');
  const circuit = await until('Alice holds a circuit to Bob', async () => {
    const found = connectionsTo(await info(a), b.peer).find(c => c.relayed);
    return found ?? null;
  }, 60000);
  const active = [first, second].find(r => circuit.remoteAddress.includes(`/p2p/${r.peer}/p2p-circuit`));
  assert.ok(active, `circuit through an unknown relay: ${circuit.remoteAddress}`);
  assert.ok(connectionsTo(await info(a), b.peer).every(c => c.relayed), 'no direct connection between the NATs');
  const survivor = active === first ? second : first;
  evidence.circuits = {first: circuit.remoteAddress};
  evidence.outcomes.push('Contact by id, a message and a reply between the NATs; their connection is a circuit through a testnet relay');
  // The active relay is gone for both networks.
  since = Date.now();
  for (const router of [routerA, routerB]) await fixture(router, 'block', {lan: router.ip, host: SERVER, port: active.port, comment: 'ain-relay-gone'});
  await until('each keeps one relay', async () => (await info(a)).relayRoutes.length === 1 && (await info(b)).relayRoutes.length === 1, 300000);
  mark('relayLost', since);
  since = Date.now();
  evidence.seconds.afterRelayLost = await sendAndRead(a, b, conversation, 'Without the first relay', 'af07-m3');
  const replaced = await until('a circuit through the other relay', async () => {
    const found = connectionsTo(await info(a), b.peer).find(c => c.relayed && c.remoteAddress.includes(`/p2p/${survivor.peer}/p2p-circuit`));
    return found ?? null;
  }, 300000);
  mark('circuitReplaced', since);
  evidence.seconds.replyAfterRelayLost = await sendAndRead(b, a, conversation, 'And a reply without it', 'af07-m4');
  evidence.circuits.replaced = replaced.remoteAddress;
  for (const router of [routerA, routerB]) {
    const rules = (await fixture(router, 'metrics')).rules;
    assert.ok(countRule(rules, 'ain-relay-gone') > 0, 'the lost relay was really cut off');
    assert.ok(countRule(rules, '-j MASQUERADE') > 0);
  }
  evidence.af07Nodes = {alice: (await info(a)).mailboxSwarm, bob: (await info(b)).mailboxSwarm};
  evidence.outcomes.push('The relay in use disappears for both NATs: each keeps the other relay, and messages go on through a circuit there');
  return {a, b, conversation};
}

async function af06(net, publicNet, peers) {
  // An independent peer: another participant's node, not in the preset, not
  // a holder, no bond. It serves the DHT and circuits; with a book of its own
  // it also knows the holders' records, which it hands a newcomer (a peer
  // without a book is shown none, so it would have none to hand on).
  const peer = await container('af06-peer', publicNet);
  peer.name = 'independent';
  await start(peer, [...net.flags, ...bootstrapArgs(net.routes), '--dht-server', '--relay-server']);
  await call(peer, 'create_identity', {name: 'Someone else'});
  const peerRoute = `/ip4/${peer.ip}/udp/4001/quic-v1/p2p/${peer.peer}`;
  // A newcomer for whom our web services are gone: its router drops HTTPS to
  // the server (kaikichat.com, its preset, id.kaikichat.com, the directory).
  // It is given only the independent peer to join through.
  const {client, router} = await natSingle(publicNet, 'af06');
  client.name = 'carol';
  for (const protocol of ['tcp', 'udp']) await fixture(router, 'block', {lan: router.ip, host: SERVER, port: 443, protocols: [protocol], comment: 'ain-web-gone'});
  let since = Date.now();
  await start(client, [...net.flags, '--bootstrap', peerRoute, '--identity-server', 'https://id.kaikichat.com']);
  await call(client, 'create_identity', {name: 'Carol, a newcomer'});
  // The node keeps asking the identity server; no login link ever comes.
  const answers = new Set();
  while (Date.now() - since < 60000) {
    const answer = await rpc(client, 'coins_claim');
    assert.equal(answer.result, undefined, `a claim without the identity server: ${JSON.stringify(answer)}`);
    answers.add(answer.error.code);
    await new Promise(r => setTimeout(r, 2000));
  }
  evidence.af06 = {claimAnswers: [...answers], balance: await call(client, 'coins_balance'), peerRoute};
  mark('claimWithoutLink', since);
  evidence.outcomes.push(`Without our web services the newcomer gets no free coins: for a minute its claim answers ${[...answers].join(', ')} and no login link`);
  if (!pay) return {client};
  evidence.purchases.push(await buyBook(peer));
  await until('the independent peer\'s swarm directory', async () => (await info(peer)).mailboxSwarm.holders === 10, 300000);
  evidence.purchases.push(await buyBook(client));
  since = Date.now();
  await joined(client);
  mark('newcomerJoined', since);
  const hints = (await info(client)).bootstrap;
  evidence.af06.bootstrap = hints;
  since = Date.now();
  const conversation = await requestContact(client, peers.a, 'Alice', 'af06-contact');
  await until('Alice has Carol', async () => (await snapshot(peers.a)).conversations.some(c => c.id === conversation), 180000);
  mark('newcomerContact', since);
  evidence.seconds.newcomerMessage = await sendAndRead(client, peers.a, conversation, 'I joined through an independent node', 'af06-m1');
  evidence.seconds.newcomerReply = await sendAndRead(peers.a, client, conversation, 'Welcome', 'af06-m2');
  const rules = (await fixture(router, 'metrics')).rules;
  assert.ok(countRule(rules, 'ain-web-gone') > 0, 'the newcomer really tried our web services');
  evidence.outcomes.push('The newcomer joins through the independent peer, buys a book, finds Alice by id and talks with her');
  return {client, conversation};
}

// A directory search with a pass of `node`'s book, from the host.
async function searchWith(node) {
  const made = await rpc(node, 'discover_pass');
  if (!made.result) return {status: 'refused by its own node', body: made.error};
  const pass = made.result.pass;
  const response = await fetch('https://directory.kaikichat.com/v1/search', {method: 'POST', headers: {'content-type': 'application/json'}, body: JSON.stringify({query: 'lobby', pass})});
  return {status: response.status, body: await response.json().catch(() => null)};
}

// A profile restored from an older backup spends the same stamps again on
// other messages: holders prove it and the book is blocked.
async function doubleSpend(newcomer, peers) {
  const {client, conversation} = newcomer;
  const before = await until('the directory takes the book', async () => {
    const answer = await searchWith(client);
    return answer.status === 200 ? answer : null;
  }, 180000);
  evidence.doubleSpendBefore = {status: before.status};
  await fixture(client, 'stop'); await fixture(client, 'backup');
  await fixture(client, 'start');
  for (let n = 0; n < 3; n++) await call(client, 'send_message', {conversationId: conversation, text: `before recovery ${n}`, operationId: `ds-a-${n}`});
  await until('the first three stored', async () => (await snapshot(client)).conversations.find(c => c.id === conversation).messages.filter(m => m.own && m.text.startsWith('before recovery') && m.delivery.phase === 'delivered').length === 3, 180000);
  await fixture(client, 'stop'); await fixture(client, 'restore');
  await fixture(client, 'start');
  const since = Date.now();
  for (let n = 0; n < 3; n++) await call(client, 'send_message', {conversationId: conversation, text: `after recovery ${n}`, operationId: `ds-b-${n}`});
  // The discovery service asks a testnet node whether the book is active.
  const book = evidence.purchases.at(-1).book;
  const blocked = await until('the directory refuses the book', async () => {
    const answer = await searchWith(client);
    return (answer.status === 401 && answer.body.error === 'book_required') || answer.body?.code === 'book_required' ? answer : null;
  }, 600000);
  evidence.doubleSpend = {book, directory: blocked, seconds: (Date.now() - since) / 1000, atAlice: await call(peers.a, 'book_status', {book: book.replace(/^0x/, '')}), client: (await info(client)).mailboxSwarm};
  evidence.outcomes.push('A profile restored from a backup spends its stamps again: the double spend is proven and the book blocked');
}

// A thief with the identity server's hot key grants books with it: the
// network takes them within the GrantIssuer's daily cap and refuses the
// first serial beyond it. The directory's node checks a search pass's grant
// with the holders' rules read from the chain. One grant per serial all day.
async function stolenKey() {
  const tool = join(root, 'target/debug/examples/grant_pass');
  const key = join(local, 'testnet/issuer.key');
  const cap = deployment.params.grantDailyCapCoins / deployment.params.grantBookSize;
  const search = async serial => {
    const seed = createHash('sha256').update(`af05-stolen-issuer-key-${serial}`).digest('hex');
    const {stdout} = await exec(tool, [key, String(serial), seed]);
    const response = await fetch('https://directory.kaikichat.com/v1/search', {method: 'POST', headers: {'content-type': 'application/json'}, body: stdout});
    return {serial, status: response.status, body: await response.json().catch(() => null), book: JSON.parse(stdout).pass.book};
  };
  const since = Date.now();
  const within = await until('the last serial within the cap is taken', async () => {
    const answer = await search(cap - 1);
    return answer.status === 200 ? answer : null;
  }, 180000);
  mark('grantWithinCapTaken', since);
  const beyond = [await search(cap), await search(cap + 1)];
  for (const answer of beyond) assert.deepEqual([answer.status, answer.body?.error], [401, 'book_required'], JSON.stringify(answer));
  evidence.stolenKey = {capSerials: cap, within: {serial: within.serial, status: within.status, cards: within.body?.cards?.length}, beyond: beyond.map(a => ({serial: a.serial, status: a.status, error: a.body?.error}))};
  evidence.outcomes.push(`A grant signed with the issuer's key is taken up to serial ${cap - 1} of the day and refused from serial ${cap}: the daily cap holds against a stolen key`);
}

await mkdir(output, {recursive: true});
try {
  evidence.docker = await docker(['info', '--format', '{{.OSType}} {{.Architecture}}']);
  evidence.sourceHash = await sourceHash();
  if (!argv.includes('--no-build')) await build(evidence.sourceHash);
  const imageInfo = JSON.parse(await docker(['image', 'inspect', image]))[0];
  assert.equal(imageInfo.Config.Labels['org.agentic.network-test.source-hash'], evidence.sourceHash, 'test image is stale; rebuild the actual source');
  evidence.imageId = imageInfo.Id;
  const net = await testnet();
  evidence.preset = {network: net.preset.network, serial: net.preset.serial};
  if (attach) {
    // The kept run's clients, as its containers are named.
    const kept = async (name, label) => {
      const details = JSON.parse(await docker(['container', 'inspect', `${runId}-${name}`]))[0];
      assert.equal(details.Config.Labels['org.agentic.network-test.run'], runId);
      const node = {id: details.Id, name: label};
      node.peer = (await info(node)).peerId;
      watched.push(node);
      return node;
    };
    const alice = await kept('af07-a', 'alice');
    const client = await kept('af06-client', 'carol');
    const aliceId = (await snapshot(alice)).identity.networkId;
    const conversation = (await snapshot(client)).conversations.find(c => c.title === 'Alice')?.id;
    assert.ok(conversation, 'the newcomer\'s conversation with Alice');
    const books = (await call(client, 'coins_balance')).books;
    evidence.purchases = [{book: books[0].book}];
    evidence.attached = {aliceId, conversation};
    if (selected.includes('double-spend')) await doubleSpend({client, conversation}, {a: alice});
  } else if (selected.includes('af07')) {
    const publicNet = await network('public');
    const peers = await af07(net, publicNet);
    let newcomer;
    if (selected.includes('af06')) newcomer = await af06(net, publicNet, peers);
    if (selected.includes('double-spend') && pay) await doubleSpend(newcomer, peers);
  }
  if (selected.includes('stolen-key')) await stolenKey();
  evidence.seconds.total = (Date.now() - started) / 1000;
  evidence.passed = true;
} catch (error) {
  evidence.error = String(error.stack ?? error);
  process.exitCode = 1;
  console.error(error);
  evidence.diagnostics = {};
  for (const node of watched) {
    evidence.diagnostics[node.name ?? node.id] = await Promise.allSettled([info(node), call(node, 'coins_balance')]).then(r => r.map(x => x.value ?? String(x.reason)));
  }
} finally {
  const cleanup = [];
  if (attach) {containers.length = 0; networks.length = 0;}
  if (keep && !evidence.passed) {
    console.error(`kept: ${containers.join(' ')}`);
    containers.length = 0; networks.length = 0;
  }
  for (const name of [...containers].reverse()) {
    try {
      const details = JSON.parse(await docker(['container', 'inspect', name]))[0];
      assert.equal(details.Config.Labels['org.agentic.network-test.run'], runId);
      await docker(['rm', '-f', details.Id]);
    } catch (error) {if (!/No such/i.test(String(error))) cleanup.push(String(error));}
  }
  for (const name of [...networks].reverse()) {
    try {
      const details = JSON.parse(await docker(['network', 'inspect', name]))[0];
      assert.equal(details.Labels['org.agentic.network-test.run'], runId);
      await docker(['network', 'rm', details.Id]);
    } catch (error) {if (!/(No such|not found)/i.test(String(error))) cleanup.push(String(error));}
  }
  evidence.cleanupErrors = cleanup;
  if (cleanup.length) {evidence.passed = false; process.exitCode = 1;}
  await writeFile(join(output, 'result.json'), JSON.stringify(evidence, null, 2) + '\n');
  console.log(JSON.stringify({passed: evidence.passed, outcomes: evidence.outcomes, seconds: evidence.seconds, evidence: output}, null, 2));
}
