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
  avgUptime: {
    value: '65 %',
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
  stakeSaturation: {
    value: '80 %',
    progressBarValue: 80,
  },
};

const rowVeryHighProbabilitySelection: EconomicsInfoRowWithIndex = {
  ...row,
  selectionChance: {
    value: 'Very High',
  },
};

const rowModerateProbabilitySelection: EconomicsInfoRowWithIndex = {
  ...row,
  selectionChance: {
    value: 'Moderate',
  },
};

const rowLowProbabilitySelection: EconomicsInfoRowWithIndex = {
  ...row,
  selectionChance: {
    value: 'Low',
  },
};

const rowVeryLowProbabilitySelection: EconomicsInfoRowWithIndex = {
  ...row,
  selectionChance: {
    value: 'Very Low',
  },
};

const emptyRow: EconomicsInfoRowWithIndex = {
  id: 1,
  selectionChance: {
    value: '-',
    progressBarValue: 0,
  },
  avgUptime: {
    value: '-',
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
  stakeSaturation: {
    value: '-',
    progressBarValue: 0,
  },
};

const Template: ComponentStory<typeof DelegatorsInfoTable> = (args) => <DelegatorsInfoTable {...args} />;

export const Empty = Template.bind({});
Empty.args = {
  rows: [emptyRow],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceVeryHigh = Template.bind({});
selectionChanceVeryHigh.args = {
  rows: [rowVeryHighProbabilitySelection],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceHigh = Template.bind({});
selectionChanceHigh.args = {
  rows: [row],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceModerate = Template.bind({});
selectionChanceModerate.args = {
  rows: [rowModerateProbabilitySelection],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceLow = Template.bind({});
selectionChanceLow.args = {
  rows: [rowLowProbabilitySelection],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};

export const selectionChanceVeryLow = Template.bind({});
selectionChanceVeryLow.args = {
  rows: [rowVeryLowProbabilitySelection],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};
