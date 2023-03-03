/* eslint-disable camelcase */
import { MixNodeResponse, MixNodeResponseItem, MixnodeStatus, NodePerformance } from '../../typeDefs/explorer-api';
import { toPercentIntegerString } from '../../utils';
import { unymToNym } from '../../utils/currency';

export type MixnodeRowType = {
  mix_id: number;
  id: string;
  status: MixnodeStatus;
  owner: string;
  location: string;
  identity_key: string;
  bond: number;
  self_percentage: string;
  pledge_amount: number;
  host: string;
  layer: string;
  profit_percentage: string;
  avg_uptime: string;
  stake_saturation: React.ReactNode;
  operating_cost: string;
  node_performance: NodePerformance['most_recent'];
};

export function mixnodeToGridRow(arrayOfMixnodes?: MixNodeResponse): MixnodeRowType[] {
  return (arrayOfMixnodes || []).map(mixNodeResponseItemToMixnodeRowType);
}

export function mixNodeResponseItemToMixnodeRowType(item: MixNodeResponseItem): MixnodeRowType {
  const pledge = Number(item.pledge_amount.amount) || 0;
  const delegations = Number(item.total_delegation.amount) || 0;
  const totalBond = pledge + delegations;
  const selfPercentage = ((pledge * 100) / totalBond).toFixed(2);
  const profitPercentage = toPercentIntegerString(item.profit_margin_percent) || 0;
  const uncappedSaturation = typeof item.uncapped_saturation === 'number' ? item.uncapped_saturation * 100 : 0;

  return {
    mix_id: item.mix_id,
    id: item.owner,
    status: item.status,
    owner: item.owner,
    identity_key: item.mix_node.identity_key || '',
    bond: totalBond || 0,
    location: item?.location?.country_name || '',
    self_percentage: selfPercentage,
    pledge_amount: pledge,
    host: item?.mix_node?.host || '',
    layer: item?.layer || '',
    profit_percentage: `${profitPercentage}%`,
    avg_uptime: `${toPercentIntegerString(item.node_performance.last_24h)}%`,
    stake_saturation: uncappedSaturation.toFixed(2),
    operating_cost: `${unymToNym(item.operating_cost?.amount, 6)} NYM`,
    node_performance: `${toPercentIntegerString(item.node_performance.most_recent)}%`,
  };
}
