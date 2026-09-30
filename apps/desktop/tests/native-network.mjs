// Real native settings controls, packaged daemon, independent relay and daemon restart.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {randomBytes} from 'node:crypto';
import {mkdir} from 'node:fs/promises';
import {createConnection} from 'node:net';
import {join} from 'node:path';
class Provider {
  constructor(binary,directory,until) {this.binary=binary;this.directory=directory;this.until=until;this.token=randomBytes(32);this.log='';}
  async start() {
    await mkdir(this.directory,{mode:0o700});
    this.socket=join(this.directory,'node.sock');
    this.child=spawn(this.binary,['serve','--profile',join(this.directory,'profile.db'),'--ipc',this.socket,'--secrets-stdin','--listen','/ip4/127.0.0.1/tcp/0','--relay-server','--autonat-server','--autonat-allow-local'],{stdio:['pipe','pipe','pipe']});
    this.child.on('error',error=>this.error=error);
    this.child.stdout.on('data',b=>this.log+=b);this.child.stderr.on('data',b=>this.log+=b);
    this.child.stdin.end(JSON.stringify({masterKey:randomBytes(32).toString('hex'),ownerToken:this.token.toString('hex')})+'\n');
    const info=await this.until('packaged relay startup',async()=>{
      if(this.error)throw this.error;
      if(this.child.exitCode!==null)throw new Error(`relay exited: ${this.log}`);
      const value=await this.info();return value.listeners?.length===1?value:null;
    });
    this.route=info.listeners[0];assert.equal(info.relayServer.enabled,true);assert.equal(info.autoNat.serverEnabled,true);
  }
  info() {
    return new Promise((resolve,reject)=>{
      const socket=createConnection(this.socket);let buffer=Buffer.alloc(0);
      socket.setTimeout(5000,()=>socket.destroy(new Error('relay IPC timed out')));
      socket.on('error',reject);
      socket.on('connect',()=>{
        const body=Buffer.from(JSON.stringify({token:this.token.toString('hex'),method:'node_info',request:{}}));
        const length=Buffer.alloc(4);length.writeUInt32BE(body.length);socket.write(Buffer.concat([length,body]));
      });
      socket.on('data',chunk=>{
        buffer=Buffer.concat([buffer,chunk]);if(buffer.length<4)return;
        const size=buffer.readUInt32BE();if(size>2097152){socket.destroy(new Error('relay response exceeds bound'));return;}
        if(buffer.length<4+size)return;
        try {const value=JSON.parse(buffer.subarray(4,4+size));assert.ok(!value.error);resolve(value.result);socket.destroy();}
        catch(error){socket.destroy(error);}
      });
      socket.on('end',()=>{if(buffer.length<4||buffer.length<4+buffer.readUInt32BE())reject(new Error('incomplete relay response'));});
    });
  }
  async stop() {
    if(this.child&&!this.error&&this.child.exitCode===null&&this.child.signalCode===null) {
      this.child.kill('SIGTERM');
      await this.until('owned relay exit',()=>this.child.exitCode!==null||this.child.signalCode!==null,5000).catch(()=>this.child.kill('SIGKILL'));
      await this.until('owned relay reaped',()=>this.child.exitCode!==null||this.child.signalCode!==null,5000);
    }
    this.token.fill(0);
  }
}
export async function nativeNetworkScenario({Client,until,nodeBinary,temporary,stopDaemon}) {
  const relay=new Provider(nodeBinary,join(temporary,'relay-provider'),until);
  try {
    await relay.start();
    const dht=await new Client('dht-owner').launch();await dht.createProfile('Discovery peer');
    const dhtIdentity=(await dht.snapshot()).identity;
    await dht.nav('settings');await until('native DHT control',()=>dht.textIncludes('Help the network find nodes'));
    assert.equal(await dht.read('return document.querySelector("#network-dht").checked;'),false);
    assert.equal((await dht.invoke('network_settings')).status.routing.mode,'client');
    await dht.click('#network-dht');await dht.button('Save and reconnect');
    await until('native DHT server live',async()=>{
      const value=await dht.invoke('network_settings');return value.revision===1&&value.preferences.dhtServer&&value.status.routing.mode==='server';
    });
    await until('native DHT server rendered',()=>dht.textIncludes('Node lookup: serving the network'));
    await dht.screenshot('dht-server');
    await dht.stop();await stopDaemon(dht);await dht.launch();
    await dht.nav('settings');await until('native restored DHT role',()=>dht.textIncludes('Node lookup: serving the network'));
    assert.equal(await dht.read('return document.querySelector("#network-dht").checked;'),true);
    const restoredRole=await dht.invoke('network_settings');assert.equal(restoredRole.revision,1);assert.equal(restoredRole.preferences.dhtServer,true);
    assert.deepEqual((await dht.snapshot()).identity,dhtIdentity);
    await dht.click('#network-dht');await dht.button('Save and reconnect');
    await until('native DHT client restored',async()=>{
      const value=await dht.invoke('network_settings');return value.revision===2&&!value.preferences.dhtServer&&value.status.routing.mode==='client';
    });
    await until('native DHT client rendered',()=>dht.textIncludes('Node lookup: queries only'));
    await dht.screenshot('dht-client');
    await dht.click('#network-dht');await dht.fill('#network-relays',relay.route);await dht.click('#network-relay-only');
    await dht.button('Save and reconnect');
    await until('native DHT role suppressed behind real relay',async()=>{
      const value=await dht.invoke('network_settings');
      return value.revision===3&&value.preferences.dhtServer&&value.status.routing.mode==='disabled'&&value.status.routing.blockedByPolicy&&value.status.relayRoutes.length===1;
    });
    await until('native DHT policy rendered',()=>dht.textIncludes('Node lookup is off in relay mode.'));
    assert.equal(await dht.read('return document.querySelector("#network-dht").checked;'),true);
    assert.equal(await dht.read('return document.querySelector(".network-form button[type=submit]").disabled;'),true);
    await dht.screenshot('dht-relay-policy');await dht.stop();await stopDaemon(dht);
    const a=await new Client('network-alice').launch();const b=await new Client('network-bob').launch();
    await a.createProfile('Alice');await b.createProfile('Bob');
    const original=(await a.snapshot()).identity;
    const originalSettings=await a.invoke('network_settings');
    // Fresh native profile enters through an owner-supplied hint before any invitation/chat.
    await a.nav('settings');await until('bootstrap form',()=>a.textIncludes('Nodes to join the network'));
    assert.equal(originalSettings.status.bootstrap.state,'bootstrap-needed');
    await until('actionable native missing-hint state',()=>a.textIncludes('Network: no verified connections'));
    await a.fill('#network-bootstrap',relay.route);await a.button('Save and reconnect');
    await until('bootstrap settings saved',()=>a.textIncludes('Settings saved.'));
    const relayPeer=(await relay.info()).peerId;
    const bootstrapped=await until('real native signed bootstrap handshake',async()=>{
      const settings=await a.invoke('network_settings');
      return settings.status.bootstrap.verifiedPeers.some(p=>p.peerId===relayPeer)?settings:null;
    });
    assert.equal(bootstrapped.revision,1);
    assert.deepEqual(bootstrapped.preferences,{relays:[],relayOnly:false,autoNatPeers:[],bootstrapPeers:[relay.route],lanDiscovery:false,dhtServer:false});
    assert.deepEqual((await a.snapshot()).conversations,[]);
    await until('reciprocal signed bootstrap admission on independent provider',async()=>
      (await relay.info()).bootstrap.verifiedPeers.some(p=>p.peerId===bootstrapped.status.peerId));
    await until('verified network status rendered',()=>a.textIncludes('Network: verified nodes — 1'));
    assert.equal(await a.read('return document.querySelector(".network-form button[type=submit]").disabled;'),true);
    await a.screenshot('bootstrap-alice-connected');await a.button('Back to chat');
    for(const client of [a,b]) {
      await client.nav('settings');await until('network form',()=>client.textIncludes('Routes'));
      await client.fill('#network-relays',relay.route);await client.click('#network-relay-only');
      // Opt-in is persisted but relay-only must suppress local multicast, including in native tests.
      await client.click('#network-lan');
      await client.fill('#network-verifiers',relay.route);await client.button('Save and reconnect');
      await until('saved settings visible',()=>client.textIncludes('Settings saved.'));
      const ready=await until('native configured relay and verified reachability',async()=>{
        const settings=await client.invoke('network_settings');
        return settings.status.relayRoutes.length===1&&settings.status.autoNat.status==='public'?settings:null;
      });
      assert.equal(ready.revision,client===a?2:1);assert.deepEqual(ready.preferences,{relays:[relay.route],relayOnly:true,autoNatPeers:[relay.route],bootstrapPeers:client===a?[relay.route]:[],lanDiscovery:true,dhtServer:false});
      assert.deepEqual(ready.status.lanDiscovery,{enabled:true,active:false,blockedByPolicy:true,peers:[]});
      assert.equal(ready.status.holePunchEnabled,false);
      assert.deepEqual(ready.status.advertisedAddresses,ready.status.relayRoutes);
      assert.ok(ready.status.relayRoutes[0].startsWith(relay.route+'/p2p-circuit/'));
      await until('actual relay status in panel',()=>client.textIncludes('Relay: 1 of 1'));
      await until('actual AutoNAT status in panel',()=>client.textIncludes('Address verified'));
      assert.equal(await client.read('return document.querySelector(".network-form button[type=submit]").disabled;'),true,'saved preferences must not appear modified');
      await client.screenshot(`${client.name}-settings`);await client.button('Back to chat');
    }
    const invitation=await b.shareInvitation();
    assert.ok(invitation.startsWith('ain-invite1:'));
    await a.addContact('Bob',invitation);
    await until('relay MLS contact accepted',()=>b.textIncludes('No messages on this device yet.'));
    const first='A link through the relay chosen in the app settings.';
    await a.fill('#message',first);await a.click('button[aria-label="Send message"]');
    await until('native relay message received',()=>b.textIncludes(first));
    await until('native relay recipient receipt',()=>a.textIncludes('Delivered'));
    const sent=(await a.snapshot()).conversations[0].messages[0];
    assert.equal(sent.text,first);assert.equal(sent.delivery.phase,'delivered');
    assert.equal((await b.snapshot()).conversations[0].messages[0].id,sent.id);
    assert.ok((await relay.info()).relayServer.acceptedCircuits>0,'independent relay must actually carry the application circuit');
    const saved=await a.invoke('network_settings');
    assert.deepEqual(saved.status.listeners,originalSettings.status.listeners);
    await a.stop();await stopDaemon(a);
    const pending='The queue survived the restart of the Alice node.';
    await b.fill('#message',pending);await b.click('button[aria-label="Send message"]');
    await until('message stays queued while recipient daemon is stopped',()=>b.read('return [...document.querySelectorAll(".message.own")].some(m=>m.querySelector("p")?.textContent===arguments[0] && m.querySelector(".delivery-queued"));',[pending]));
    const queued=(await b.snapshot()).conversations[0].messages.at(-1);assert.equal(queued.text,pending);assert.equal(queued.delivery.phase,'queued');
    await a.launch();
    await until('original queued operation reaches restarted native profile',()=>a.textIncludes(pending));
    await until('original queued operation gets its recipient receipt',async()=>(await b.snapshot()).conversations[0].messages.find(m=>m.id===queued.id)?.delivery.phase==='delivered');
    const restored=await a.invoke('network_settings');
    assert.equal(restored.revision,saved.revision);assert.deepEqual(restored.preferences,saved.preferences);
    assert.equal(restored.status.peerId,originalSettings.status.peerId);assert.deepEqual(restored.status.listeners,originalSettings.status.listeners);
    assert.deepEqual((await a.snapshot()).identity,original);
    assert.equal((await a.snapshot()).conversations[0].messages.at(-1).id,queued.id);
    assert.deepEqual((await a.snapshot()).conversations[0].messages.map(m=>m.text),[first,pending]);
    assert.deepEqual((await b.snapshot()).conversations[0].messages.map(m=>m.text),[first,pending]);
    await a.nav('settings');await until('restored relay confirmation',()=>a.textIncludes('Relay: 1 of 1'));
    assert.equal(await a.read('return document.querySelector("#network-relays").value;'),relay.route);
    assert.equal(await a.read('return document.querySelector("#network-verifiers").value;'),relay.route);
    assert.equal(await a.read('return document.querySelector("#network-relay-only").checked;'),true);
    assert.equal(await a.read('return document.querySelector(".network-form button[type=submit]").disabled;'),true,'reopened preferences must not appear modified');
    assert.equal(await a.read('return document.querySelector("#network-bootstrap").value;'),relay.route);
    assert.equal(await a.read('return document.querySelector("#network-lan").checked;'),true);
    await until('restored relay policy explains LAN suppression',()=>a.textIncludes('Local discovery is off in relay mode.'));
    await a.screenshot('network-alice-restored');
    return 'Native DHT client/server control saves and restores the actual daemon role and explains relay suppression; network settings configure an independent packaged relay/AutoNAT provider; real MLS and receipts flow through its circuit; daemon restart restores preferences, listener ports, identity and the original queued message';
  } finally {await relay.stop();}
}
