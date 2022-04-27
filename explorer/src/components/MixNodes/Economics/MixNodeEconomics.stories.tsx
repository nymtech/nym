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
  active_set_probability: {
    value: '50 %',
    progressBarValue: 50,
  },
  avg_uptime: {
    value: '65 %',
  },
  estimated_operator_reward: {
    value: '80000.123456 NYM',
  },
  estimated_total_reward: {
    value: '80000.123456 NYM',
  },
  profit_margin: {
    value: '10 %',
  },
  stake_saturation: {
    value: '120 %',
    progressBarValue: 120,
  },
};

const emptyRow: EconomicsInfoRowWithIndex = {
  id: 1,
  active_set_probability: {
    value: '-',
    progressBarValue: 0,
  },
  avg_uptime: {
    value: '-',
  },
  estimated_operator_reward: {
    value: '-',
  },
  estimated_total_reward: {
    value: '-',
  },
  profit_margin: {
    value: '-',
  },
  stake_saturation: {
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

export const Default = Template.bind({});
Default.args = {
  rows: [row],
  columnsData: EconomicsInfoColumns,
  tableName: 'storybook',
};
