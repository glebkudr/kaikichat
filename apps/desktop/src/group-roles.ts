import type {BannedId,Group,GroupRole} from './types';

/** What this profile may do in a group (spec/groups-v1.md): the owner and
 * admins add, remove and ban, an admin removes or bans neither the owner nor
 * another admin, only the owner names admins or lifts the owner's bans, and
 * nobody removes the owner. Only a channel's owner adds to its team. */
export function groupRights(group:Group) {
  const manager=group.role==='owner'||group.role==='admin';
  const canRemove=(member:string)=>{
    if(!manager||member===group.owner)return false;
    if(group.role==='admin'&&group.admins.includes(member))return false;
    return true;
  };
  return {
    canAdd:manager&&(group.kind!=='channel'||group.role==='owner'),
    canBanById:manager,
    canSetAdmins:group.role==='owner',
    canRemove,
    canBan:canRemove,
    canUnban:(ban:BannedId)=>group.role==='owner'||(group.role==='admin'&&!ban.byOwner),
  };
}
export function roleOf(group:Group,member:string):GroupRole {
  return member===group.owner?'owner':group.admins.includes(member)?'admin':'member';
}
/** A network id short enough for a list; the whole id stays in the title. */
export const shortId=(id:string)=>id.length>18?`${id.slice(0,10)}…${id.slice(-6)}`:id;
/** A network id as the node writes it: `ain1` and 64 hex digits. */
export const validNetworkId=(id:string)=>/^ain1[0-9a-f]{64}$/.test(id);
