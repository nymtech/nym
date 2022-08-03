import { EnumFilterKey, TFilters } from '../../typeDefs/filters';

export const generateFilterSchema = (upperSaturationValue?: number) => ({
  profitMargin: {
    label: 'Profit margin (%)',
    id: EnumFilterKey.profitMargin,
    value: [0, 100],
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
    tooltipInfo:
      'As a delegator you want to chose nodes with lower profit margin, meaning more payout for their delegators',
  },
  stakeSaturation: {
    label: 'Stake saturation (%)',
    id: EnumFilterKey.stakeSaturation,
    value: [0, upperSaturationValue || 100],
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
    tooltipInfo: "Select nodes with <100% saturation. Any additional stake above 100% saturation won't get rewards",
  },
  stake: {
    label: 'Routing Score (%)',
    id: EnumFilterKey.stake,
    value: [20, 90],
    min: 20,
    max: 90,
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
        label: '10k',
      },
      {
        value: 50,
        label: '100k',
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
    ],
    tooltipInfo: 'The higher the routing score the better the performance of the node and so its rewards',
  },
});

const formatStakeValuesToMinorDenom = ([value_1, value_2]: number[]) => {
  const lowerValue = 10 ** (value_1 / 10) * 1_000_000;
  const upperValue = 10 ** (value_2 / 10) * 1_000_000;

  return [lowerValue, upperValue];
};

const formatStakeSaturationValues = ([value_1, value_2]: number[]) => {
  const lowerValue = value_1 / 100;
  const upperValue = value_2 / 100;

  return [lowerValue, upperValue];
};

export const formatOnSave = (filters: TFilters) => ({
  stake: formatStakeValuesToMinorDenom(filters.stake.value),
  profitMargin: filters.profitMargin.value,
  stakeSaturation: formatStakeSaturationValues(filters.stakeSaturation.value),
});
