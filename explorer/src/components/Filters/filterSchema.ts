import { EnumFilterKey } from '../../typeDefs/filters';

export const generateFilterSchema = (upperSaturationValue?: number) => ({
  profitMargin: {
    label: 'Profit margin (%)',
    id: EnumFilterKey.profitMargin,
    value: [0, 10],
    marks: [
      { label: '0', value: 0 },
      { label: '10', value: 10 },
      { label: '20', value: 20 },
      { label: '30', value: 30 },
      { label: '40', value: 40 },
      { label: '50', value: 50 },
      { label: '60', value: 60 },
      { label: '70', value: 70 },
      { label: '80', value: 80 },
      { label: '90', value: 90 },
      { label: '100', value: 100 },
    ],
  },
  stakeSaturation: {
    label: 'Stake saturation (%)',
    id: EnumFilterKey.stakeSaturation,
    value: [0, 10],
    marks: [
      { label: '0', value: 0 },

      {
        label: '10',
        value: 10,
      },

      {
        label: '50',
        value: 50,
      },
      { label: '90', value: 90 },

      {
        label: upperSaturationValue ? `${upperSaturationValue}` : '100',
        value: upperSaturationValue || 100,
      },
    ],
    max: upperSaturationValue,
  },
  stake: {
    label: 'Stake',
    id: EnumFilterKey.stake,
    value: [20, 100],
    marks: [
      {
        value: 0,
        label: '1',
      },
      {
        value: 10,
        label: '10',
      },
      {
        value: 20,
        label: '100',
      },
      {
        value: 30,
        label: '1k',
      },
      {
        value: 40,
        label: '10K',
      },
      {
        value: 50,
        label: '100K',
      },
      {
        value: 60,
        label: '1M',
      },
      {
        value: 70,
        label: '10M',
      },
      {
        value: 80,
        label: '100M',
      },
      {
        value: 90,
        label: '1B',
      },
      {
        value: 100,
        label: '10B',
      },
    ],
    min: 20,
    scale: (value: number) => 10 ** (value / 10),
  },
});
