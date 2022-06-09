import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { Paper } from '@mui/material';
import { RewardsSummary } from './RewardsSummary';

export default {
  title: 'Rewards/Components/Rewards Summary',
  component: RewardsSummary,
} as ComponentMeta<typeof RewardsSummary>;

export const Default = () => (
  <Paper elevation={0} sx={{ px: 4, py: 2 }}>
    <RewardsSummary totalDelegation="860.123 NYM" totalRewards="4.86723 NYM" />
  </Paper>
);

export const Empty = () => (
  <Paper elevation={0} sx={{ px: 4, py: 2 }}>
    <RewardsSummary />
  </Paper>
);

export const Loading = () => (
  <Paper elevation={0} sx={{ px: 4, py: 2 }}>
    <RewardsSummary isLoading />
  </Paper>
);
