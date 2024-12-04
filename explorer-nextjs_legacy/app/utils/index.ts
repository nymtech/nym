/* eslint-disable camelcase */
import { MutableRefObject } from 'react';
import { Theme } from '@mui/material/styles';
import { registerLocale, getName } from 'i18n-iso-countries';
import Big from 'big.js';
import { CountryData } from '@/app/typeDefs/explorer-api';
import { EconomicsRowsType } from '@/app/components/MixNodes/Economics/types';
import { Network } from '../typeDefs/network';

registerLocale(require('i18n-iso-countries/langs/en.json'));

export function formatNumber(num: number): string {
  return new Intl.NumberFormat().format(num);
}

export function scrollToRef(ref: MutableRefObject<HTMLDivElement | undefined>): void {
  if (ref?.current) ref.current.scrollIntoView();
}

export type CountryDataRowType = {
  id: number;
  ISO3: string;
  nodes: number;
  countryName: string;
  percentage: string;
};

export function countryDataToGridRow(countriesData: CountryData[]): CountryDataRowType[] {
  const totalNodes = countriesData.reduce((acc, obj) => acc + obj.nodes, 0);
  const formatted = countriesData.map((each: CountryData, index: number) => {
    const updatedCountryRecord: CountryDataRowType = {
      ...each,
      id: index,
      countryName: getName(each.ISO3, 'en', { select: 'alias' }),
      percentage: ((each.nodes * 100) / totalNodes).toFixed(1),
    };
    return updatedCountryRecord;
  });

  const sorted = formatted.sort((a, b) => (a.nodes < b.nodes ? 1 : -1));
  return sorted;
}

export const splice = (start: number, deleteCount: number, address?: string): string => {
  if (address) {
    const array = address.split('');
    array.splice(start, deleteCount, '...');
    return array.join('');
  }
  return '';
};

export const trimAddress = (address = '', trimBy = 6) => `${address.slice(0, trimBy)}...${address.slice(-trimBy)}`;

/**
 * Converts a stringified percentage float (0.0-1.0) to a stringified integer (0-100).
 *
 * @param value - the percentage to convert
 * @returns A stringified integer
 */
export const toPercentIntegerString = (value: string) => Math.round(Number(value) * 100).toString();
export const toPercentInteger = (value: string) => Math.round(Number(value) * 100);

export const textColour = (value: EconomicsRowsType, field: string, theme: Theme) => {
  const progressBarValue = value?.progressBarValue || 0;
  const fieldValue = value.value;

  if (progressBarValue > 100) {
    return theme.palette.warning.main;
  }
  if (field === 'selectionChance') {
    // TODO: when v2 will be deployed, remove cases: VeryHigh, Moderate and VeryLow
    switch (fieldValue) {
      case 'High':
      case 'VeryHigh':
        return theme.palette.nym.networkExplorer.selectionChance.overModerate;
      case 'Good':
      case 'Moderate':
        return theme.palette.nym.networkExplorer.selectionChance.moderate;
      case 'Low':
      case 'VeryLow':
        return theme.palette.nym.networkExplorer.selectionChance.underModerate;
      default:
        return theme.palette.nym.wallet.fee;
    }
  }
  return theme.palette.nym.wallet.fee;
};

export const isGreaterThan = (a: number, b: number) => a > b;

export const isLessThan = (a: number, b: number) => a < b;

/**
 *
 * Checks if the user's balance is enough to pay the fee
 * @param balance - The user's current balance
 * @param fee - The fee for the tx
 * @param tx - The amount of the tx
 * @returns boolean
 *
 */

export const isBalanceEnough = (fee: string, tx: string = '0', balance: string = '0') => {
  console.log('balance', balance, fee, tx);
  try {
    return Big(balance).gte(Big(fee).plus(Big(tx)));
  } catch (e) {
    console.log(e);
    return false;
  }
};

export const urls = (networkName?: Network) =>
  networkName === 'MAINNET'
    ? {
      mixnetExplorer: 'https://mixnet.explorers.guru/',
      blockExplorer: 'https://blocks.nymtech.net',
      networkExplorer: 'https://explorer.nymtech.net',
    }
    : {
      blockExplorer: `https://${networkName}-blocks.nymtech.net`,
      networkExplorer: `https://${networkName}-explorer.nymtech.net`,
    };
