import {fakeApi} from './fake-api';
import {render,screen,waitFor} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {describe,it,expect,vi} from 'vitest';
import {AgentPanel} from '../src/AgentPanel';
import type {DesktopApi,ProvisionRuntimeRequest,RuntimeInfo,RuntimeSetup} from '../src/types';

const conversations=[{id:'ab'.repeat(32),title:'Bob',unread:0,messages:[]},{id:'cd'.repeat(32),title:'Direct conversation',unread:0,messages:[]}];
const config={mcpServers:{'ain-test':{command:'/Applications/Kaiki Chat.app/Contents/MacOS/agentic-mcp',args:['--credentials',"/private/owner's profile/runtimes/grant.json"]}}};
const cliConfig={command:'/Applications/Kaiki Chat.app/Contents/MacOS/agentic-cli',args:['--credentials',"/private/owner's profile/runtimes/grant.json"]};
function publicRuntime(request:ProvisionRuntimeRequest):RuntimeInfo {
  return {grantId:Array(32).fill(19),name:request.name,principal:Array(32).fill(20),agentId:request.agentId,serviceId:request.serviceId,conversationIds:request.conversationIds,actions:request.actions,expiresAt:request.expiresAt,maxDataBytes:request.maxDataBytes,status:'active'};
}
function client() {
  let records:RuntimeInfo[]=[];
  return {
    ...fakeApi(),
    listRuntimes:vi.fn(async()=>structuredClone(records)),
    provisionRuntime:vi.fn(async(request:ProvisionRuntimeRequest):Promise<RuntimeSetup>=>{const runtime=publicRuntime(request);records=[runtime];return {runtime,credentialsPath:cliConfig.args[1],mcpConfig:config,cliConfig};}),
    revokeRuntime:vi.fn(async({grantId}:{grantId:number[]})=>{records=records.map(r=>JSON.stringify(r.grantId)===JSON.stringify(grantId)?{...r,status:'revoked'}:r);}),
  } satisfies DesktopApi;
}
async function selectForm(user:ReturnType<typeof userEvent.setup>) {
  await user.type(await screen.findByRole('textbox',{name:'Agent name'}),'Helper');
  expect(screen.getByRole('button',{name:'Grant access'})).toBeDisabled();
  await user.click(screen.getByRole('checkbox',{name:'Bob'}));
}
describe('owner runtime permissions panel',()=>{
  it('requires chosen dialogs, defaults to read only, and displays the exact returned config',async()=>{
    const api=client();const user=userEvent.setup();render(<AgentPanel api={api} conversations={conversations} onBack={()=>{}}/>);
    await selectForm(user);
    expect(screen.getByRole('checkbox',{name:'Allow sending messages'})).not.toBeChecked();
    expect(screen.getByRole('checkbox',{name:'Direct conversation'})).not.toBeChecked();
    const before=Math.floor(Date.now()/1000);
    await user.click(screen.getByRole('button',{name:'Grant access'}));
    const shown=await screen.findByRole('textbox',{name:'MCP configuration'});
    expect(shown).toHaveValue(JSON.stringify(config,null,2));expect(shown).toHaveAttribute('readonly');
    const cli=screen.getByRole('textbox',{name:'CLI command'});
    expect(cli).toHaveValue("'/Applications/Kaiki Chat.app/Contents/MacOS/agentic-cli' '--credentials' '/private/owner'\"'\"'s profile/runtimes/grant.json' 'context'");
    expect(cli).toHaveAttribute('readonly');
    const request=api.provisionRuntime.mock.calls[0][0];
    expect(request).toMatchObject({name:'Helper',conversationIds:[conversations[0].id],actions:['read_inbox'],maxDataBytes:4096});
    expect(request.operationId.length).toBeGreaterThan(0);
    expect(request.agentId).toHaveLength(32);expect(request.serviceId).toHaveLength(32);expect(request.agentId).not.toEqual(request.serviceId);
    expect(request.expiresAt).toBeGreaterThan(before);expect(request.expiresAt).toBeLessThanOrEqual(before+7*86400+2);
    for(const forbidden of ['principal','signingSeed','credentialsPath','command'])expect(request).not.toHaveProperty(forbidden);
    expect(await screen.findByText('Active')).toBeVisible();
  });
  it('keeps the complete approved request on unchanged retry and allocates a new operation after edits',async()=>{
    const api=client();const user=userEvent.setup();api.provisionRuntime.mockRejectedValueOnce(new Error('The core reply was lost'));
    render(<AgentPanel api={api} conversations={conversations} onBack={()=>{}}/>);await selectForm(user);
    await user.click(screen.getByRole('checkbox',{name:'Allow sending messages'}));
    await user.click(screen.getByRole('button',{name:'Grant access'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('The core reply was lost');
    expect(screen.queryByRole('textbox',{name:'MCP configuration'})).not.toBeInTheDocument();
    expect(api.listRuntimes.mock.calls.length).toBeGreaterThanOrEqual(2);
    const original=structuredClone(api.provisionRuntime.mock.calls[0][0]);
    api.provisionRuntime.mockRejectedValueOnce(new Error('Retry is not available yet'));
    await user.click(screen.getByRole('button',{name:'Grant access'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('Retry is not available yet');
    expect(api.provisionRuntime.mock.calls[1][0]).toEqual(original);
    const name=screen.getByRole('textbox',{name:'Agent name'});await user.clear(name);await user.type(name,'Another helper');
    await user.click(screen.getByRole('button',{name:'Grant access'}));
    await screen.findByRole('textbox',{name:'MCP configuration'});
    const edited=api.provisionRuntime.mock.calls[2][0];expect(edited.operationId).not.toEqual(original.operationId);
    expect(edited.name).toBe('Another helper');expect(edited.actions).toEqual(['read_inbox','send_message']);
  });
  it('revokes the exact listed grant and preserves active status when the daemon rejects revocation',async()=>{
    const api=client();const user=userEvent.setup();render(<AgentPanel api={api} conversations={conversations} onBack={()=>{}}/>);await selectForm(user);
    await user.click(screen.getByRole('button',{name:'Grant access'}));await screen.findByText('Active');
    api.revokeRuntime.mockRejectedValueOnce(new Error('No connection to the core'));
    await user.click(screen.getByRole('button',{name:'Revoke access Helper'}));
    expect(await screen.findByRole('alert')).toHaveTextContent('No connection to the core');expect(screen.getByText('Active')).toBeVisible();
    expect(screen.queryByText('Revoked')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button',{name:'Revoke access Helper'}));
    await waitFor(()=>expect(screen.getByText('Revoked')).toBeVisible());
    expect(api.revokeRuntime).toHaveBeenLastCalledWith({grantId:Array(32).fill(19)});
    expect(screen.queryByRole('button',{name:'Revoke access Helper'})).not.toBeInTheDocument();
    expect(screen.queryByRole('textbox',{name:'CLI command'})).not.toBeInTheDocument();
  });
});
