// Independent newline JSON-RPC client for the packaged product E2E, no owner API.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {setTimeout as delay} from 'node:timers/promises';

export class McpClient {
  constructor(config) {
    this.pending=new Map();this.nextId=1;this.buffer='';this.log='';
    this.child=spawn(config.command,config.args,{stdio:['pipe','pipe','pipe']});
    this.child.on('error',error=>{this.launchError=error;this.fail(error);});
    this.child.on('exit',(code,signal)=>this.fail(new Error(`MCP exited ${code}/${signal}: ${this.log}`)));
    this.child.stdin.on('error',error=>this.fail(error));
    this.child.stderr.on('data',chunk=>{this.log=(this.log+chunk).slice(-8192);});
    this.child.stdout.setEncoding('utf8');
    this.child.stdout.on('data',chunk=>{
      this.buffer+=chunk;
      if(Buffer.byteLength(this.buffer)>1024*1024)return this.fail(new Error('MCP output exceeds bound'));
      let index;
      while((index=this.buffer.indexOf('\n'))!==-1) {
        const line=this.buffer.slice(0,index);this.buffer=this.buffer.slice(index+1);
        try {
          const value=JSON.parse(line);
          if(value.id===undefined)continue;
          const pending=this.pending.get(value.id);assert.ok(pending,'response to unknown request');
          this.pending.delete(value.id);clearTimeout(pending.timer);
          if(value.error)pending.reject(new Error(JSON.stringify(value.error)));
          else pending.resolve(value.result);
        }catch(error){this.fail(error);}
      }
    });
  }
  fail(error) {
    this.fault=error;
    for(const pending of this.pending.values()){clearTimeout(pending.timer);pending.reject(error);}
    this.pending.clear();
  }
  notify(method,params) {this.child.stdin.write(JSON.stringify({jsonrpc:'2.0',method,params})+'\n');}
  request(method,params) {
    if(this.fault)return Promise.reject(this.fault);
    const id=this.nextId++;
    return new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{this.pending.delete(id);reject(new Error(`MCP ${method} timed out`));},10000);
      this.pending.set(id,{resolve,reject,timer});
      this.child.stdin.write(JSON.stringify({jsonrpc:'2.0',id,method,params})+'\n');
    });
  }
  async initialize() {
    const info=await this.request('initialize',{protocolVersion:'2025-11-25',capabilities:{},clientInfo:{name:'native-product-e2e',version:'1'}});
    assert.equal(info.protocolVersion,'2025-11-25');assert.equal(info.serverInfo.name,'agentic-internet');
    this.notify('notifications/initialized',{});
    const tools=await this.request('tools/list',{});
    assert.deepEqual(tools.tools.map(tool=>tool.name),['inbox.poll','inbox.ack','messages.send','delivery.get']);
  }
  tool(name,args) {return this.request('tools/call',{name,arguments:args});}
  async runtimeContext() {
    const catalog=await this.request('resources/list',{});
    assert.deepEqual(catalog.resources.map(resource=>resource.uri),['agentic://runtime']);
    const result=await this.request('resources/read',{uri:catalog.resources[0].uri});
    assert.equal(result.contents.length,1);assert.equal(result.contents[0].mimeType,'application/json');
    assert.ok(Buffer.byteLength(result.contents[0].text)<=16*1024);
    return JSON.parse(result.contents[0].text);
  }
  async success(name,args) {
    const result=await this.tool(name,args);assert.equal(result.isError,false,JSON.stringify(result));
    assert.ok(result.structuredContent);return result.structuredContent;
  }
  async stop(assertClean=false) {
    if(this.launchError){if(assertClean)throw this.launchError;return;}
    this.child.stdin.end();
    let deadline=Date.now()+3000;
    while(this.child.exitCode===null&&this.child.signalCode===null&&Date.now()<deadline)await delay(20);
    if(this.child.exitCode===null&&this.child.signalCode===null) {
      this.child.kill('SIGKILL');deadline=Date.now()+3000;
      while(this.child.exitCode===null&&this.child.signalCode===null&&Date.now()<deadline)await delay(20);
    }
    assert.ok(this.child.exitCode!==null||this.child.signalCode!==null,'MCP process did not reap');
    if(assertClean)assert.equal(this.child.exitCode,0,'MCP must exit successfully after stdin EOF');
  }
}
