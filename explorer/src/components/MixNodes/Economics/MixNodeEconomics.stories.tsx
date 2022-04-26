import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { DelegatorsInfoTable } from './Table';
import { delegatorsInfoColumns } from './Columns';
import { DelegatorsInfoRowWithIndex } from './types';

export default {
  title: 'Mix Node Detail/Economics',
  component: DelegatorsInfoTable,
} as ComponentMeta<typeof DelegatorsInfoTable>;

export const Default = () => {
  const row: DelegatorsInfoRowWithIndex = {
    id: 1,
    active_set_probability: {
      value: '50 %',
      percentaje: 50,
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
      percentaje: 120,
    },
  };
  return <DelegatorsInfoTable columnsData={delegatorsInfoColumns} tableName="storybook" rows={[row]} />;
};
