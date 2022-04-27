import * as React from 'react';
import { ComponentMeta, ComponentStory } from '@storybook/react';
import { DelegatorsInfoTable } from './Table';
import { delegatorsInfoColumns } from './Columns';
import { DelegatorsInfoRowWithIndex } from './types';

export default {
  title: 'Mix Node Detail/Economics/Table',
  component: DelegatorsInfoTable,
} as ComponentMeta<typeof DelegatorsInfoTable>;

const row: DelegatorsInfoRowWithIndex = {
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

const emptyRow: DelegatorsInfoRowWithIndex = {
  id: 1,
  active_set_probability: {
    value: '0 %',
    progressBarValue: 0,
  },
  avg_uptime: {
    value: '0 %',
  },
  estimated_operator_reward: {
    value: '0 NYM',
  },
  estimated_total_reward: {
    value: '0 NYM',
  },
  profit_margin: {
    value: '0 %',
  },
  stake_saturation: {
    value: '0 %',
    progressBarValue: 0,
  },
};

const Template: ComponentStory<typeof DelegatorsInfoTable> = (args) => <DelegatorsInfoTable {...args} />;

export const Empty = Template.bind({});
Empty.args = {
  rows: [emptyRow],
  columnsData: delegatorsInfoColumns,
  tableName: 'storybook',
};

export const Default = Template.bind({});
Default.args = {
  rows: [row],
  columnsData: delegatorsInfoColumns,
  tableName: 'storybook',
};
