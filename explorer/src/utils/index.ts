/* eslint-disable camelcase */
import { MutableRefObject } from 'react';
import { Theme } from '@mui/material/styles';
import { registerLocale, getName } from 'i18n-iso-countries';
import { CountryData } from '../typeDefs/explorer-api';
import { EconomicsRowsType } from '../components/MixNodes/Economics/types';

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

/**
 * Converts a stringified percentage float (0.0-1.0) to a stringified integer (0-100).
 *
 * @param value - the percentage to convert
 * @returns A stringified integer
 */
export const toPercentIntegerString = (value: string) => Math.round(Number(value) * 100).toString();

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
