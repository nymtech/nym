import { EnumFilterKey, TFilters } from '../../typeDefs/filters';

export const generateFilterSchema = () => ({
  profitMargin: {
    label: 'Profit margin (%)',
    id: EnumFilterKey.profitMargin,
    value: [0, 100],
    isSmooth: true,
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
    value: [0, 100],
    isSmooth: true,
    marks: [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100].map((value) => ({
      value: value < 100 ? value : 100,
      label: value < 100 ? value : '>100',
    })),
    tooltipInfo: "Select nodes with <100% saturation. Any additional stake above 100% saturation won't get rewards",
  },
  routingScore: {
    label: 'Routing score (%)',
    id: EnumFilterKey.routingScore,
    value: [0, 100],
    isSmooth: true,
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
    tooltipInfo: 'The higher the routing score the better the performance of the node and so its rewards',
  },
});

const formatStakeSaturationValues = ([value_1, value_2]: number[]) => {
  const lowerValue = value_1 / 100;
  const upperValue = value_2 / 100;

  return [lowerValue, upperValue];
};

export const formatOnSave = (filters: TFilters) => ({
  routingScore: filters.routingScore.value,
  profitMargin: filters.profitMargin.value,
  stakeSaturation: formatStakeSaturationValues(filters.stakeSaturation.value),
});
