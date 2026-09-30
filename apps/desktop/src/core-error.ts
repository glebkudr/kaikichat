/** A refusal from the daemon or the native side: `{code, message, retryable}`.
 * The screens show it in the current language by its code. */
export class CoreError extends Error {
  constructor(readonly code:string,message:string,readonly retryable:boolean){super(message);this.name='CoreError';}
}
/** A native answer turned into an error the screens can show and branch on. */
export function coreError(value:unknown):Error {
  if(value instanceof Error)return value;
  if(value&&typeof value==='object'&&typeof (value as {code?:unknown}).code==='string') {
    const {code,message,retryable}=value as {code:string;message?:unknown;retryable?:unknown};
    return new CoreError(code,typeof message==='string'&&message?message:code,retryable===true);
  }
  return new Error(typeof value==='string'?value:'unknown core error');
}
export const isCode=(error:unknown,code:string)=>error instanceof CoreError&&error.code===code;
