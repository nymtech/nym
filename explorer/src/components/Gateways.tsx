import * as React from 'react';
import { GatewayResponse } from '../typeDefs/explorer-api';

export type GatewayRowType = {
  id: string;
  owner: string;
  identity_key: string;
  bond: number;
  host: string;
  location: string;
};

export function gatewayToGridRow(
  arrayOfGateways: GatewayResponse,
): GatewayRowType[] {
  return !arrayOfGateways
    ? []
    : arrayOfGateways.map((gw) => ({
        id: gw.owner,
        owner: gw.owner,
        identity_key: gw.gateway.identity_key || '',
        location: gw?.gateway?.location || '',
        bond: gw.pledge_amount.amount || 0,
        host: gw.gateway.host || '',
      }));
}
