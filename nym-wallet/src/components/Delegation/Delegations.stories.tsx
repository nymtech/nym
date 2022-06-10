import React from 'react';
import { ComponentMeta } from '@storybook/react';

import { Paper } from '@mui/material';
import { Delegations } from './Delegations';
import { items } from './DelegationList.stories';

const explorerUrl = 'https://sandbox-explorer.nymtech.net';

export default {
  title: 'Delegation/Components/Delegations',
  component: Delegations,
} as ComponentMeta<typeof Delegations>;

export const Default = () => (
  <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
    <h2>Your Delegations</h2>
    <Delegations items={items} explorerUrl={explorerUrl} />
  </Paper>
);

export const Empty = () => (
  <Paper elevation={0} sx={{ px: 4, pt: 2, pb: 4 }}>
    <h2>Your Delegations</h2>
    <Delegations items={[]} explorerUrl={explorerUrl} />
  </Paper>
);
