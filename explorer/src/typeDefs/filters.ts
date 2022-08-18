// eslint-disable-next-line import/no-extraneous-dependencies
import { Mark } from '@mui/base';

export enum EnumFilterKey {
  profitMargin = 'profitMargin',
  stakeSaturation = 'stakeSaturation',
  routingScore = 'routingScore',
}

export type TFilterItem = {
  label: string;
  id: EnumFilterKey;
  value: number[];
  isSmooth?: boolean;
  marks: Mark[];
  min?: number;
  max?: number;
  scale?: (value: number) => number;
  tooltipInfo?: string;
};

export type TFilters = { [key in EnumFilterKey]: TFilterItem };
