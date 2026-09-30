import type {Delivery} from './types';
import {useT} from './i18n';
export function DeliveryBadge({delivery}:{delivery:Delivery}) {
  const t=useT();
  let label:string;
  switch(delivery.phase) {
    case 'queued':label=t.delivery.queued;break;
    case 'stored':label=delivery.replicas>=delivery.target?t.delivery.stored(delivery.replicas,delivery.target):t.delivery.repairing(delivery.replicas,delivery.target);break;
    case 'delivered':label=t.delivery.delivered;break;
    case 'read':label=t.delivery.read;break;
    case 'failed':label=t.delivery.failed;break;
  }
  return <span className={`delivery delivery-${delivery.phase}`}>{label}</span>;
}
