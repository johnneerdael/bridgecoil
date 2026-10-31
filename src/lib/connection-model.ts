export type PairRecord={host:string;guid:string;pairedAt:number};
export const pairStoreKey="bridgecoil.pairings.v1";
export function endpointHost(endpoint:string):string|null{
 const value=endpoint.trim();if(value.startsWith("[")){const end=value.indexOf("]");return end>1?value.slice(1,end).toLowerCase():null;}
 const at=value.lastIndexOf(":");return at>0?value.slice(0,at).toLowerCase():null;
}
export function loadPairs(raw:string|null):PairRecord[]{
 try{const data:unknown=JSON.parse(raw??"[]");if(!Array.isArray(data))return [];return data.filter((r):r is PairRecord=>!!r&&typeof r==="object"&&typeof r.host==="string"&&r.host.length<=255&&typeof r.guid==="string"&&r.guid.length<=128&&typeof r.pairedAt==="number"&&Number.isFinite(r.pairedAt)&&r.pairedAt>=0&&r.pairedAt<=8640000000000000).slice(0,32);}catch{return [];}
}
export function recordPair(records:PairRecord[],endpoint:string,guid:string,now=Date.now()):PairRecord[]{
 const host=endpointHost(endpoint);if(!host)return records;return [{host,guid:guid.slice(0,128),pairedAt:now},...records.filter(r=>r.host!==host)].slice(0,32);
}
export type ConnectionProgress={attemptId:string;stage:"preparing"|"connecting"|"negotiating"|"waitingForApproval"|"inspecting"|"pairing"};
export const stageLabel:Record<ConnectionProgress["stage"],string>={preparing:"Preparing this app's key",connecting:"Connecting to TV",negotiating:"Checking ADB authorization",waitingForApproval:"Approve debugging on the TV",inspecting:"Reading device information",pairing:"Pairing with TV"};
