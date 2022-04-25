/* eslint-disable camelcase */
import { MixNodeResponse, MixNodeResponseItem, MixnodeStatus } from '../../typeDefs/explorer-api';

export type MixnodeRowType = {
  id: string;
  status: MixnodeStatus;
  owner: string;
  location: string;
  identity_key: string;
  bond: number;
  self_percentage: string;
  host: string;
  layer: string;
  profit_percentage: string;
  avg_uptime?: number;  //Todo the question mark, when the back will be connected
};

export function mixnodeToGridRow(arrayOfMixnodes?: MixNodeResponse): MixnodeRowType[] {
  return (arrayOfMixnodes || []).map(mixNodeResponseItemToMixnodeRowType);
}

export function mixNodeResponseItemToMixnodeRowType(item: MixNodeResponseItem): MixnodeRowType {
  const pledge = Number(item.pledge_amount.amount) || 0;
  const delegations = Number(item.total_delegation.amount) || 0;
  const totalBond = pledge + delegations;
  const selfPercentage = ((pledge * 100) / totalBond).toFixed(2);
  const profit_percentage = item.mix_node.profit_margin_percent || 0;
  return {
    id: item.owner,
    status: item.status,
    owner: item.owner,
    identity_key: item.mix_node.identity_key || '',
    bond: totalBond || 0,
    location: item?.location?.country_name || '',
    self_percentage: selfPercentage,
    host: item?.mix_node?.host || '',
    layer: item?.layer || '',
    profit_percentage: `${profit_percentage}%`,
    avg_uptime: undefined
  };
}
