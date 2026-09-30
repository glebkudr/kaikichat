import {useT} from './i18n';
export function LocalTime({value}:{value:number}) {
  const t=useT();const date=new Date(value*1000);
  return <time dateTime={date.toISOString()}>{date.toLocaleString(t.tag,{day:'2-digit',month:'2-digit',year:'numeric',hour:'2-digit',minute:'2-digit'})}</time>;
}
