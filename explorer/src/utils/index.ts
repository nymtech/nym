/* eslint-disable camelcase */
import { MutableRefObject } from 'react';
import {
  CountryData,
  GatewayResponse,
  MixNodeResponse,
} from 'src/typeDefs/explorer-api';
import { registerLocale, getName } from 'i18n-iso-countries';

registerLocale(require('i18n-iso-countries/langs/en.json'));

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

export type CountryDataRowType = {
  id: number;
  ISO3: string;
  nodes: number;
  countryName: string;
  percentage: string;
};

export function countryDataToGridRow(
  countriesData: CountryData[],
): CountryDataRowType[] {
  const totalNodes = countriesData.reduce((acc, obj) => acc + obj.nodes, 0);
  const formatted = countriesData.map((each: CountryData, index: number) => {
    const updatedCountryRecord: CountryDataRowType = {
      ...each,
      id: index,
      countryName: getName(each.ISO3, 'en', { select: 'official' }),
      percentage: ((each.nodes * 100) / totalNodes).toFixed(1),
    };
    return updatedCountryRecord;
  });

  const sorted = formatted.sort((a, b) => (a.nodes < b.nodes ? 1 : -1));
  return sorted;
}

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
        bond:
          Number(mn.bond_amount.amount) + Number(mn.total_delegation.amount) ||
          0,
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
