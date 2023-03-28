import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { DelegatorsInfoTable } from './Table';
import { EconomicsInfoColumns } from './Columns';
import { EconomicsInfoRowWithIndex } from './types';

export default {
  title: 'Mix Node Detail/Economics',
  component: DelegatorsInfoTable,
} as ComponentMeta<typeof DelegatorsInfoTable>;

const row: EconomicsInfoRowWithIndex = {
  id: 1,
  selectionChance: {
    value: 'High',
  },

  estimatedOperatorReward: {
    value: '80000.123456 NYM',
  },
  estimatedTotalReward: {
    value: '80000.123456 NYM',
  },
  profitMargin: {
    value: '10 %',
  },
  operatingCost: {
    value: '11121 NYM',
  },
  avgUptime: {
    value: '-',
  },
  nodePerformance: {
    value: '-',
  },
};

const rowGoodProbabilitySelection: EconomicsInfoRowWithIndex = {
  ...row,
  selectionChance: {
    value: 'Good',
  },
};

const rowLowProbabilitySelection: EconomicsInfoRowWithIndex = {
  ...row,
  selectionChance: {
    value: 'Low',
  },
};

const emptyRow: EconomicsInfoRowWithIndex = {
  id: 1,
  selectionChance: {
    value: '-',
    progressBarValue: 0,
  },

  estimatedOperatorReward: {
    value: '-',
  },
  estimatedTotalReward: {
    value: '-',
  },
  profitMargin: {
    value: '-',
  },
  operatingCost: {
    value: '-',
  },
  avgUptime: {
    value: '-',
  },
  nodePerformance: {
    value: '-',
  },
};

const Template: ComponentStory<typeof DelegatorsInfoTable> = (args) => <DelegatorsInfoTable {...args} />;

export const Empty = Template.bind({});
Empty.args = {
  rows: [emptyRow],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceHigh = Template.bind({});
selectionChanceHigh.args = {
  rows: [row],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceGood = Template.bind({});
selectionChanceGood.args = {
  rows: [rowGoodProbabilitySelection],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceLow = Template.bind({});
selectionChanceLow.args = {
  rows: [rowLowProbabilitySelection],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};
