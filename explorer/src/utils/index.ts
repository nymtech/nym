/* eslint-disable camelcase */
import { MutableRefObject } from 'react';
import { CountryData } from 'src/typeDefs/explorer-api';
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
      countryName: getName(each.ISO3, 'en', { select: 'alias' }),
      percentage: ((each.nodes * 100) / totalNodes).toFixed(1),
    };
    return updatedCountryRecord;
  });

  const sorted = formatted.sort((a, b) => (a.nodes < b.nodes ? 1 : -1));
  return sorted;
}
