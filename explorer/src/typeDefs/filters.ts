import { Mark } from '@mui/base';

export enum EnumFilterKey {
  profitMargin = 'profitMargin',
  stakeSaturation = 'stakeSaturation',
  stake = 'stake',
}

export type TFilterItem = {
  label: string;
  id: EnumFilterKey;
  value: number[];
  marks: Mark[];
  min?: number;
  max?: number;
  scale?: (value: number) => number;
};

export type TFilters = { [key in EnumFilterKey]: TFilterItem };
