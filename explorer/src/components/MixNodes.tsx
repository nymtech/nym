import * as React from 'react';
import { MixNodeResponse, MixnodeStatus } from '../typeDefs/explorer-api';

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
};

export function mixnodeToGridRow(
  arrayOfMixnodes: MixNodeResponse,
): MixnodeRowType[] {
  return !arrayOfMixnodes
    ? []
    : arrayOfMixnodes.map((mn) => {
        const pledge = Number(mn.pledge_amount.amount) || 0;
        const delegations = Number(mn.total_delegation.amount) || 0;
        const totalBond = pledge + delegations;
        const selfPercentage = ((pledge * 100) / totalBond).toFixed(2);
        return {
          id: mn.owner,
          status: mn.status,
          owner: mn.owner,
          identity_key: mn.mix_node.identity_key || '',
          bond: totalBond || 0,
          location: mn?.location?.country_name || '',
          self_percentage: selfPercentage,
          host: mn?.mix_node?.host || '',
          layer: mn?.layer || '',
        };
      });
}
