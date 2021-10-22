/* eslint-disable camelcase */
import { MutableRefObject } from 'react';
import { GatewayResponse, MixNodeResponse } from 'src/typeDefs/explorer-api';

export function formatNumber(num: number): string {
  return new Intl.NumberFormat().format(num);
}

export function scrollToRef(
  ref: MutableRefObject<HTMLDivElement | undefined>,
): void {
  if (ref?.current) ref.current.scrollIntoView();
}

export type MixnodeRowType = {
  id: string;
  owner: string;
  location: string;
  identity_key: string;
  bond: number;
  host: string;
  layer: string;
};

export type GatewayRowType = {
  id: string;
  owner: string;
  identity_key: string;
  bond: number;
  host: string;
  location: string;
};

export function mixnodeToGridRow(
  arrayOfMixnodes: MixNodeResponse,
): MixnodeRowType[] {
  return !arrayOfMixnodes
    ? []
    : arrayOfMixnodes.map((mn) => ({
        id: mn.owner,
        owner: mn.owner,
        location: mn?.location?.country_name || '',
        identity_key: mn.mix_node.identity_key || '',
        bond: mn.bond_amount.amount || 0,
        host: mn.mix_node.host || '',
        layer: mn.layer || '',
      }));
}

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
        bond: gw.bond_amount.amount || 0,
        host: gw.gateway.host || '',
      }));
}
